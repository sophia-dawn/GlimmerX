use tauri::State;

use crate::AppState;

/// Synchronize the UI language so backend error messages are localized too.
#[tauri::command]
pub fn set_ui_language(language: String) {
    crate::errors::set_language(&language);
}

#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or(crate::errors::message("errors.databaseLocked"))?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::settings::get_setting(&conn, &key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Reject endpoints that would send the API key in cleartext. Loopback
    // hosts stay allowed for local mock servers and development.
    if key == "ai.base_url" {
        crate::services::ai::validate_base_url(&value).map_err(|e| e.to_string())?;
    }

    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or(crate::errors::message("errors.databaseLocked"))?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::settings::set_setting(&conn, &key, &value).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn mock_app(db: Option<Database>) -> tauri::WebviewWindow<tauri::test::MockRuntime> {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(db),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![get_setting, set_setting])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap()
    }

    fn invoke(
        webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        cmd: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        let req = tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        tauri::test::get_ipc_response(webview, req)
            .map(|b| b.deserialize::<serde_json::Value>().unwrap())
    }

    #[test]
    fn test_setting_roundtrip_via_ipc() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("t.db"), "pw").unwrap();
        let webview = mock_app(Some(db));

        let missing = invoke(
            &webview,
            "get_setting",
            serde_json::json!({ "key": "nope" }),
        )
        .unwrap();
        assert!(missing.is_null());

        invoke(
            &webview,
            "set_setting",
            serde_json::json!({ "key": "theme", "value": "dark" }),
        )
        .unwrap();

        let got = invoke(
            &webview,
            "get_setting",
            serde_json::json!({ "key": "theme" }),
        )
        .unwrap();
        assert_eq!(got.as_str(), Some("dark"));
    }

    #[test]
    fn test_setting_commands_locked_db_fails() {
        let webview = mock_app(None);
        for cmd in ["get_setting", "set_setting"] {
            let err = invoke(
                &webview,
                cmd,
                serde_json::json!({ "key": "k", "value": "v" }),
            )
            .unwrap_err();
            let message = err.as_str().unwrap();
            assert!(!message.is_empty());
            assert!(!message.starts_with("errors."));
            assert!(!message.contains("not unlocked"));
        }
    }

    #[test]
    fn test_set_ai_base_url_rejects_cleartext_remote_host() {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("t.db"), "secret").unwrap();
        let webview = mock_app(Some(db));

        let err = invoke(
            &webview,
            "set_setting",
            serde_json::json!({
                "key": "ai.base_url",
                "value": "http://api.example.com/v1"
            }),
        )
        .unwrap_err();
        let message = err.as_str().unwrap();
        assert!(!message.starts_with("errors."), "{message}");
        assert!(message.contains("https"), "{message}");

        // Loopback is allowed for local mock servers.
        invoke(
            &webview,
            "set_setting",
            serde_json::json!({
                "key": "ai.base_url",
                "value": "http://127.0.0.1:9"
            }),
        )
        .unwrap();
    }
}
