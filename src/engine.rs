use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::fs::{self, File};
use rayon::ThreadPoolBuilder;

use crate::args::CopyOptions;
use crate::progress::{ProgressCallback, ProgressInfo, ProgressState};
use crate::stats::Statistics;
use crate::utils::{Logger, format_time};
use crate::copy::copy_directory;

pub struct CopyEngine {
    options: CopyOptions,
    stats: Arc<Statistics>,
    progress: Arc<dyn ProgressCallback>,
}

impl CopyEngine {
    pub fn new(options: CopyOptions, progress: Arc<dyn ProgressCallback>) -> Self {
        Self {
            options,
            stats: Arc::new(Statistics::new()),
            progress,
        }
    }

    pub fn run(&self) -> std::io::Result<Arc<Statistics>> {
        let source_dir = &self.options.source;
        let dest_dir = &self.options.destination;
        let source_path = Path::new(source_dir);
        let dest_path = Path::new(dest_dir);

        // Check if source directory exists
        if !source_path.exists() {
            let msg = format!("ERROR: Source directory does not exist: {}", source_dir);
            self.progress.on_log(&msg);
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, msg));
        }

        // Configure thread pool if needed
        if self.options.threads > 1 {
            let _ = ThreadPoolBuilder::new()
                .num_threads(self.options.threads)
                .build_global(); // Ignore error if already initialized
        }

        // Initialize logger
        let log_file = if let Some(log_path) = &self.options.log_file {
            Some(File::create(log_path)?)
        } else {
            None
        };
        let logger = Logger::new(log_file);

        // Log start message
        let start_time = SystemTime::now();
        let start_msg = format!(
            "-------------------------------------------------------------------------------\n\
             RBCP - Started: {}\n\
             Source: {}\n\
             Destination: {}\n\
             Patterns: {}\n\
             Options: {}\n\
             -------------------------------------------------------------------------------\n",
            format_time(start_time),
            source_dir,
            dest_dir,
            self.options.patterns.join(" "),
            self.options.to_string_flags()
        );
        self.progress.on_log(&start_msg);
        logger.log(&start_msg);

        // Scan source for progress info
        let mut total_files = 0;
        let mut total_bytes = 0;
        
        if self.options.show_progress {
             let mut info = ProgressInfo::default();
             info.state = ProgressState::Scanning;
             self.progress.on_progress(&info);
             
             if let Ok((files, bytes)) = self.scan_source(source_path) {
                 total_files = files;
                 total_bytes = bytes;
                 info.files_total = files;
                 info.bytes_total = bytes;
                 self.progress.on_progress(&info);
             }
        }

        // Create destination directory if it doesn't exist
        if !dest_path.exists() {
            if !self.options.list_only {
                let msg = format!("Creating destination directory: {}", dest_dir);
                self.progress.on_log(&msg);
                logger.log(&msg);
                fs::create_dir_all(dest_path)?;
            } else {
                let msg = format!("Would create destination directory: {}", dest_dir);
                self.progress.on_log(&msg);
                logger.log(&msg);
            }
        }

        // Perform the copy operation
        let mut info = ProgressInfo {
            state: ProgressState::Copying,
            files_total: total_files,
            bytes_total: total_bytes,
            ..Default::default()
        };
        self.progress.on_progress(&info);

        // Wrapper to merge global stats with local progress
        struct ProgressWrapper<'a> {
            inner: &'a dyn ProgressCallback,
            stats: &'a Statistics,
            total_files: u64,
            total_bytes: u64,
        }

        impl<'a> ProgressCallback for ProgressWrapper<'a> {
            fn on_progress(&self, info: &ProgressInfo) {
                // Merge local file progress with global stats
                use std::sync::atomic::Ordering;
                let files_done = self.stats.files_copied.load(Ordering::Relaxed) as u64;
                let bytes_done = self.stats.bytes_copied.load(Ordering::Relaxed);
                
                let mut new_info = info.clone();
                new_info.files_done = files_done;
                
                // Total bytes done = bytes of fully copied files + bytes of current file
                new_info.bytes_done = bytes_done + info.current_file_bytes_done;
                
                new_info.files_total = self.total_files;
                new_info.bytes_total = self.total_bytes;
                
                self.inner.on_progress(&new_info);
            }
            
            fn on_log(&self, message: &str) { self.inner.on_log(message); }
            fn is_cancelled(&self) -> bool { self.inner.is_cancelled() }
            fn is_paused(&self) -> bool { self.inner.is_paused() }
        }

        let wrapper = ProgressWrapper {
            inner: self.progress.as_ref(),
            stats: &self.stats,
            total_files,
            total_bytes,
        };

        // Handle child-only mode
        if self.options.child_only && source_path.is_dir() {
             if let Ok(entries) = fs::read_dir(source_path) {
                let entries: Vec<_> = entries.collect::<Result<Vec<_>, _>>()?;
                
                use rayon::prelude::*;
                
                let process_child = |entry: &fs::DirEntry| -> std::io::Result<()> {
                    let child_path = entry.path();
                    if child_path.is_dir() {
                        let child_name = child_path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let child_dest = dest_path.join(&child_name);

                        let msg = format!("\nProcessing child directory: {}", child_name);
                        self.progress.on_log(&msg);
                        logger.log(&msg);

                        copy_directory(&child_path, &child_dest, &self.options, &logger, &self.stats, &wrapper)?;
                    }
                    Ok(())
                };

                if self.options.threads > 1 {
                    entries.par_iter().try_for_each(process_child)?;
                } else {
                    entries.iter().try_for_each(process_child)?;
                }
            }
        } else {
            copy_directory(source_path, dest_path, &self.options, &logger, &self.stats, &wrapper)?;
        }

        // Log completion
        let end_time = SystemTime::now();
        let elapsed = end_time.duration_since(start_time).unwrap_or(Duration::from_secs(0));
        
        use std::sync::atomic::Ordering;
        let summary = format!(
            "-------------------------------------------------------------------------------\n\
             RBCP - Finished: {}\n\
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
            self.stats.dirs_created.load(Ordering::Relaxed),
            self.stats.files_copied.load(Ordering::Relaxed),
            self.stats.bytes_copied.load(Ordering::Relaxed),
            self.stats.dirs_skipped.load(Ordering::Relaxed),
            self.stats.files_skipped.load(Ordering::Relaxed),
            self.stats.files_failed.load(Ordering::Relaxed),
            self.stats.dirs_removed.load(Ordering::Relaxed),
            self.stats.files_removed.load(Ordering::Relaxed),
            elapsed.as_secs()
        );

        self.progress.on_log(&summary);
        logger.log(&summary);
        
        info.state = ProgressState::Completed;
        self.progress.on_progress(&info);

        Ok(self.stats.clone())
    }

    fn scan_source(&self, path: &Path) -> std::io::Result<(u64, u64)> {
        let mut files = 0;
        let mut bytes = 0;
        
        if path.is_dir() {
            let entries = match fs::read_dir(path) {
                Ok(e) => e,
                Err(e) => {
                    // Log error but don't fail the entire scan
                    self.progress.on_log(&format!("Warning: Could not scan directory {}: {}", path.display(), e));
                    return Ok((0, 0));
                }
            };

            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        if self.options.recursive {
                            if let Ok((f, b)) = self.scan_source(&path) {
                                files += f;
                                bytes += b;
                            }
                        }
                    } else {
                         let file_name = path.file_name().unwrap_or_default().to_string_lossy();
                         let matches = self.options.patterns.iter().any(|p| crate::utils::matches_pattern(&file_name, p));
                         if matches {
                             files += 1;
                             if let Ok(metadata) = fs::metadata(&path) {
                                bytes += metadata.len();
                             }
                         }
                    }
                }
            }
        } else if path.is_file() {
             // If source is a file (not typical for this app but possible if user passed file path)
             // The app assumes source is dir usually.
             // But let's handle it safely.
             let file_name = path.file_name().unwrap().to_string_lossy();
             let matches = self.options.patterns.iter().any(|p| crate::utils::matches_pattern(&file_name, p));
             if matches {
                 files += 1;
                 bytes += fs::metadata(&path)?.len();
             }
        }
        Ok((files, bytes))
    }
}
