#![windows_subsystem = "windows"]

use std::sync::Arc;
use rbcp::{CopyOptions, CopyEngine, CliProgress};
use rbcp::args::print_usage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if we should launch GUI (no args provided)
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        rbcp::run_gui()?;
        return Ok(());
    }

    // Parse command line arguments
    let options = match CopyOptions::parse() {
        Ok(opts) => opts,
        Err(e) => {
            if e == "Not enough arguments" {
                // This case is technically unreachable now due to check above,
                // but CopyOptions::parse might still return it if logic changes.
                print_usage(&args[0]);
                return Ok(());
            }
            eprintln!("Error: {}", e);
            return Ok(());
        }
    };

    // Create progress callback
    let progress = Arc::new(CliProgress::new(
        options.show_progress,
        options.log_file_names
    ));

    // Create and run engine
    let engine = CopyEngine::new(options, progress.clone());
    
    // Handle Ctrl+C
    let cancel_handle = progress.cancel_handle();
    ctrlc::set_handler(move || {
        println!("\nCancelling...");
        use std::sync::atomic::Ordering;
        cancel_handle.store(true, Ordering::Relaxed);
    }).expect("Error setting Ctrl-C handler");

    match engine.run() {
        Ok(_) => Ok(()),
        Err(e) => {
            // If cancelled, it might return error or ok depending on implementation.
            // Engine returns Ok(stats) even if cancelled/failed usually, unless critical error.
            // But run() returns io::Result.
            eprintln!("Execution finished with error: {}", e);
            Ok(())
        }
    }
}
