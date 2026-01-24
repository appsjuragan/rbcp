//! Progress tracking and callback infrastructure for the copy engine.
//!
//! This module provides traits and structs for reporting progress to
//! different frontends (CLI, GUI) without coupling the core engine
//! to any specific UI implementation.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Current state of a copy operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressState {
    /// Initial state, not started
    Idle,
    /// Scanning source to count files
    Scanning,
    /// Actively copying files
    Copying,
    /// Operation paused by user
    Paused,
    /// Operation cancelled by user
    Cancelled,
    /// Operation completed successfully
    Completed,
    /// Operation failed with error
    Failed,
}

/// Information about the current progress of a copy operation
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    /// Current state of the operation
    pub state: ProgressState,
    /// Path of the file currently being processed
    pub current_file: String,
    /// Number of files processed so far
    pub files_done: u64,
    /// Total number of files to process (0 if not yet scanned)
    pub files_total: u64,
    /// Number of bytes copied so far
    pub bytes_done: u64,
    /// Total bytes to copy (0 if not yet scanned)
    pub bytes_total: u64,
    /// Current file's bytes copied
    pub current_file_bytes_done: u64,
    /// Current file's total bytes
    pub current_file_bytes_total: u64,
}

impl Default for ProgressInfo {
    fn default() -> Self {
        Self {
            state: ProgressState::Idle,
            current_file: String::new(),
            files_done: 0,
            files_total: 0,
            bytes_done: 0,
            bytes_total: 0,
            current_file_bytes_done: 0,
            current_file_bytes_total: 0,
        }
    }
}

impl ProgressInfo {
    /// Calculate overall progress as a percentage (0-100)
    pub fn percentage(&self) -> f32 {
        if self.bytes_total == 0 {
            0.0
        } else {
            (self.bytes_done as f32 / self.bytes_total as f32) * 100.0
        }
    }

    /// Calculate current file progress as a percentage (0-100)
    pub fn file_percentage(&self) -> f32 {
        if self.current_file_bytes_total == 0 {
            0.0
        } else {
            (self.current_file_bytes_done as f32 / self.current_file_bytes_total as f32) * 100.0
        }
    }
}

/// Trait for receiving progress updates from the copy engine.
///
/// Implementations of this trait can be used to update CLI progress bars,
/// GUI progress indicators, or any other progress display mechanism.
pub trait ProgressCallback: Send + Sync {
    /// Called when progress information is updated
    fn on_progress(&self, info: &ProgressInfo);

    /// Called when a log message is generated
    fn on_log(&self, message: &str);

    /// Check if the operation should be cancelled
    fn is_cancelled(&self) -> bool;

    /// Check if the operation should be paused
    fn is_paused(&self) -> bool;

    /// Wait while paused (blocking)
    fn wait_if_paused(&self) {
        while self.is_paused() && !self.is_cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
}

/// A null progress callback that does nothing.
/// Useful for headless/silent operations.
pub struct NullProgress;

impl ProgressCallback for NullProgress {
    fn on_progress(&self, _info: &ProgressInfo) {}
    fn on_log(&self, _message: &str) {}
    fn is_cancelled(&self) -> bool { false }
    fn is_paused(&self) -> bool { false }
}

/// A CLI progress callback that prints to stdout.
pub struct CliProgress {
    cancel_flag: Arc<AtomicBool>,
    show_progress: bool,
    show_file_names: bool,
}

impl CliProgress {
    pub fn new(show_progress: bool, show_file_names: bool) -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            show_progress,
            show_file_names,
        }
    }

    /// Get a handle to request cancellation
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        self.cancel_flag.clone()
    }
}

impl ProgressCallback for CliProgress {
    fn on_progress(&self, info: &ProgressInfo) {
        if !self.show_progress {
            return;
        }

        match info.state {
            ProgressState::Scanning => {
                print!("\rScanning: {} files found...", info.files_total);
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            ProgressState::Copying => {
                let pct = info.percentage();
                print!(
                    "\r{:.0}% - {} of {} files",
                    pct, info.files_done, info.files_total
                );
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            ProgressState::Completed => {
                println!("\nCompleted!");
            }
            _ => {}
        }
    }

    fn on_log(&self, message: &str) {
        if self.show_file_names {
            println!("{}", message);
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    fn is_paused(&self) -> bool {
        false // CLI doesn't support pause
    }
}

/// Shared progress state that can be accessed by both the engine and UI.
/// This is useful for GUI applications where the UI thread needs to
/// poll the current progress.
#[derive(Clone)]
pub struct SharedProgress {
    cancel_flag: Arc<AtomicBool>,
    pause_flag: Arc<AtomicBool>,
    info: Arc<std::sync::Mutex<ProgressInfo>>,
    log_messages: Arc<std::sync::Mutex<Vec<String>>>,
}

impl SharedProgress {
    pub fn new() -> Self {
        Self {
            cancel_flag: Arc::new(AtomicBool::new(false)),
            pause_flag: Arc::new(AtomicBool::new(false)),
            info: Arc::new(std::sync::Mutex::new(ProgressInfo::default())),
            log_messages: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Request cancellation of the current operation
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::Relaxed);
    }

    /// Toggle pause state
    pub fn toggle_pause(&self) {
        let current = self.pause_flag.load(Ordering::Relaxed);
        self.pause_flag.store(!current, Ordering::Relaxed);
    }

    /// Set pause state
    pub fn set_paused(&self, paused: bool) {
        self.pause_flag.store(paused, Ordering::Relaxed);
    }

    /// Get the current progress info
    pub fn get_info(&self) -> ProgressInfo {
        self.info.lock().unwrap().clone()
    }

    /// Get and clear log messages
    pub fn take_logs(&self) -> Vec<String> {
        let mut logs = self.log_messages.lock().unwrap();
        std::mem::take(&mut *logs)
    }

    /// Get log messages without clearing
    pub fn peek_logs(&self) -> Vec<String> {
        self.log_messages.lock().unwrap().clone()
    }

    /// Reset the progress state for a new operation
    pub fn reset(&self) {
        self.cancel_flag.store(false, Ordering::Relaxed);
        self.pause_flag.store(false, Ordering::Relaxed);
        *self.info.lock().unwrap() = ProgressInfo::default();
        self.log_messages.lock().unwrap().clear();
    }
}

impl Default for SharedProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressCallback for SharedProgress {
    fn on_progress(&self, info: &ProgressInfo) {
        *self.info.lock().unwrap() = info.clone();
    }

    fn on_log(&self, message: &str) {
        self.log_messages.lock().unwrap().push(message.to_string());
    }

    fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::Relaxed)
    }

    fn is_paused(&self) -> bool {
        self.pause_flag.load(Ordering::Relaxed)
    }
}
