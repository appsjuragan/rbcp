// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(commands::AppState {
            progress: rbcp_core::SharedProgress::new(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_copy,
            commands::cancel_copy,
            commands::toggle_pause,
            commands::check_conflicts
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
