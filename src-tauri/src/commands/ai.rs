use tauri::State;

use crate::db::categories::{create_category, find_by_type_and_name, list_by_type, CategoryType};
use crate::db::{accounts, settings, transactions, AppError};
use crate::services::ai::{self, AiConfig, AiContext};
use crate::AppState;

// ---------------------------------------------------------------------------
// Helpers (category matching, account inference)
// ---------------------------------------------------------------------------

/// Match or create a category by name and type.
/// 1. Exact match (case-insensitive, trimmed)
/// 2. Contains match (bidirectional)
/// 3. Auto-create if not found
fn resolve_category(
    conn: &rusqlite::Connection,
    name: &str,
    category_type: &CategoryType,
) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::AiError("parseFailed".to_string()));
    }

    // 1. Exact match
    if let Some(cat) = find_by_type_and_name(conn, category_type, trimmed)? {
        return Ok(cat.id);
    }

    // 2. Contains match (bidirectional)
    let existing = list_by_type(conn, category_type)?;
    let lower = trimmed.to_lowercase();
    for cat in &existing {
        let cat_lower = cat.name.to_lowercase();
        if cat_lower.contains(&lower) || lower.contains(&cat_lower) {
            return Ok(cat.id.clone());
        }
    }

    // 3. Auto-create
    let id = create_category(conn, trimmed, category_type, None)?;
    Ok(id)
}

/// Infer the source asset account from AI hint or fall back to default.
/// When matching by hint, longer account-name matches are preferred to avoid
/// a short name (e.g. "现金") hijacking a more specific hint (e.g. "微信").
fn resolve_source_account(
    accounts: &[accounts::AccountRecord],
    hint: &Option<String>,
    default_id: &Option<String>,
) -> Result<String, AppError> {
    let mut asset_accounts: Vec<&accounts::AccountRecord> = accounts
        .iter()
        .filter(|a| a.account_type == "asset" && a.is_active)
        .collect();
    // Prefer longer names first so specific accounts win over generic ones.
    asset_accounts.sort_by_key(|a| std::cmp::Reverse(a.name.len()));

    // 1. Try matching hint against account names (bidirectional, single pass)
    if let Some(hint_str) = hint {
        let hint_lower = hint_str.to_lowercase();
        for acc in &asset_accounts {
            let cat_lower = acc.name.to_lowercase();
            if cat_lower.contains(&hint_lower) || hint_lower.contains(&cat_lower) {
                return Ok(acc.id.clone());
            }
        }
    }

    // 2. Fall back to default
    if let Some(default) = default_id {
        if asset_accounts.iter().any(|a| &a.id == default) {
            return Ok(default.clone());
        }
    }

    Err(AppError::AiError("noSourceAccount".to_string()))
}

/// Load AiConfig from settings. Returns error if any required key is missing.
fn load_ai_config(conn: &rusqlite::Connection) -> Result<AiConfig, AppError> {
    let base_url = settings::get_setting(conn, "ai.base_url")?
        .ok_or_else(|| AppError::AiError("noBaseUrl".to_string()))?;
    let api_key = settings::get_setting(conn, "ai.api_key")?
        .ok_or_else(|| AppError::AiError("noApiKey".to_string()))?;
    let model = settings::get_setting(conn, "ai.model")?
        .ok_or_else(|| AppError::AiError("noModel".to_string()))?;

    Ok(AiConfig {
        base_url,
        api_key,
        model,
    })
}

