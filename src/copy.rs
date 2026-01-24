use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::args::CopyOptions;
use crate::stats::Statistics;
use crate::utils::{Logger, matches_pattern, secure_remove_dir_all, securely_delete_file};

pub fn copy_directory(
    src_dir: &Path,
    dst_dir: &Path,
    options: &CopyOptions,
    logger: &Logger,
    stats: &Statistics,
) -> io::Result<()> {
    // Ensure the destination directory exists
    if !dst_dir.exists() {
        if !options.list_only {
            logger.log(&format!("Creating directory: {}", dst_dir.display()));
            fs::create_dir_all(dst_dir)?;
            stats.add_dir_created();
        } else {
            logger.log(&format!("Would create directory: {}", dst_dir.display()));
            stats.add_dir_created();
        }
    }

    // Collect the source files and directories
    // We collect them into a Vec to enable parallel iteration
    let entries: Vec<_> = fs::read_dir(src_dir)?.collect::<Result<Vec<_>, io::Error>>()?;

    // We need to keep track of source filenames for the purge step
    // Since we are inside a function that might be running in parallel (if recursive),
    // we need to be careful. But here we are just reading the current directory.
    let src_names: HashSet<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // Process entries in parallel if threads > 1, otherwise sequential
    // We use try_for_each to stop on error (matching original behavior)
    let process_entry = |entry: &fs::DirEntry| -> io::Result<()> {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();

        if path.is_file() {
            // Check if file matches any pattern
            let matches = options
                .patterns
                .iter()
                .any(|p| matches_pattern(&file_name, p));

            if matches {
                let dst_path = dst_dir.join(&file_name);
                copy_file(&path, &dst_path, options, logger, stats)?;
            }
        } else if path.is_dir() && options.recursive {
            let dst_subdir = dst_dir.join(&file_name);

            // Skip empty directories if not including them
            if !options.include_empty {
                // Check if directory is empty
                // Note: This check is slightly expensive as it opens the dir
                let is_empty = path.read_dir()?.next().is_none();
                if is_empty {
                    if options.log_file_names {
                        logger.log(&format!("Skipping empty directory: {}", path.display()));
                    }
                    stats.add_dir_skipped();
                    return Ok(());
                }
            }

            copy_directory(&path, &dst_subdir, options, logger, stats)?;

            // Move (delete source dir) if requested
            if options.move_dirs && !options.list_only {
                let is_empty = path.read_dir()?.next().is_none();
                if is_empty {
                    let _ = fs::remove_dir(&path);
                }
            }
        }
        Ok(())
    };

    if options.threads > 1 {
        entries.par_iter().try_for_each(process_entry)?;
    } else {
        entries.iter().try_for_each(process_entry)?;
    }

    // Purge files/directories in destination that don't exist in source
    if (options.purge || options.mirror) && !options.list_only {
        if let Ok(dst_entries) = fs::read_dir(dst_dir) {
            let dst_entries: Vec<_> = dst_entries.collect::<Result<Vec<_>, io::Error>>()?;

            let process_purge = |entry: &fs::DirEntry| -> io::Result<()> {
                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                if !src_names.contains(&file_name) {
                    if path.is_file() {
                        if options.shred_files {
                            logger.log(&format!("Securely removing file: {}", path.display()));
                            securely_delete_file(&path, logger)?;
                        } else {
                            logger.log(&format!("Removing file: {}", path.display()));
                            fs::remove_file(&path)?;
                        }
                        stats.add_file_removed();
                    } else if path.is_dir() {
                        if options.shred_files {
                            logger.log(&format!("Securely removing directory: {}", path.display()));
                            secure_remove_dir_all(&path, logger)?;
                        } else {
                            logger.log(&format!("Removing directory: {}", path.display()));
                            fs::remove_dir_all(&path)?;
                        }
                        stats.add_dir_removed();
                    }
                }
                Ok(())
            };

            if options.threads > 1 {
                dst_entries.par_iter().try_for_each(process_purge)?;
            } else {
                dst_entries.iter().try_for_each(process_purge)?;
            }
        }
    }

    Ok(())
}

fn should_copy_file(src_meta: &Metadata, dst_meta: Option<&Metadata>) -> bool {
    if dst_meta.is_none() {
        return true;
    }

    let dst_meta = dst_meta.unwrap();
    let src_modified = src_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let dst_modified = dst_meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);

    if src_modified > dst_modified {
        return true;
    }

    if src_modified == dst_modified && src_meta.len() != dst_meta.len() {
        return true;
    }

    false
}

