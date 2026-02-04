use rbcp_core::{CopyEngine, CopyOptions, ProgressCallback, ProgressInfo, SharedProgress};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub progress: SharedProgress,
}

#[tauri::command]
pub async fn start_copy(
    app: AppHandle,
    state: State<'_, AppState>,
    options: CopyOptions,
) -> Result<(), String> {
    let options = options.clone();
    let progress = state.progress.clone();

    // Reset progress before starting
    progress.reset();

    // Span a thread for the copy operation
    std::thread::spawn(move || {
        let engine = CopyEngine::new(
            options,
            Arc::new(TauriProgress {
                app: app.clone(),
                shared: progress,
            }),
        );

        let _ = engine.run();
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_copy(state: State<'_, AppState>) -> Result<(), String> {
    state.progress.cancel();
    Ok(())
}

#[tauri::command]
pub fn toggle_pause(state: State<'_, AppState>) -> Result<(), String> {
    state.progress.toggle_pause();
    Ok(())
}

#[tauri::command]
pub fn check_conflicts(sources: Vec<String>, destination: String) -> Result<bool, String> {
    use std::path::Path;

    let dest_path = Path::new(&destination);
    if !dest_path.exists() {
        return Ok(false); // Destination doesn't exist, no conflicts
    }

    for source in &sources {
        let src_path = Path::new(source);
        if let Some(name) = src_path.file_name() {
            let target = dest_path.join(name);
            if target.exists() {
                return Ok(true); // Found a conflict
            }
        }
    }

    Ok(false)
}

// Wrapper to emit events to frontend
struct TauriProgress {
    app: AppHandle,
    shared: SharedProgress,
}

impl ProgressCallback for TauriProgress {
    fn on_progress(&self, info: &ProgressInfo) {
        self.shared.on_progress(info);
        let _ = self.app.emit("copy-progress", info);
    }

    fn on_log(&self, message: &str) {
        self.shared.on_log(message);
        let _ = self.app.emit("copy-log", message);
    }

    fn is_cancelled(&self) -> bool {
        self.shared.is_cancelled()
    }

    fn is_paused(&self) -> bool {
        self.shared.is_paused()
    }
}