/// Build AiContext from current DB state.
fn build_ai_context(conn: &rusqlite::Connection) -> Result<AiContext, AppError> {
    let today = crate::utils::time::today_date();
    let tz = chrono::Local::now().offset().to_string();

    let expense_cats = list_by_type(conn, &CategoryType::Expense)?
        .into_iter()
        .map(|c| c.name)
        .collect();
    let income_cats = list_by_type(conn, &CategoryType::Income)?
        .into_iter()
        .map(|c| c.name)
        .collect();

    let all_accounts = accounts::list_accounts(conn)?;
    let asset_accounts = all_accounts
        .iter()
        .filter(|a| a.account_type == "asset" && a.is_active)
        .map(|a| a.name.clone())
        .collect();

    Ok(AiContext {
        today,
        timezone_offset: tz,
        expense_categories: expense_cats,
        income_categories: income_cats,
        asset_accounts,
    })
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn ai_parse_transaction(
    text: String,
    state: State<'_, AppState>,
) -> Result<crate::commands::transactions::TransactionDto, String> {
    // Phase 1: Load config and context (sync, drop conn before await)
    let (config, context, default_source_id, all_accounts) = {
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or("Database is not unlocked. Please unlock it first.")?;
        let conn = db.get_conn().map_err(|e| e.to_string())?;

        let config = load_ai_config(&conn).map_err(|e| e.to_string())?;
        let context = build_ai_context(&conn).map_err(|e| e.to_string())?;
        let default_source_id = settings::get_setting(&conn, "ai.default_source_account_id")
            .map_err(|e| e.to_string())?;
        let all_accounts = accounts::list_accounts(&conn).map_err(|e| e.to_string())?;
        (config, context, default_source_id, all_accounts)
    };

    // Phase 2: Call AI (async, no DB lock held)
    let ai_result = ai::parse_transaction(&config, &text, &context)
        .await
        .map_err(|e| e.to_string())?;

    // Validate fields (no DB needed)
    ai::validate_mode(&ai_result.mode).map_err(|e| e.to_string())?;
    ai::validate_amount(&ai_result.amount).map_err(|e| e.to_string())?;
    let date = ai::validate_date(&ai_result.date).map_err(|e| e.to_string())?;

    // Phase 3: Write to DB (re-acquire conn)
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    // Resolve category
    let category_type = match ai_result.mode.as_str() {
        "expense" => CategoryType::Expense,
        "income" => CategoryType::Income,
        _ => return Err(AppError::AiError("invalidMode".to_string()).to_string()),
    };

    let category_id = if let Some(ref cat_name) = ai_result.category_name {
        Some(resolve_category(&conn, cat_name, &category_type).map_err(|e| e.to_string())?)
    } else {
        None
    };

    // Resolve accounts (reuse accounts fetched in Phase 1)
    let (source_account_id, destination_account_id) = match ai_result.mode.as_str() {
        "expense" => {
            let source =
                resolve_source_account(&all_accounts, &ai_result.account_hint, &default_source_id)
                    .map_err(|e| e.to_string())?;
            (Some(source), None)
        }
        "income" => {
            let dest =
                resolve_source_account(&all_accounts, &ai_result.account_hint, &default_source_id)
                    .map_err(|e| e.to_string())?;
            (None, Some(dest))
        }
        _ => {
            return Err(AppError::AiError("invalidMode".to_string()).to_string());
        }
    };

    // Assemble QuickAddInput and call quick_add_transaction
    let quick_input = transactions::QuickAddInput {
        mode: ai_result.mode.clone(),
        amount: ai_result.amount.clone(),
        source_account_id,
        destination_account_id,
        category_id,
        description: Some(ai_result.description.clone()),
        date: Some(date),
    };

    let tx_with_postings =
        transactions::quick_add_transaction(&conn, &quick_input).map_err(|e| e.to_string())?;

    Ok(crate::commands::transactions::to_dto(tx_with_postings))
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> Result<(), String> {
    let config = {
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or("Database is not unlocked. Please unlock it first.")?;
        let conn = db.get_conn().map_err(|e| e.to_string())?;
        load_ai_config(&conn).map_err(|e| e.to_string())?
    };

    ai::test_connection(&config)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts::{create_account_with_path, CreateAccountFullInput};
    use crate::db::{Database, RecentDbs};
    use std::io::{Read, Write};
    use std::sync::Mutex;
    use tauri::Manager;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Shared test helpers
    // -----------------------------------------------------------------------

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    fn seed_ai_settings(conn: &rusqlite::Connection, base_url: &str) {
        settings::set_setting(conn, "ai.base_url", base_url).unwrap();
        settings::set_setting(conn, "ai.api_key", "sk-test").unwrap();
        settings::set_setting(conn, "ai.model", "test-model").unwrap();
    }

    fn create_asset_account(conn: &rusqlite::Connection, path: &str) -> String {
        create_account_with_path(
            conn,
            path,
            "CNY",
            Some(&CreateAccountFullInput {
                description: None,
                account_number: None,
                iban: None,
                is_active: true,
                include_net_worth: true,
            }),
        )
        .unwrap()
    }

    fn mock_app(
        db: Option<Database>,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(db),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![
                ai_parse_transaction,
                ai_test_connection
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (app, webview)
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

    static NO_PROXY_ONCE: std::sync::Once = std::sync::Once::new();

    fn ensure_no_proxy() {
        NO_PROXY_ONCE.call_once(|| {
            for var in [
                "HTTP_PROXY",
                "HTTPS_PROXY",
                "ALL_PROXY",
                "http_proxy",
                "https_proxy",
                "all_proxy",
            ] {
                std::env::remove_var(var);
            }
        });
    }

    /// Serve `responses` sequentially on a random local port.
    fn spawn_mock_server(responses: Vec<(u16, &'static str, String)>) -> String {
        ensure_no_proxy();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            for (status, reason, body) in responses {
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let mut buf = [0u8; 8192];
                let _ = stream.read(&mut buf);
                let response = format!(
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        format!("http://{}", addr)
    }

    fn chat_body(content: &str) -> String {
        serde_json::json!({ "choices": [{ "message": { "content": content } }] }).to_string()
    }

    fn ai_content(mode: &str, category: Option<&str>, hint: Option<&str>) -> String {
        format!(
            r#"{{"mode":"{}","amount":"12.50","categoryName":{},"accountHint":{},"date":"2026-01-15","description":"lunch"}}"#,
            mode,
            category
                .map(|c| format!(r#""{}""#, c))
                .unwrap_or("null".into()),
            hint.map(|h| format!(r#""{}""#, h)).unwrap_or("null".into()),
        )
    }

    // -----------------------------------------------------------------------
    // Helper unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_category_exact_match_case_insensitive() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        let id = create_category(&conn, "餐饮美食", &CategoryType::Expense, None).unwrap();
        assert_eq!(
            resolve_category(&conn, "  餐饮美食 ", &CategoryType::Expense).unwrap(),
            id
        );
    }

    #[test]
    fn test_resolve_category_contains_match() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        let id = create_category(&conn, "餐饮美食", &CategoryType::Expense, None).unwrap();
        assert_eq!(
            resolve_category(&conn, "餐饮", &CategoryType::Expense).unwrap(),
            id
        );
        assert_eq!(
            resolve_category(&conn, "超好吃的餐饮美食", &CategoryType::Expense).unwrap(),
            id
        );
    }

    #[test]
    fn test_resolve_category_autocreate() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        let id = resolve_category(&conn, "全新分类", &CategoryType::Income).unwrap();
        let found = find_by_type_and_name(&conn, &CategoryType::Income, "全新分类")
            .unwrap()
            .unwrap();
        assert_eq!(found.id, id);
    }

    #[test]
    fn test_resolve_category_empty_name_fails() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        assert!(resolve_category(&conn, "", &CategoryType::Expense).is_err());
        assert!(resolve_category(&conn, "   ", &CategoryType::Expense).is_err());
    }

    fn make_account(id: &str, name: &str, is_active: bool) -> accounts::AccountRecord {
        accounts::AccountRecord {
            id: id.into(),
            name: name.into(),
            account_type: "asset".into(),
            currency: "CNY".into(),
            description: String::new(),
            account_number: None,
            is_system: false,
            iban: None,
            is_active,
            include_net_worth: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn make_accounts() -> Vec<accounts::AccountRecord> {
        vec![
            make_account("id-cash", "Cash", true),
            make_account("id-wechat-long-name", "WeChat Wallet", true),
            make_account("id-inactive", "Old Wallet", false),
        ]
    }

    #[test]
    fn test_resolve_source_account_prefers_longer_hint_match() {
        let accs = make_accounts();
        // Hint "wallet" matches both; longer name must win.
        let id = resolve_source_account(&accs, &Some("wallet".into()), &None).unwrap();
        assert_eq!(id, "id-wechat-long-name");
    }

    #[test]
    fn test_resolve_source_account_falls_back_to_default() {
        let accs = make_accounts();
        let id = resolve_source_account(&accs, &None, &Some("id-cash".into())).unwrap();
        assert_eq!(id, "id-cash");
    }

    #[test]
    fn test_resolve_source_account_default_must_be_active_asset() {
        let accs = make_accounts();
        assert!(resolve_source_account(&accs, &None, &Some("id-inactive".into())).is_err());
        assert!(resolve_source_account(&accs, &None, &Some("missing".into())).is_err());
    }

    #[test]
    fn test_resolve_source_account_no_match_errors() {
        let accs = make_accounts();
        assert!(resolve_source_account(&accs, &None, &None).is_err());
    }

    #[test]
    fn test_load_ai_config_ok_and_missing() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        assert!(load_ai_config(&conn).is_err());
        seed_ai_settings(&conn, "http://127.0.0.1:9");
        let cfg = load_ai_config(&conn).unwrap();
        assert_eq!(cfg.model, "test-model");
        assert_eq!(cfg.base_url, "http://127.0.0.1:9");
    }

    #[test]
    fn test_build_ai_context_collects_names() {
        let (_dir, db) = test_db();
        let conn = db.get_conn().unwrap();
        create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();
        create_category(&conn, "Salary", &CategoryType::Income, None).unwrap();
        create_asset_account(&conn, "Assets/Cash");
        let ctx = build_ai_context(&conn).unwrap();
        assert_eq!(ctx.expense_categories, vec!["Food".to_string()]);
        assert_eq!(ctx.income_categories, vec!["Salary".to_string()]);
        assert_eq!(ctx.asset_accounts, vec!["Cash".to_string()]);
        assert!(!ctx.today.is_empty());
        assert!(!ctx.timezone_offset.is_empty());
    }

    // -----------------------------------------------------------------------
    // Command-level tests via IPC
    // -----------------------------------------------------------------------

    #[test]
    fn test_ai_parse_transaction_expense_flow() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
            create_asset_account(&conn, "Assets/WeChat");
        }
        // Restart DB? No — settings written before app starts; reuse same db.
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&ai_content("expense", Some("餐饮美食"), Some("wechat"))),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let resp = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "lunch cost 12.5" }),
        )
        .unwrap();
        assert_eq!(resp["description"], "lunch");

        // Category auto-created, transaction recorded with balanced postings.
        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let db = db_state.as_ref().unwrap();
        let conn = db.get_conn().unwrap();
        let cat = find_by_type_and_name(&conn, &CategoryType::Expense, "餐饮美食")
            .unwrap()
            .expect("category should be auto-created");
        drop(cat);
        let txs = transactions::list_transactions(&conn, None, None, None).unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_ai_parse_transaction_income_flow() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
            let wechat = create_asset_account(&conn, "Assets/WeChat");
            // Income without a hint falls back to the default destination account.
            settings::set_setting(&conn, "ai.default_source_account_id", &wechat).unwrap();
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&ai_content("income", Some("Salary"), None)),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let resp = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "got paid" }),
        )
        .unwrap();
        assert_eq!(resp["description"], "lunch");
        assert_eq!(resp["postings"].as_array().map(Vec::len), Some(2));

        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let conn = db_state.as_ref().unwrap().get_conn().unwrap();
        let txs = transactions::list_transactions(&conn, None, None, None).unwrap();
        assert_eq!(txs.len(), 1);
    }

    #[test]
    fn test_ai_parse_transaction_invalid_mode_from_ai() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&ai_content("transfer", None, None)),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("invalidMode"));
    }

    #[test]
    fn test_ai_parse_transaction_invalid_amount_from_ai() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&ai_content("expense", None, None).replace("\"12.50\"", "\"-1\"")),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("invalidAmount"));
    }

    #[test]
    fn test_ai_parse_transaction_missing_date_from_ai() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&ai_content("expense", None, None).replace(r#""2026-01-15""#, "null")),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("missingDate"));
    }

    #[test]
    fn test_ai_parse_transaction_locked_db_fails() {
        let (_app, webview) = mock_app(None);
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("not unlocked"));
    }

    #[test]
    fn test_ai_parse_transaction_missing_config_fails() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app(Some(db));
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("noBaseUrl"));
    }

    #[test]
    fn test_ai_test_connection_success() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        let base_url = spawn_mock_server(vec![(200, "OK", "{}".to_string())]);
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, &base_url);
        }
        let (_app, webview) = mock_app(Some(sdb));
        invoke(&webview, "ai_test_connection", serde_json::json!({})).unwrap();
    }

    #[test]
    fn test_ai_test_connection_failure() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        let base_url = spawn_mock_server(vec![(500, "Internal Server Error", "{}".to_string())]);
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, &base_url);
        }
        let (_app, webview) = mock_app(Some(sdb));
        let err = invoke(&webview, "ai_test_connection", serde_json::json!({})).unwrap_err();
        assert!(err.as_str().unwrap().contains("apiCallFailed"));
    }
}
