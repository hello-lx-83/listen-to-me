use serde::Serialize;

use tauri::AppHandle;
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

use crate::{
    adapters::{
        ai::test_qwen_rewrite_connection,
        secrets::credential_store::{wipe_string, CredentialStore},
    },
    app_state::AppState,
    core::models::{is_supported_qwen_rewrite_model, AppSettings, QwenModelSettings},
    services::voice::VoiceService,
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QwenCredentialStatus {
    configured: bool,
}

#[tauri::command]
pub fn get_qwen_credential_status() -> Result<QwenCredentialStatus, String> {
    Ok(QwenCredentialStatus {
        configured: CredentialStore::has_qwen_api_key()?,
    })
}

#[tauri::command]
pub fn save_qwen_api_key(api_key: String) -> Result<QwenCredentialStatus, String> {
    let mut secret = api_key;
    let result = CredentialStore::save_qwen_api_key(secret.trim());
    wipe_string(&mut secret);
    result?;

    Ok(QwenCredentialStatus { configured: true })
}

#[tauri::command]
pub fn delete_qwen_api_key() -> Result<QwenCredentialStatus, String> {
    CredentialStore::delete_qwen_api_key()?;
    Ok(QwenCredentialStatus { configured: false })
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    state.store().settings()
}

#[tauri::command]
pub fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    state.store().update_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub fn get_qwen_model_settings(state: State<'_, AppState>) -> Result<QwenModelSettings, String> {
    state.store().qwen_model_settings()
}

#[tauri::command]
pub fn update_qwen_model_settings(
    state: State<'_, AppState>,
    settings: QwenModelSettings,
) -> Result<QwenModelSettings, String> {
    state.store().update_qwen_model_settings(&settings)?;
    Ok(settings)
}

#[tauri::command]
pub async fn test_qwen_asr_model() -> Result<String, String> {
    let mut api_key = CredentialStore::qwen_api_key()?;
    let result = VoiceService::test_cloud_asr(api_key.clone()).await;
    wipe_string(&mut api_key);
    result
}

#[tauri::command]
pub async fn test_qwen_rewrite_model(model: String) -> Result<(), String> {
    if !is_supported_qwen_rewrite_model(&model) {
        return Err("unsupported Qwen rewrite model".to_owned());
    }
    let mut api_key = CredentialStore::qwen_api_key()?;
    let result = test_qwen_rewrite_connection(api_key.clone(), model).await;
    wipe_string(&mut api_key);
    result
}

#[tauri::command]
pub fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    app.autolaunch()
        .is_enabled()
        .map_err(|_| "Windows autostart status is unavailable".to_owned())
}

#[tauri::command]
pub fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    let manager = app.autolaunch();
    if enabled {
        manager.enable()
    } else {
        manager.disable()
    }
    .map_err(|_| "Windows could not update the autostart setting".to_owned())?;
    manager
        .is_enabled()
        .map_err(|_| "Windows autostart status is unavailable".to_owned())
}
