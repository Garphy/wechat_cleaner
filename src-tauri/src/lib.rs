pub mod types;
pub mod scanner;
pub mod cleaner;
pub mod config;
pub mod platform;
pub mod commands;
pub mod error;
pub mod debug;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::detect_wechat_paths,
            commands::get_config,
            commands::save_config,
            commands::validate_directory,
            commands::start_scan,
            commands::get_scan_progress,
            commands::pause_scan,
            commands::resume_scan,
            commands::cancel_scan,
            commands::get_scan_result,
            commands::get_paged_results,
            commands::execute_cleanup,
            commands::check_wechat_running,
            commands::set_debug_mode,
            commands::get_debug_mode,
            commands::get_log_path,
            commands::clear_debug_log,
            commands::debug_get_scan_result_state,
        ])
        .setup(|app| {
            // Initialize debug log
            debug::init_debug_log();

            // Load config and check debug mode
            let config = crate::config::wechat::load_config();
            debug::set_debug_enabled(config.debug_enabled);
            debug::log(&format!("Application started. debug_enabled={}", config.debug_enabled));

            // Auto-open DevTools if debug mode is enabled
            if config.debug_enabled {
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                    debug::log("DevTools opened (debug mode enabled)");
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
