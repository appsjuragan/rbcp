use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Statistics {
    pub dirs_created: AtomicUsize,
    pub files_copied: AtomicUsize,
    pub bytes_copied: AtomicU64,
    pub dirs_skipped: AtomicUsize,
    pub files_skipped: AtomicUsize,
    pub files_failed: AtomicUsize,
    pub dirs_removed: AtomicUsize,
    pub files_removed: AtomicUsize,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics {
            dirs_created: AtomicUsize::new(0),
            files_copied: AtomicUsize::new(0),
            bytes_copied: AtomicU64::new(0),
            dirs_skipped: AtomicUsize::new(0),
            files_skipped: AtomicUsize::new(0),
            files_failed: AtomicUsize::new(0),
            dirs_removed: AtomicUsize::new(0),
            files_removed: AtomicUsize::new(0),
        }
    }
}

impl Statistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_dir_created(&self) {
        self.dirs_created.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_file_copied(&self, bytes: u64) {
        self.files_copied.fetch_add(1, Ordering::Relaxed);
        self.bytes_copied.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn add_dir_skipped(&self) {
        self.dirs_skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_file_skipped(&self) {
        self.files_skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_file_failed(&self) {
        self.files_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_dir_removed(&self) {
        self.dirs_removed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_file_removed(&self) {
        self.files_removed.fetch_add(1, Ordering::Relaxed);
    }
}

impl fmt::Display for Statistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Statistics:")?;
        writeln!(
            f,
            "    Directories created: {}",
            self.dirs_created.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Files copied:        {}",
            self.files_copied.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Bytes copied:        {}",
            self.bytes_copied.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Directories skipped: {}",
            self.dirs_skipped.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Files skipped:       {}",
            self.files_skipped.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Files failed:        {}",
            self.files_failed.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Directories removed: {}",
            self.dirs_removed.load(Ordering::Relaxed)
        )?;
        writeln!(
            f,
            "    Files removed:       {}",
            self.files_removed.load(Ordering::Relaxed)
        )
    }
}
