use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::settings::get_setting(&conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::settings::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}
