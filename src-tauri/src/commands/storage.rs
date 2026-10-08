use tauri::State;

use crate::app::state::{AppState, StorageInfo};

#[tauri::command]
pub(crate) fn get_storage_info(state: State<'_, AppState>) -> StorageInfo {
    StorageInfo::from(&*state)
}
