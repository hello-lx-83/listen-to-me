use tauri::{AppHandle, Manager, State};

use crate::{
    adapters::secrets::credential_store::CredentialStore,
    app_state::AppState,
    core::models::{AppSnapshot, DashboardOverview},
};

#[tauri::command]
pub fn show_splashscreen(app: AppHandle) {
    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        let _ = splashscreen.show();
    }
}

#[tauri::command]
pub fn get_app_snapshot(state: State<'_, AppState>) -> AppSnapshot {
    state.snapshot()
}

#[tauri::command]
pub fn get_dashboard_overview(state: State<'_, AppState>) -> Result<DashboardOverview, String> {
    let (history_count, dictionary_count) = state.store().dashboard_counts()?;
    Ok(DashboardOverview {
        qwen_configured: CredentialStore::has_qwen_api_key()?,
        history_count,
        dictionary_count,
    })
}

#[tauri::command]
pub fn finish_startup(app: AppHandle) {
    if let Some(splashscreen) = app.get_webview_window("splashscreen") {
        let _ = splashscreen.close();
    }

    if let Some(main) = app.get_webview_window("main") {
        let _ = main.show();
        let _ = main.set_focus();
    }
}
