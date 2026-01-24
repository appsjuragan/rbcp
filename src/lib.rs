//! RBCP - Robust Copy Utility Library
//!
//! This library provides the core copy engine that can be used by both
//! CLI and GUI frontends.

pub mod args;
pub mod copy;
pub mod stats;
pub mod utils;

mod engine;
mod progress;

pub use args::CopyOptions;
pub use engine::CopyEngine;
pub use progress::{ProgressCallback, ProgressInfo, ProgressState, NullProgress};
pub use stats::Statistics;
pub use utils::Logger;

/// Application version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Application name
pub const APP_NAME: &str = "RBCP";
