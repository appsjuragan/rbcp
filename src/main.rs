use std::sync::Arc;
use rbcp::{CopyOptions, CopyEngine, CliProgress};
use rbcp::args::print_usage;

fn main() -> std::io::Result<()> {
    // Parse command line arguments
    let options = match CopyOptions::parse() {
        Ok(opts) => opts,
        Err(e) => {
            if e == "Not enough arguments" {
                let args: Vec<String> = std::env::args().collect();
                // If no args, we might want to launch GUI in the future.
                // For now, just print usage as before.
                // But wait, the requirement is "add gui capability when it available (desktop mode) without losing cli capability"
                // So if no args are provided, we should probably launch GUI.
                // But for this step (Phase 1/2), I'll stick to CLI behavior or maybe detect if I should launch GUI?
                // The plan says: "main.rs checks if args provided -> CLI mode. No args -> GUI mode."
                
                // Let's check if we have any args other than program name
                if args.len() <= 1 {
                    // TODO: Launch GUI here in Phase 3
                    print_usage(&args[0]);
                    return Ok(());
                }
                
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
