use tauri::State;

use crate::{app_state::AppState, core::models::HistoryRecord};

#[tauri::command]
pub fn list_history(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<HistoryRecord>, String> {
    state.store().list_history(limit.unwrap_or(200))
}

#[tauri::command]
pub fn delete_history(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.store().delete_history(id)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    state.store().clear_history()
}
