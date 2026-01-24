use std::fs::{self, File};
use std::io::{self, Write, Seek};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use rand::{Rng, thread_rng};
use glob::Pattern;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Logger {
    file: Arc<Mutex<Option<File>>>,
}

impl Logger {
    pub fn new(file: Option<File>) -> Self {
        Logger {
            file: Arc::new(Mutex::new(file)),
        }
    }

    pub fn log(&self, message: &str) {
        // Print to stdout
        println!("{}", message);

        // Write to file if it exists
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = writeln!(file, "{}", message);
            }
        }
    }
    
    // Log only to file, not stdout
    pub fn log_file_only(&self, message: &str) {
        if let Ok(mut file_guard) = self.file.lock() {
            if let Some(file) = file_guard.as_mut() {
                let _ = writeln!(file, "{}", message);
            }
        }
    }
}

pub fn format_time(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::from_secs(0));
    let secs = duration.as_secs();

    let (hour, remainder) = (secs / 3600, secs % 3600);
    let (min, sec) = (remainder / 60, remainder % 60);

    format!("{:02}:{:02}:{:02}", hour % 24, min, sec)
}

pub fn matches_pattern(entry_name: &str, pattern: &str) -> bool {
    // Try glob first
    if let Ok(compiled_pattern) = Pattern::new(pattern) {
        if compiled_pattern.matches(entry_name) {
            return true;
        }
    }
    
    // Fallback/Legacy support
    if pattern == "*" || pattern == "*.*" {
        return true;
    }

    if let Some(suffix) = pattern.strip_prefix('*') {
        if pattern.ends_with('*') {
             // *contains*
             let substr = &suffix[..suffix.len() - 1];
             entry_name.contains(substr)
        } else {
             // *ends_with
             entry_name.ends_with(suffix)
        }
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        // starts_with*
        entry_name.starts_with(prefix)
    } else {
        entry_name == pattern
    }
}

pub fn securely_delete_file(path: &Path, logger: &Logger) -> io::Result<()> {
    let metadata = fs::metadata(path)?;
    let file_size = metadata.len();

    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)?;

    const BUFFER_SIZE: usize = 64 * 1024;
    let patterns = [0xFF, 0x00, 0xAA, 0x55, 0xF0, 0x0F];
    let mut buffer = vec![0; BUFFER_SIZE];

    for &pattern in &patterns {
        for item in buffer.iter_mut().take(BUFFER_SIZE) {
            *item = pattern;
        }

        file.seek(io::SeekFrom::Start(0))?;
        let mut remaining = file_size;
        while remaining > 0 {
            let to_write = std::cmp::min(remaining, BUFFER_SIZE as u64) as usize;
            file.write_all(&buffer[..to_write])?;
            remaining -= to_write as u64;
        }
        file.flush()?;
    }

    let mut rng = thread_rng();
    for item in buffer.iter_mut().take(BUFFER_SIZE) {
        *item = rng.gen_range(0..=255);
    }

    file.seek(io::SeekFrom::Start(0))?;
    let mut remaining = file_size;
    while remaining > 0 {
        let to_write = std::cmp::min(remaining, BUFFER_SIZE as u64) as usize;
        file.write_all(&buffer[..to_write])?;
        remaining -= to_write as u64;
    }
    file.flush()?;

    drop(file);
    fs::remove_file(path)?;

    logger.log_file_only(&format!("Securely deleted file: {}", path.display()));

    Ok(())
}

pub fn secure_remove_dir_all(dir: &Path, logger: &Logger) -> io::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                secure_remove_dir_all(&path, logger)?;
            } else {
                securely_delete_file(&path, logger)?;
            }
        }
        fs::remove_dir(dir)?;
        logger.log_file_only(&format!("Removed directory after secure file deletion: {}", dir.display()));
    }
    Ok(())
}
