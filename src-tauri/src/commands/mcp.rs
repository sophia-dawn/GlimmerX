//! MCP-related commands: the settings UI and the approval resolution
//! callback from the webview (design doc 25, §5.4 / §8.1).

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::db::AppError;
use crate::mcp;
use crate::mcp::approval::ApprovalSlot;
use crate::mcp::host::{DbFn, McpHost};
use crate::mcp::McpRuntime;

/// Full MCP state for the settings page, including the token: this is the
/// user's own local UI and the token is only readable while unlocked.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConfigDto {
    pub enabled: bool,
    /// Configured (preferred) port.
    pub port: u16,
    /// Port actually bound; `null` when the server is not running.
    pub actual_port: Option<u16>,
    pub running: bool,
    pub trust_mode: String,
    pub token: String,
}

/// Run a settings mutation through the host seam; the guards stay inside.
fn host_update(
    runtime: &McpRuntime,
    f: impl FnOnce(&rusqlite::Connection) -> Result<(), String> + Send + 'static,
) -> Result<(), String> {
    let host: &Arc<dyn McpHost> = runtime.handle().ok_or("MCP runtime not initialized")?;
    let boxed: DbFn = Box::new(move |conn| {
        f(conn)
            .map(|_| serde_json::Value::Null)
            .map_err(AppError::IoError)
    });
    host.with_db(boxed).map(|_| ())
}

#[tauri::command]
pub async fn mcp_get_config(runtime: State<'_, McpRuntime>) -> Result<McpConfigDto, String> {
    let config = mcp::current_config(&runtime).map_err(|e| e.to_string())?;
    let status = mcp::status_from(&runtime);
    Ok(McpConfigDto {
        enabled: config.enabled,
        port: config.port,
        actual_port: status.port,
        running: status.running,
        trust_mode: config.trust.as_str().to_string(),
        token: config.token,
    })
}

/// Update any subset of the MCP config; persists and (re)starts the server.
#[tauri::command]
pub async fn mcp_set_config(
    runtime: State<'_, McpRuntime>,
    enabled: Option<bool>,
    port: Option<u16>,
    trust: Option<String>,
) -> Result<McpConfigDto, String> {
    host_update(&runtime, move |conn| {
        if let Some(enabled) = enabled {
            mcp::settings::save_enabled(conn, enabled).map_err(|e| e.to_string())?;
        }
        if let Some(port) = port {
            if port == 0 {
                return Err(crate::errors::message("errors.mcp.invalidPort").to_string());
            }
            mcp::settings::save_port(conn, port).map_err(|e| e.to_string())?;
        }
        if let Some(ref trust) = trust {
            mcp::settings::save_trust(conn, trust).map_err(|e| e.to_string())?;
        }
        Ok(())
    })?;
    mcp::sync_from(&runtime);
    mcp_get_config(runtime).await
}

/// Rotate the token. The auth middleware captured the old token, so the
/// server is hard-restarted when it was running.
#[tauri::command]
pub async fn mcp_reset_token(runtime: State<'_, McpRuntime>) -> Result<McpConfigDto, String> {
    host_update(&runtime, |conn| {
        mcp::settings::reset_token(conn)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })?;
    mcp::restart_from(&runtime);
    mcp_get_config(runtime).await
}

/// Resolve the pending approval request (the webview confirmation card).
/// Returns Ok(false) when the id no longer matches the pending request —
/// the card is stale, which the UI treats as "already handled".
#[tauri::command]
pub async fn mcp_approve_request(
    request_id: String,
    approved: bool,
    state: State<'_, ApprovalSlot>,
) -> Result<bool, String> {
    Ok(state.resolve(&request_id, approved))
}

