//! Integration tests for the MCP server: a real mock app, a real axum
//! server on loopback, real JSON-RPC POSTs. This is the only layer that can
//! reach the macro-generated tool dispatch, the auth middleware and the
//! Streamable HTTP assembly at once.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;

use crate::db::{Database, RecentDbs};
use crate::mcp::approval::ApprovalSlot;
use crate::mcp::{McpHost, McpRuntime};
use crate::AppState;

/// Unique port per test so parallel test threads never collide.
fn setup_app(trust: &str, port: u16) -> (tauri::App<tauri::test::MockRuntime>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::create(&dir.path().join("t.db"), "pw").unwrap();
    {
        let conn = db.get_conn().unwrap();
        crate::db::settings::set_setting(&conn, "mcp.enabled", "true").unwrap();
        crate::db::settings::set_setting(&conn, "mcp.port", &port.to_string()).unwrap();
        crate::db::settings::set_setting(&conn, "mcp.trust", trust).unwrap();
        crate::db::accounts::create_account_with_path(&conn, "Assets/Cash", None).unwrap();
        crate::db::accounts::create_account_with_path(&conn, "Assets/WeChat", None).unwrap();
    }
    let state = AppState {
        database: Mutex::new(Some(db)),
        recent_dbs: Mutex::new(RecentDbs::empty()),
    };
    let app = tauri::test::mock_builder()
        .manage(state)
        .manage(McpRuntime::default())
        .manage(ApprovalSlot::new())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .unwrap();
    // The same injection the production `.setup()` hook performs.
    let runtime = app.state::<McpRuntime>();
    runtime.set_handle(Arc::new(app.handle().clone()) as Arc<dyn McpHost>);
    crate::mcp::sync_from(&app.state::<McpRuntime>());
    (app, dir)
}

fn stop(app: &tauri::App<tauri::test::MockRuntime>) {
    crate::mcp::sync_from(&app.state::<McpRuntime>());
}

async fn wait_up(client: &reqwest::Client, url: &str, token: &str) -> reqwest::Response {
    for _ in 0..100 {
        if let Ok(resp) = client
            .post(url)
            .header("Authorization", format!("Bearer {token}"))
            .json(&serde_json::json!({
                "jsonrpc": "2.0", "id": 1, "method": "ping", "params": {}
            }))
            .send()
            .await
        {
            return resp;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("MCP server never came up at {url}");
}

async fn call(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({
            "jsonrpc": "2.0", "id": 1, "method": method, "params": params
        }))
        .send()
        .await
        .expect("request");
    assert!(
        resp.status().is_success(),
        "{method} failed: {}",
        resp.status()
    );
    resp.json().await.expect("json body")
}

/// Spawnable variant of a tools/call: owns its url/token.
async fn call_tool(
    url: String,
    token: String,
    name: &str,
    args: serde_json::Value,
) -> serde_json::Value {
    call(
        &reqwest::Client::new(),
        &url,
        &token,
        "tools/call",
        serde_json::json!({"name": name, "arguments": args}),
    )
    .await
}

/// Run a closure on the app's database; every guard stays inside.
fn with_app_conn<T>(
    app: &tauri::App<tauri::test::MockRuntime>,
    f: impl FnOnce(&rusqlite::Connection) -> T,
) -> T {
    let state = app.state::<AppState>();
    let guard = state.database.lock().unwrap();
    let db = guard.as_ref().expect("db");
    let conn = db.get_conn().expect("conn");
    f(&conn)
}

/// Result content as text (MCP wraps tool output in content blocks).
fn tool_text(result: &serde_json::Value) -> String {
    result["result"]["content"][0]["text"]
        .as_str()
        .expect("text content")
        .to_string()
}

fn token_of(app: &tauri::App<tauri::test::MockRuntime>) -> String {
    with_app_conn(app, |conn| {
        crate::mcp::settings::load_config(conn).unwrap().token
    })
}

#[tokio::test]
async fn test_unauthorized_without_token() {
    let (app, _dir) = setup_app("readonly", 19501);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19501/mcp".to_string();
    wait_up(&client, &url, &token_of(&app)).await;

    let resp = client
        .post(&url)
        .header("Accept", "application/json, text/event-stream")
        .json(&serde_json::json!({"jsonrpc":"2.0","id":1,"method":"ping","params":{}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNAUTHORIZED);
    stop(&app);
}

#[tokio::test]
async fn test_tools_list_and_reads() {
    let (app, _dir) = setup_app("readonly", 19502);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19502/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    // initialize handshake first, like any well-behaved client.
    let init = call(
        &client,
        &url,
        &token,
        "initialize",
        serde_json::json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "test", "version": "1.0"}
        }),
    )
    .await;
    assert!(init["result"].is_object(), "initialize: {init}");

    let list = call(&client, &url, &token, "tools/list", serde_json::json!({})).await;
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected in [
        "ledger_status",
        "list_accounts",
        "list_categories",
        "query_transactions",
        "get_balances",
        "summary",
        "record_transaction",
        "create_account",
    ] {
        assert!(names.contains(&expected), "missing tool {expected}");
    }

    // A read works in read-only mode.
    let accounts = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({"name": "list_accounts", "arguments": {}}),
    )
    .await;
    let accounts = tool_text(&accounts);
    assert!(accounts.contains("Cash"), "accounts: {accounts}");
    assert!(accounts.contains("WeChat"), "accounts: {accounts}");

    // ledger_status reports the unlocked state and the trust mode.
    let status = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({"name": "ledger_status", "arguments": {}}),
    )
    .await;
    let status = tool_text(&status);
    assert!(status.contains("readonly"), "status: {status}");
    assert!(status.contains("t.db"), "status: {status}");
    stop(&app);
}

