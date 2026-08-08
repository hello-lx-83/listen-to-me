use tauri::State;

use crate::{
    app_state::AppState,
    core::models::{DictionaryCategory, DictionaryEntry, DictionaryEntryInput},
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

#[tauri::command]
pub fn list_dictionary_categories(
    state: State<'_, AppState>,
) -> Result<Vec<DictionaryCategory>, String> {
    state.store().list_dictionary_categories()
}

#[tauri::command]
pub fn create_dictionary_category(
    state: State<'_, AppState>,
    name: String,
) -> Result<DictionaryCategory, String> {
    state.store().create_dictionary_category(&name)
}

#[tauri::command]
pub fn rename_dictionary_category(
    state: State<'_, AppState>,
    old_name: String,
    new_name: String,
) -> Result<DictionaryCategory, String> {
    state
        .store()
        .rename_dictionary_category(&old_name, &new_name)
}

#[tauri::command]
pub fn delete_dictionary_category(state: State<'_, AppState>, name: String) -> Result<(), String> {
    state.store().delete_dictionary_category(&name)
}