fn copy_file(
    src_path: &Path,
    dst_path: &Path,
    options: &CopyOptions,
    logger: &Logger,
    stats: &Statistics,
) -> io::Result<()> {
    let src_meta = fs::metadata(src_path)?;
    let dst_meta = fs::metadata(dst_path).ok();

    if !should_copy_file(&src_meta, dst_meta.as_ref()) {
        if options.log_file_names {
            // logger.log(&format!("Skipping identical file: {}", dst_path.display()));
            // Robocopy doesn't usually log skipped files unless verbose
        }
        stats.add_file_skipped();
        return Ok(());
    }

    if options.list_only {
        logger.log(&format!(
            "Would copy file: {} -> {}",
            src_path.display(),
            dst_path.display()
        ));
        stats.add_file_copied(src_meta.len()); // Count as copied for stats in list mode? Original did.
        return Ok(());
    }

    if options.log_file_names {
        logger.log(&format!(
            "Copying file: {} -> {}",
            src_path.display(),
            dst_path.display()
        ));
    }

    let mut retry_count = 0;
    loop {
        match copy_file_content(src_path, dst_path, src_meta.len(), options) {
            Ok(_) => {
                // Preserve timestamps
                if let Ok(src_time) = src_meta.modified() {
                    let _ = filetime::set_file_mtime(
                        dst_path,
                        filetime::FileTime::from_system_time(src_time),
                    );
                }

                // Handle attributes (Windows only)
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if !options.attributes_add.is_empty() || !options.attributes_remove.is_empty() {
                        if let Ok(metadata) = fs::metadata(dst_path) {
                            let mut attributes = metadata.file_attributes();

                            // Add attributes
                            for c in options.attributes_add.chars() {
                                match c {
                                    'R' => attributes |= 0x00000001,
                                    'A' => attributes |= 0x00000020,
                                    'S' => attributes |= 0x00000004,
                                    'H' => attributes |= 0x00000002,
                                    'C' => attributes |= 0x00000800,
                                    'N' => attributes |= 0x00000080,
                                    _ => {}
                                }
                            }

                            // Remove attributes
                            for c in options.attributes_remove.chars() {
                                match c {
                                    'R' => attributes &= !0x00000001,
                                    'A' => attributes &= !0x00000020,
                                    'S' => attributes &= !0x00000004,
                                    'H' => attributes &= !0x00000002,
                                    'C' => attributes &= !0x00000800,
                                    'N' => attributes &= !0x00000080,
                                    _ => {}
                                }
                            }

                            // Apply using attrib command (simplest way to ensure it works)
                            let _ = std::process::Command::new("attrib")
                                .arg(format!("+{}", attributes))
                                .arg(dst_path.to_string_lossy().to_string())
                                .output();
                        }
                    }
                }

                // Move/Delete source
                if options.move_files {
                    if options.shred_files {
                        securely_delete_file(src_path, logger)?;
                    } else {
                        let _ = fs::remove_file(src_path);
                    }
                }

                stats.add_file_copied(src_meta.len());
                break;
            }
            Err(e) => {
                retry_count += 1;
                if retry_count >= options.retries {
                    logger.log(&format!(
                        "Failed to copy after {} retries: {} -> {}, Error: {}",
                        options.retries,
                        src_path.display(),
                        dst_path.display(),
                        e
                    ));
                    stats.add_file_failed();
                    return Err(e);
                }

                logger.log(&format!(
                    "Retry {} of {}: {} -> {}, Error: {}",
                    retry_count,
                    options.retries,
                    src_path.display(),
                    dst_path.display(),
                    e
                ));

                thread::sleep(Duration::from_secs(options.wait_time));
            }
        }
    }

    Ok(())
}

fn copy_file_content(
    src_path: &Path,
    dst_path: &Path,
    total_size: u64,
    options: &CopyOptions,
) -> io::Result<()> {
    if options.empty_files {
        let mut dst_file = File::create(dst_path)?;
        dst_file.flush()?;
        return Ok(());
    }

    const BUFFER_SIZE: usize = 64 * 1024;
    let mut src_file = File::open(src_path)?;
    let mut dst_file = File::create(dst_path)?;

    let mut buffer = [0; BUFFER_SIZE];
    let mut bytes_copied: u64 = 0;
    let mut last_progress = 0;

    // Only show progress if threads == 1 (to avoid garbled output)
    // or if we implement a better UI later. For now, matching Robocopy /MT behavior (mostly silent on progress)
    let show_progress = options.show_progress && options.threads == 1;

    loop {
        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dst_file.write_all(&buffer[..bytes_read])?;

        if options.restartable {
            dst_file.flush()?;
        }

        bytes_copied += bytes_read as u64;

        if show_progress && total_size > 0 {
            let progress = ((bytes_copied * 100) / total_size) as usize;
            if progress > last_progress {
                print!("\rCopying: {}% complete", progress);
                io::stdout().flush()?;
                last_progress = progress;
            }
        }
    }

    if show_progress && total_size > 0 {
        println!("\rCopying: 100% complete");
    }

    dst_file.flush()?;
    Ok(())
}
