#![allow(clippy::collapsible_if)]
mod args;
mod copy;
mod stats;
mod utils;

use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

use crate::args::{CopyOptions, print_usage};
use crate::copy::copy_directory;
use crate::stats::Statistics;
use crate::utils::{Logger, format_time};

fn main() -> io::Result<()> {
    // Parse command line arguments
    let options = match CopyOptions::parse() {
        Ok(opts) => opts,
        Err(e) => {
            if e == "Not enough arguments" {
                let args: Vec<String> = std::env::args().collect();
                print_usage(&args[0]);
                return Ok(());
            }
            eprintln!("Error: {}", e);
            return Ok(());
        }
    };

    let source_dir = &options.source;
    let dest_dir = &options.destination;

    // Check if source directory exists
    let source_path = Path::new(source_dir);
    if !source_path.exists() {
        eprintln!("ERROR: Source directory does not exist: {}", source_dir);
        return Ok(());
    }

    // Configure thread pool if needed
    if options.threads > 1 {
        ThreadPoolBuilder::new()
            .num_threads(options.threads)
            .build_global()
            .unwrap();
    }

    // Initialize logger
    let log_file = if let Some(log_path) = &options.log_file {
        Some(File::create(log_path)?)
    } else {
        None
    };
    let logger = Logger::new(log_file);

    // Log start message
    let start_time = SystemTime::now();
    let start_msg = format!(
        "-------------------------------------------------------------------------------\n\
         ColemenCopy - Started: {}\n\
         Source: {}\n\
         Destination: {}\n\
         Patterns: {}\n\
         Options: {}\n\
         -------------------------------------------------------------------------------\n",
        format_time(start_time),
        source_dir,
        dest_dir,
        options.patterns.join(" "),
        format_options(&options)
    );

    logger.log(&start_msg);

    // Create destination directory if it doesn't exist
    let dest_path = Path::new(dest_dir);
    if !dest_path.exists() {
        if !options.list_only {
            logger.log(&format!("Creating destination directory: {}", dest_dir));
            fs::create_dir_all(dest_path)?;
        } else {
            logger.log(&format!("Would create destination directory: {}", dest_dir));
        }
    }

    // Perform the copy operation
    let stats = Statistics::new();

    // Handle child-only mode
    if options.child_only && source_path.is_dir() {
        // Process each child directory individually
        if let Ok(entries) = fs::read_dir(source_path) {
            let entries: Vec<_> = entries.collect::<Result<Vec<_>, _>>()?;

            let process_child = |entry: &fs::DirEntry| -> io::Result<()> {
                let child_path = entry.path();
                if child_path.is_dir() {
                    let child_name = child_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let child_dest = dest_path.join(&child_name);

                    // Log the child directory processing
                    logger.log(&format!("\nProcessing child directory: {}", child_name));

                    // Process this child directory
                    copy_directory(&child_path, &child_dest, &options, &logger, &stats)?;
                }
                Ok(())
            };

            if options.threads > 1 {
                entries.par_iter().try_for_each(process_child)?;
            } else {
                entries.iter().try_for_each(process_child)?;
            }
        }
    } else {
        // Regular mode - process the entire source directory
        copy_directory(source_path, dest_path, &options, &logger, &stats)?;
    }

    // Log completion message
    let end_time = SystemTime::now();
    let elapsed = end_time
        .duration_since(start_time)
        .unwrap_or(Duration::from_secs(0));

    // Load stats atomically
    use std::sync::atomic::Ordering;

    let summary = format!(
        "-------------------------------------------------------------------------------\n\
         ColemenCopy - Finished: {}\n\
         Source: {}\n\
         Destination: {}\n\n\
         Statistics:\n\
             Directories: {}\n\
             Files: {}\n\
             Bytes: {}\n\
             Directories skipped: {}\n\
             Files skipped: {}\n\
             Files failed: {}\n\
             Directories removed: {}\n\
             Files removed: {}\n\n\
         Elapsed time: {} seconds\n\
         -------------------------------------------------------------------------------\n",
        format_time(end_time),
        source_dir,
        dest_dir,
        stats.dirs_created.load(Ordering::Relaxed),
        stats.files_copied.load(Ordering::Relaxed),
        stats.bytes_copied.load(Ordering::Relaxed),
        stats.dirs_skipped.load(Ordering::Relaxed),
        stats.files_skipped.load(Ordering::Relaxed),
        stats.files_failed.load(Ordering::Relaxed),
        stats.dirs_removed.load(Ordering::Relaxed),
        stats.files_removed.load(Ordering::Relaxed),
        elapsed.as_secs()
    );

    logger.log(&summary);

    Ok(())
}

fn format_options(options: &CopyOptions) -> String {
    let mut result = Vec::new();

    if options.recursive {
        if options.include_empty {
            result.push("/E".to_string());
        } else {
            result.push("/S".to_string());
        }
    }

    if options.restartable {
        result.push("/Z".to_string());
    }

    if options.backup_mode {
        result.push("/B".to_string());
    }

    if options.mirror {
        result.push("/MIR".to_string());
    } else if options.purge {
        result.push("/PURGE".to_string());
    }

    if options.move_dirs {
        result.push("/MOVE".to_string());
    } else if options.move_files {
        result.push("/MOV".to_string());
    }

    if !options.attributes_add.is_empty() {
        result.push(format!("/A+:{}", options.attributes_add));
    }

    if !options.attributes_remove.is_empty() {
        result.push(format!("/A-:{}", options.attributes_remove));
    }

    if options.threads != 1 {
        result.push(format!("/MT:{}", options.threads));
    }

    if options.retries != 1_000_000 {
        result.push(format!("/R:{}", options.retries));
    }

    if options.wait_time != 30 {
        result.push(format!("/W:{}", options.wait_time));
    }

    if options.list_only {
        result.push("/L".to_string());
    }

    if !options.show_progress {
        result.push("/NP".to_string());
    }

    if !options.log_file_names {
        result.push("/NFL".to_string());
    }

    if options.empty_files {
        result.push("/EMPTY".to_string());
    }

    if options.child_only {
        result.push("/CHILDONLY".to_string());
    }

    if options.shred_files {
        result.push("/SHRED".to_string());
    }

    result.join(" ")
}
