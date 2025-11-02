// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod analysis;
mod commands;
mod demangler;
mod storage;

use commands::AppState;

fn main() {
    // Set environment variables for Linux Wayland compatibility
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
        std::env::set_var("GDK_BACKEND", "x11");
    }

    // Initialize app state
    let app_state = AppState::new().expect("Failed to initialize app state");

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::analyze_directory,
            commands::list_platforms,
            commands::demangle_name,
            commands::demangle_batch,
            commands::save_schedule,
            commands::load_schedule,
            commands::list_schedules,
            commands::delete_schedule,
            commands::get_storage_stats,
            commands::pick_directory,
            commands::get_app_version,
            commands::health_check,
            commands::list_board_configs,
            commands::validate_board_config,
            commands::export_board_config,
            commands::analyze_multicore,
            commands::analyze_veecle_project,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
