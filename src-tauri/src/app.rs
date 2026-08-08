use tauri::Manager;

use crate::{adapters::persistence::sqlite::SqliteStore, app_state::AppState, commands};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            #[cfg(target_os = "windows")]
            crate::platform::windows::lifecycle::show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let store = SqliteStore::open(&data_dir.join("listen-to-me.sqlite3"))
                .map_err(std::io::Error::other)?;
            app.manage(AppState::new(store));

            #[cfg(target_os = "windows")]
            crate::platform::windows::lifecycle::setup_tray(app)?;

            if let Some(overlay) = app.get_webview_window("voice-overlay") {
                overlay.set_focusable(false)?;
            }

            #[cfg(target_os = "windows")]
            crate::platform::windows::voice_runtime::start(app.handle().clone())
                .map_err(std::io::Error::other)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::show_splashscreen,
            commands::app::finish_startup,
            commands::app::get_app_snapshot,
            commands::app::get_dashboard_overview,
            commands::settings::get_qwen_credential_status,
            commands::settings::save_qwen_api_key,
            commands::settings::delete_qwen_api_key,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_qwen_model_settings,
            commands::settings::update_qwen_model_settings,
            commands::settings::test_qwen_asr_model,
            commands::settings::test_qwen_rewrite_model,
            commands::settings::get_autostart_enabled,
            commands::settings::set_autostart_enabled,
            commands::history::list_history,
            commands::history::delete_history,
            commands::history::clear_history,
            commands::dictionary::list_dictionary,
            commands::dictionary::upsert_dictionary,
            commands::dictionary::delete_dictionary,
            commands::dictionary::list_dictionary_categories,
            commands::dictionary::create_dictionary_category,
            commands::dictionary::rename_dictionary_category,
            commands::dictionary::delete_dictionary_category,
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Listen to Me");
}