/// Re-fetch the pending approval (e.g. after a window refocus).
#[tauri::command]
pub async fn mcp_current_approval(
    state: State<'_, ApprovalSlot>,
) -> Result<Option<mcp::approval::ApprovalRequest>, String> {
    Ok(state.current())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use crate::mcp::approval::{ApprovalRequest, StageOutcome};
    use crate::mcp::host::McpHost;
    use crate::AppState;
    use std::sync::{Arc, Mutex};
    use tauri::Manager;
    use tempfile::TempDir;

    /// A mock app with an unlocked ledger, an MCP runtime (host injected)
    /// and an empty approval slot — the surface the mcp commands need.
    fn mock_app() -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
        TempDir,
    ) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("t.db"), "pw").unwrap();
        let state = AppState {
            database: Mutex::new(Some(db)),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        };
        let app = tauri::test::mock_builder()
            .manage(state)
            .manage(crate::mcp::McpRuntime::default())
            .manage(ApprovalSlot::new())
            .invoke_handler(tauri::generate_handler![
                mcp_get_config,
                mcp_set_config,
                mcp_reset_token,
                mcp_approve_request,
                mcp_current_approval,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        {
            let runtime = app.state::<crate::mcp::McpRuntime>();
            runtime.set_handle(Arc::new(app.handle().clone()) as Arc<dyn McpHost>);
        }
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (app, webview, dir)
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

    fn pending(id: &str) -> ApprovalRequest {
        ApprovalRequest {
            request_id: id.to_string(),
            tool: "record_transaction".into(),
            request: serde_json::json!({"kind": "expense"}),
            created_at: "2026-11-14T12:00:00Z".into(),
        }
    }

    #[test]
    fn test_dto_field_names_are_camel_case() {
        // Lock the wire contract with the frontend type in one line.
        let fields: Vec<String> = serde_json::to_value(&McpConfigDto {
            enabled: false,
            port: 17823,
            actual_port: None,
            running: false,
            trust_mode: "confirm".into(),
            token: "t".repeat(64),
        })
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
        // serde_json maps sort keys; compare as a sorted set.
        let mut fields = fields;
        fields.sort();
        assert_eq!(
            fields,
            vec![
                "actualPort".to_string(),
                "enabled".to_string(),
                "port".to_string(),
                "running".to_string(),
                "token".to_string(),
                "trustMode".to_string(),
            ]
        );
    }

    #[test]
    fn test_app_error_display_is_localized_text() {
        // The MCP layer stringifies AppError via Display; ensure that keeps
        // producing user text rather than a Debug dump.
        let err = AppError::NotFound("errors.accountNotFound".into());
        assert!(!err.to_string().starts_with("NotFound"));
    }

    #[test]
    fn test_mcp_get_config_roundtrip() {
        let (_app, webview, _dir) = mock_app();
        let config = invoke(&webview, "mcp_get_config", serde_json::json!({})).unwrap();
        assert_eq!(config["enabled"], false);
        assert_eq!(config["port"], 17823);
        assert_eq!(config["trustMode"], "confirm");
        assert_eq!(config["token"].as_str().unwrap().len(), 64);
        assert!(config["running"] == false);
    }

    #[test]
    fn test_mcp_get_config_locked() {
        let dir = TempDir::new().unwrap();
        let state = AppState {
            database: Mutex::new(None),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        };
        let app = tauri::test::mock_builder()
            .manage(state)
            .manage(crate::mcp::McpRuntime::default())
            .invoke_handler(tauri::generate_handler![mcp_get_config])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        // Host injected, ledger locked: the command must report the lock,
        // not the uninitialized runtime.
        let runtime = app.state::<crate::mcp::McpRuntime>();
        runtime.set_handle(Arc::new(app.handle().clone()) as Arc<dyn McpHost>);
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let err = invoke(&webview, "mcp_get_config", serde_json::json!({})).unwrap_err();
        assert!(err.to_string().contains("解锁"));
        let _ = dir;
    }

    #[test]
    fn test_mcp_set_config_persists_fields() {
        let (_app, webview, _dir) = mock_app();
        // Server stays off: enabled is never set to true here.
        let config = invoke(
            &webview,
            "mcp_set_config",
            serde_json::json!({"port": 19000, "trust": "trust"}),
        )
        .unwrap();
        assert_eq!(config["port"], 19000);
        assert_eq!(config["trustMode"], "trust");

        let reread = invoke(&webview, "mcp_get_config", serde_json::json!({})).unwrap();
        assert_eq!(reread["port"], 19000);
        assert_eq!(reread["trustMode"], "trust");
    }

    #[test]
    fn test_mcp_set_config_rejects_bad_input() {
        let (_app, webview, _dir) = mock_app();
        let bad_port =
            invoke(&webview, "mcp_set_config", serde_json::json!({"port": 0})).unwrap_err();
        assert!(bad_port.to_string().contains("端口"));

        let bad_trust = invoke(
            &webview,
            "mcp_set_config",
            serde_json::json!({"trust": "yolo"}),
        )
        .unwrap_err();
        assert!(bad_trust.to_string().contains("信任模式"));
    }

    #[test]
    fn test_mcp_reset_token_rotates() {
        let (_app, webview, _dir) = mock_app();
        let before = invoke(&webview, "mcp_get_config", serde_json::json!({})).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();
        let after = invoke(&webview, "mcp_reset_token", serde_json::json!({})).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(before, after);
    }

    #[test]
    fn test_mcp_approval_resolution() {
        let (app, webview, _dir) = mock_app();
        let slot = app.state::<ApprovalSlot>();
        let StageOutcome::Waiting(_rx) = slot.try_insert(pending("req-1")) else {
            panic!("insert failed");
        };

        // Current approval surfaces the pending request.
        let current = invoke(&webview, "mcp_current_approval", serde_json::json!({})).unwrap();
        assert_eq!(current["requestId"], "req-1");

        // Matching id resolves and reports success.
        let matched = invoke(
            &webview,
            "mcp_approve_request",
            serde_json::json!({"requestId": "req-1", "approved": true}),
        )
        .unwrap();
        assert_eq!(matched, true);
        assert!(slot.current().is_none());

        // A stale id after the slot emptied reports false.
        let stale = invoke(
            &webview,
            "mcp_approve_request",
            serde_json::json!({"requestId": "gone", "approved": false}),
        )
        .unwrap();
        assert_eq!(stale, false);
    }
}
