use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self, File, Metadata};
use std::io::{self, Read, Write};
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::args::CopyOptions;
use crate::progress::{ProgressCallback, ProgressInfo, ProgressState};
use crate::stats::Statistics;
use crate::utils::{Logger, matches_pattern, secure_remove_dir_all, securely_delete_file};

pub fn copy_directory(
    src_dir: &Path,
    dst_dir: &Path,
    options: &CopyOptions,
    logger: &Logger,
    stats: &Statistics,
    progress: &dyn ProgressCallback,
) -> io::Result<()> {
    // Check for cancellation
    if progress.is_cancelled() {
        return Ok(());
    }
    progress.wait_if_paused();

    // Ensure the destination directory exists
    if !dst_dir.exists() {
        if !options.list_only {
            let msg = format!("Creating directory: {}", dst_dir.display());
            progress.on_log(&msg);
            logger.log(&msg);
            fs::create_dir_all(dst_dir)?;
            stats.add_dir_created();
        } else {
            let msg = format!("Would create directory: {}", dst_dir.display());
            progress.on_log(&msg);
            logger.log(&msg);
            stats.add_dir_created();
        }
    }

    // Collect the source files and directories
    // We collect them into a Vec to enable parallel iteration
    let entries: Vec<_> = fs::read_dir(src_dir)?.collect::<Result<Vec<_>, io::Error>>()?;

    // We need to keep track of source filenames for the purge step
    let src_names: HashSet<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    // Process entries in parallel if threads > 1, otherwise sequential
    let process_entry = |entry: &fs::DirEntry| -> io::Result<()> {
        if progress.is_cancelled() {
            return Ok(());
        }

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
                copy_file(&path, &dst_path, options, logger, stats, progress)?;
            }
        } else if path.is_dir() && options.recursive {
            let dst_subdir = dst_dir.join(&file_name);

            // Skip empty directories if not including them
            if !options.include_empty {
                let is_empty = path.read_dir()?.next().is_none();
                if is_empty {
                    if options.log_file_names {
                        let msg = format!("Skipping empty directory: {}", path.display());
                        progress.on_log(&msg);
                        logger.log(&msg);
                    }
                    stats.add_dir_skipped();
                    return Ok(());
                }
            }

            copy_directory(&path, &dst_subdir, options, logger, stats, progress)?;

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
                if progress.is_cancelled() {
                    return Ok(());
                }

                let path = entry.path();
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();

                if !src_names.contains(&file_name) {
                    if path.is_file() {
                        if options.shred_files {
                            let msg = format!("Securely removing file: {}", path.display());
                            progress.on_log(&msg);
                            logger.log(&msg);
                            securely_delete_file(&path, logger)?;
                        } else {
                            let msg = format!("Removing file: {}", path.display());
                            progress.on_log(&msg);
                            logger.log(&msg);
                            fs::remove_file(&path)?;
                        }
                        stats.add_file_removed();
                    } else if path.is_dir() {
                        if options.shred_files {
                            let msg = format!("Securely removing directory: {}", path.display());
                            progress.on_log(&msg);
                            logger.log(&msg);
                            secure_remove_dir_all(&path, logger)?;
                        } else {
                            let msg = format!("Removing directory: {}", path.display());
                            progress.on_log(&msg);
                            logger.log(&msg);
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

fn should_copy_file(src_meta: &Metadata, dst_meta: Option<&Metadata>, force_overwrite: bool) -> bool {
    if force_overwrite {
        return true;
    }

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
    progress: &dyn ProgressCallback,
) -> io::Result<()> {
    if progress.is_cancelled() {
        return Ok(());
    }
    progress.wait_if_paused();

    let src_meta = fs::metadata(src_path)?;
    let dst_meta = fs::metadata(dst_path).ok();

    if !should_copy_file(&src_meta, dst_meta.as_ref(), options.force_overwrite) {
        stats.add_file_skipped();
        return Ok(());
    }

    if options.list_only {
        let msg = format!(
            "Would copy file: {} -> {}",
            src_path.display(),
            dst_path.display()
        );
        progress.on_log(&msg);
        logger.log(&msg);
        stats.add_file_copied(src_meta.len());
        return Ok(());
    }

    if options.log_file_names {
        let msg = format!(
            "Copying file: {} -> {}",
            src_path.display(),
            dst_path.display()
        );
        progress.on_log(&msg);
        logger.log(&msg);
    }

    let mut retry_count = 0;
    loop {
        if progress.is_cancelled() {
            return Ok(());
        }

        match copy_file_content(src_path, dst_path, src_meta.len(), options, progress) {
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

                            // Apply using attrib command
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
    progress: &dyn ProgressCallback,
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
    
    // Create a local progress info to update
    let mut progress_info = ProgressInfo {
        state: ProgressState::Copying,
        current_file: src_path.to_string_lossy().to_string(),
        current_file_bytes_total: total_size,
        ..Default::default()
    };

    loop {
        if progress.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Cancelled"));
        }
        progress.wait_if_paused();

        let bytes_read = src_file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        dst_file.write_all(&buffer[..bytes_read])?;

        if options.restartable {
            dst_file.flush()?;
        }

        bytes_copied += bytes_read as u64;
        
        // Update progress
        // Note: For global progress (files_done, total_bytes_done), we rely on the engine/stats
        // But here we can report the file progress
        progress_info.current_file_bytes_done = bytes_copied;
        progress.on_progress(&progress_info);
    }

    dst_file.flush()?;
    Ok(())
}
