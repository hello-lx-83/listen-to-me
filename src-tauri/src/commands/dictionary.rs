use tauri::State;

use crate::{
    app_state::AppState,
    core::models::{DictionaryEntry, DictionaryEntryInput},
};

#[tauri::command]
pub fn list_dictionary(state: State<'_, AppState>) -> Result<Vec<DictionaryEntry>, String> {
    state.store().list_dictionary()
}

#[tauri::command]
pub fn upsert_dictionary(
    state: State<'_, AppState>,
    input: DictionaryEntryInput,
) -> Result<DictionaryEntry, String> {
    state.store().upsert_dictionary(&input)
}

#[tauri::command]
pub fn delete_dictionary(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    state.store().delete_dictionary(id)
}