#[tokio::test]
async fn test_readonly_rejects_writes() {
    let (app, _dir) = setup_app("readonly", 19503);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19503/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    let result = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({
            "name": "record_transaction",
            "arguments": {"kind": "expense", "amount_cents": 1850, "source_account": "Cash", "category": "咖啡"}
        }),
    )
    .await;
    let text = tool_text(&result);
    assert!(text.contains("read-only"), "write in readonly: {text}");
    // isError marker present so the agent sees a tool-level failure.
    assert!(result["result"]["isError"].as_bool().unwrap_or(false));

    // Nothing landed in the ledger.
    let count: i64 = with_app_conn(&app, |conn| {
        conn.query_row("SELECT count(*) FROM transactions", [], |r| r.get(0))
            .unwrap()
    });
    assert_eq!(count, 0);
    stop(&app);
}

#[tokio::test]
async fn test_trust_mode_writes_directly() {
    let (app, _dir) = setup_app("trust", 19504);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19504/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    let result = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({
            "name": "record_transaction",
            "arguments": {
                "kind": "expense",
                "amount_cents": 1850,
                "date": "2026-11-14",
                "description": "coffee",
                "source_account": "Cash",
                "category": "咖啡"
            }
        }),
    )
    .await;
    let text = tool_text(&result);
    assert!(text.contains("transactionId"), "outcome: {text}");
    assert!(text.contains("postings"), "outcome: {text}");

    let postings: i64 = with_app_conn(&app, |conn| {
        conn.query_row("SELECT count(*) FROM postings", [], |r| r.get(0))
            .unwrap()
    });
    // Expense posts twice: the asset account and the category's mirror
    // P&L account are one and the same account.
    assert_eq!(postings, 2);
    stop(&app);
}

#[tokio::test]
async fn test_confirm_mode_approval_flow() {
    let (app, _dir) = setup_app("confirm", 19505);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19505/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    // The write blocks on approval; issue it concurrently.
    let call_task = tokio::spawn(call_tool(
        url.clone(),
        token.clone(),
        "record_transaction",
        serde_json::json!({
            "kind": "expense",
            "amount_cents": 4200,
            "description": "lunch",
            "source_account": "Cash",
            "category": "餐饮"
        }),
    ));

    // Wait for the pending approval to appear in the slot, then approve.
    let slot = app.state::<ApprovalSlot>();
    let mut request_id = None;
    for _ in 0..200 {
        if let Some(pending) = slot.current() {
            request_id = Some(pending.request_id.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let Some(request_id) = request_id else {
        // Debug: what did the call actually return?
        let result = tokio::time::timeout(Duration::from_secs(4), call_task)
            .await
            .expect("call task hung")
            .unwrap();
        panic!("approval never staged; call returned: {result}");
    };
    assert!(slot.resolve(&request_id, true));

    let result = call_task.await.unwrap();
    let text = tool_text(&result);
    assert!(text.contains("transactionId"), "approved write: {text}");

    // The slot is free again.
    assert!(slot.current().is_none());
    stop(&app);
}

#[tokio::test]
async fn test_confirm_mode_rejection() {
    let (app, _dir) = setup_app("confirm", 19506);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19506/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    let call_task = tokio::spawn(call_tool(
        url.clone(),
        token.clone(),
        "create_account",
        serde_json::json!({"path": "Assets/Alipay", "opening_balance_cents": 10000}),
    ));

    let slot = app.state::<ApprovalSlot>();
    let mut request_id = None;
    for _ in 0..200 {
        if let Some(pending) = slot.current() {
            request_id = Some(pending.request_id.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let request_id = request_id.expect("approval never staged");
    assert!(slot.resolve(&request_id, false));

    let result = call_task.await.unwrap();
    let text = tool_text(&result);
    assert!(text.contains("Rejected"), "rejected write: {text}");
    stop(&app);
}

#[tokio::test]
async fn test_locked_ledger_reports_locked() {
    let (app, _dir) = setup_app("trust", 19507);
    let client = reqwest::Client::new();
    let url = "http://127.0.0.1:19507/mcp".to_string();
    let token = token_of(&app);
    wait_up(&client, &url, &token).await;

    // Lock: same state transition db_lock performs.
    *app.state::<AppState>().database.lock().unwrap() = None;

    let result = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({"name": "list_accounts", "arguments": {}}),
    )
    .await;
    let text = tool_text(&result);
    assert!(
        text.contains("解锁") || text.contains("lock"),
        "locked: {text}"
    );

    let status = call(
        &client,
        &url,
        &token,
        "tools/call",
        serde_json::json!({"name": "ledger_status", "arguments": {}}),
    )
    .await;
    assert!(tool_text(&status).contains("解锁"));
    stop(&app);
}
