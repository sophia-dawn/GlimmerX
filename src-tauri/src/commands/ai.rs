use tauri::State;

use crate::db::categories::{list_by_type, CategoryType};
use crate::db::{accounts, settings, AppError};
use crate::ledger::{resolve_account, AccountRef};
use crate::services::ai::{self, AiConfig, AiContext};
use crate::AppState;

// ---------------------------------------------------------------------------
// Helpers (config, context, proposal resolution)
// ---------------------------------------------------------------------------

/// Load AiConfig from settings. Returns error if any required key is missing.
fn load_ai_config(conn: &rusqlite::Connection) -> Result<AiConfig, AppError> {
    let base_url = settings::get_setting(conn, "ai.base_url")?
        .ok_or_else(|| AppError::AiError("noBaseUrl".to_string()))?;
    // Defence in depth: settings written by an older build may still hold an
    // insecure endpoint.
    crate::services::ai::validate_base_url(&base_url)?;
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
// Proposal resolution (design doc 25, §4.2 — propose only, never write)
// ---------------------------------------------------------------------------

/// An account the proposal resolved to, with the id the confirm step needs.
#[derive(Debug, Clone, PartialEq)]
struct ResolvedAccount {
    id: String,
    name: String,
}

/// Resolve an account the AI named; fall back to the configured default when
/// it did not. The default must still be an active asset account.
fn resolve_named_or_default(
    accounts: &[accounts::AccountRecord],
    name: &Option<String>,
    default_id: &Option<String>,
    error_code: &str,
) -> Result<(ResolvedAccount, Vec<String>), AppError> {
    let raw = name.as_deref().map(str::trim).filter(|s| !s.is_empty());
    if let Some(raw) = raw {
        let (id, warning) = resolve_account(accounts, &AccountRef::Name(raw.to_string()))?;
        let acc = accounts.iter().find(|a| a.id == id).expect("resolved id");
        let warnings = warning.into_iter().collect();
        return Ok((
            ResolvedAccount {
                id,
                name: acc.name.clone(),
            },
            warnings,
        ));
    }

    if let Some(default) = default_id {
        if let Some(acc) = accounts
            .iter()
            .find(|a| &a.id == default && a.is_active && a.account_type == "asset")
        {
            return Ok((
                ResolvedAccount {
                    id: acc.id.clone(),
                    name: acc.name.clone(),
                },
                vec![format!("未指定账户，使用默认账户「{}」", acc.name)],
            ));
        }
    }
    Err(AppError::AiError(error_code.to_string()))
}

/// Resolve an account the AI must name (transfer endpoints): no default
/// fallback — silently guessing a transfer endpoint is unacceptable.
fn resolve_required_named(
    accounts: &[accounts::AccountRecord],
    name: &Option<String>,
    error_code: &str,
) -> Result<(ResolvedAccount, Vec<String>), AppError> {
    let raw = name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::AiError(error_code.to_string()))?;
    let (id, warning) = resolve_account(accounts, &AccountRef::Name(raw.to_string()))?;
    let acc = accounts.iter().find(|a| a.id == id).expect("resolved id");
    let warnings = warning.into_iter().collect();
    Ok((
        ResolvedAccount {
            id,
            name: acc.name.clone(),
        },
        warnings,
    ))
}

/// Warn when the proposed category would be newly created rather than
/// matched against an existing one (fuzzy check mirrors the db layer's
/// `resolve_or_create` contains-match).
fn category_warning(existing: &[String], name: &str) -> Option<String> {
    let lower = name.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }
    let matched = existing.iter().any(|c| {
        let c = c.to_lowercase();
        c.contains(&lower) || lower.contains(&c)
    });
    if matched {
        None
    } else {
        Some(format!("分类「{}」是新建的", name.trim()))
    }
}

/// Validate a proposed `create_account` path: the root must be an
/// asset/liability root, and the returned kind is derived from it.
fn validate_account_path(path: &str) -> Result<String, AppError> {
    let segments: Vec<&str> = path
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let first = segments
        .first()
        .ok_or_else(|| AppError::AiError("invalidAccountPath".to_string()))?;
    let kind = match first.to_lowercase().as_str() {
        "asset" | "assets" => "asset",
        "liability" | "liabilities" => "liability",
        _ => return Err(AppError::AiError("invalidAccountPath".to_string())),
    };
    Ok(kind.to_string())
}

/// The frontend confirmation card payload: everything the user must review
/// before anything is written. Resolved ids let the confirm step go through
/// the existing `quick_add_transaction` / `account_create` commands.
#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProposalDto {
    pub action: String,
    pub amount: Option<String>,
    pub date: String,
    pub description: String,
    pub source_account: Option<ResolvedAccountDto>,
    pub destination_account: Option<ResolvedAccountDto>,
    pub category_name: Option<String>,
    pub account_path: Option<String>,
    pub account_kind: Option<String>,
    pub opening_balance: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAccountDto {
    id: String,
    name: String,
}

impl From<ResolvedAccount> for ResolvedAccountDto {
    fn from(acc: ResolvedAccount) -> Self {
        ResolvedAccountDto {
            id: acc.id,
            name: acc.name,
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

/// Parse natural-language text into a proposal. **Never writes to the
/// database** — the user confirms (and may edit) the proposal in the frontend
/// confirmation card, then the write goes through the existing commands.
#[tauri::command]
pub async fn ai_parse_transaction(
    text: String,
    state: State<'_, AppState>,
) -> Result<AiProposalDto, String> {
    // Phase 1: Load config and context (sync, drop conn before await)
    let (config, context, default_source_id, all_accounts) = {
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or(crate::errors::message("errors.databaseLocked"))?;
        let conn = db.get_conn().map_err(|e| e.to_string())?;

        let config = load_ai_config(&conn).map_err(|e| e.to_string())?;
        let context = build_ai_context(&conn).map_err(|e| e.to_string())?;
        let default_source_id = settings::get_setting(&conn, "ai.default_source_account_id")
            .map_err(|e| e.to_string())?;
        let all_accounts = accounts::list_accounts(&conn).map_err(|e| e.to_string())?;
        (config, context, default_source_id, all_accounts)
    };

    // Phase 2: Call AI (async, no DB lock held)
    let proposal = ai::parse_transaction(&config, &text, &context)
        .await
        .map_err(|e| e.to_string())?;

    // Phase 3: Validate and resolve names into a reviewable proposal.
    ai::validate_action(&proposal.action).map_err(|e| e.to_string())?;
    let date = ai::validate_date(&proposal.date).map_err(|e| e.to_string())?;

    let mut warnings: Vec<String> = Vec::new();

    let dto = match proposal.action.as_str() {
        "expense" | "income" => {
            let amount = proposal
                .amount
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::AiError("invalidAmount".to_string()).to_string())?;
            ai::validate_amount(amount).map_err(|e| e.to_string())?;

            let (account, account_warnings, is_source) = match proposal.action.as_str() {
                "expense" => {
                    let (acc, warns) = resolve_named_or_default(
                        &all_accounts,
                        &proposal.source_account,
                        &default_source_id,
                        "noSourceAccount",
                    )
                    .map_err(|e| e.to_string())?;
                    (acc, warns, true)
                }
                _ => {
                    let (acc, warns) = resolve_named_or_default(
                        &all_accounts,
                        &proposal.destination_account,
                        &default_source_id,
                        "noDestinationAccount",
                    )
                    .map_err(|e| e.to_string())?;
                    (acc, warns, false)
                }
            };
            warnings.extend(account_warnings);

            if let Some(name) = proposal
                .category_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                let existing = if proposal.action == "expense" {
                    &context.expense_categories
                } else {
                    &context.income_categories
                };
                warnings.extend(category_warning(existing, name));
            }

            let source = is_source.then(|| account.clone().into());
            let destination = (!is_source).then(|| account.into());
            AiProposalDto {
                action: proposal.action,
                amount: Some(amount.to_string()),
                date,
                description: proposal.description,
                source_account: source,
                destination_account: destination,
                category_name: proposal.category_name,
                account_path: None,
                account_kind: None,
                opening_balance: None,
                warnings,
            }
        }

        "transfer" => {
            let amount = proposal
                .amount
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::AiError("invalidAmount".to_string()).to_string())?;
            ai::validate_amount(amount).map_err(|e| e.to_string())?;

            let (source, mut source_warnings) =
                resolve_required_named(&all_accounts, &proposal.source_account, "noSourceAccount")
                    .map_err(|e| e.to_string())?;
            let (destination, mut dest_warnings) = resolve_required_named(
                &all_accounts,
                &proposal.destination_account,
                "noDestinationAccount",
            )
            .map_err(|e| e.to_string())?;
            if source.id == destination.id {
                return Err(crate::errors::message("errors.transaction.transferToSelf"));
            }
            warnings.append(&mut source_warnings);
            warnings.append(&mut dest_warnings);

            AiProposalDto {
                action: proposal.action,
                amount: Some(amount.to_string()),
                date,
                description: proposal.description,
                source_account: Some(source.into()),
                destination_account: Some(destination.into()),
                category_name: None,
                account_path: None,
                account_kind: None,
                opening_balance: None,
                warnings,
            }
        }

        _ => {
            // create_account: path validated, duplicates warned, kind derived.
            let path = proposal
                .account_path
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| AppError::AiError("invalidAccountPath".to_string()).to_string())?;
            let kind_from_path = validate_account_path(path).map_err(|e| e.to_string())?;

            if let Some(balance) = proposal
                .opening_balance
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                ai::validate_amount(balance).map_err(|e| e.to_string())?;
            }

            // Warn when the stated kind contradicts the path root.
            if let Some(stated) = proposal.account_kind.as_deref() {
                if !stated.eq_ignore_ascii_case(&kind_from_path) {
                    warnings.push(format!(
                        "AI 提议的类型「{stated}」与路径根不一致，按「{kind_from_path}」处理"
                    ));
                }
            }

            // Duplicate leaf-name check against existing accounts.
            let leaf = path.rsplit('/').next().unwrap_or(path).trim();
            if let Some(existing) = all_accounts
                .iter()
                .find(|a| a.name.eq_ignore_ascii_case(leaf))
            {
                warnings.push(format!("已存在同名账户「{}」", existing.name));
            }

            AiProposalDto {
                action: proposal.action,
                amount: None,
                date,
                description: proposal.description,
                source_account: None,
                destination_account: None,
                category_name: None,
                account_path: Some(path.to_string()),
                account_kind: Some(kind_from_path),
                opening_balance: proposal.opening_balance,
                warnings,
            }
        }
    };

    Ok(dto)
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> Result<(), String> {
    let config = {
        let db_state = state.database.lock().map_err(|e| e.to_string())?;
        let db = db_state
            .as_ref()
            .ok_or(crate::errors::message("errors.databaseLocked"))?;
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
    use crate::db::categories::create_category;
    use crate::db::{transactions, Database, RecentDbs};
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

    /// The proposal JSON the mock model returns, with overridable fields.
    fn proposal_json(fields: serde_json::Value) -> String {
        let base = serde_json::json!({
            "action": "expense",
            "amount": "12.50",
            "categoryName": null,
            "sourceAccount": null,
            "destinationAccount": null,
            "accountPath": null,
            "accountKind": null,
            "openingBalance": null,
            "date": "2026-01-15",
            "description": "lunch",
        });
        let mut merged = base;
        if let (Some(dst), Some(src)) = (merged.as_object_mut(), fields.as_object()) {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        }
        merged.to_string()
    }

    fn make_account(id: &str, name: &str, is_active: bool) -> accounts::AccountRecord {
        accounts::AccountRecord {
            id: id.into(),
            name: name.into(),
            account_type: "asset".into(),
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

    // -----------------------------------------------------------------------
    // Helper unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_named_exact_no_warning() {
        let accs = vec![make_account("id-cash", "Cash", true)];
        let (acc, warns) =
            resolve_named_or_default(&accs, &Some("cash".into()), &None, "noSourceAccount")
                .unwrap();
        assert_eq!(acc.id, "id-cash");
        assert!(warns.is_empty());
    }

    #[test]
    fn test_resolve_named_fuzzy_records_warning() {
        let accs = vec![make_account("id-w", "WeChat Wallet", true)];
        let (acc, warns) =
            resolve_named_or_default(&accs, &Some("wallet".into()), &None, "noSourceAccount")
                .unwrap();
        assert_eq!(acc.id, "id-w");
        assert_eq!(
            warns,
            vec!["账户「wallet」按名称匹配到「WeChat Wallet」".to_string()]
        );
    }

    #[test]
    fn test_resolve_named_falls_back_to_default_with_warning() {
        let accs = vec![make_account("id-cash", "Cash", true)];
        let (acc, warns) =
            resolve_named_or_default(&accs, &None, &Some("id-cash".into()), "noSourceAccount")
                .unwrap();
        assert_eq!(acc.id, "id-cash");
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("默认账户"));
    }

    #[test]
    fn test_resolve_named_no_name_no_default_errors() {
        let accs = vec![make_account("id-cash", "Cash", true)];
        let err = resolve_named_or_default(&accs, &None, &None, "noSourceAccount").unwrap_err();
        assert!(matches!(err, AppError::AiError(code) if code == "noSourceAccount"));
    }

    #[test]
    fn test_resolve_named_inactive_default_rejected() {
        let accs = vec![make_account("id-off", "Old", false)];
        assert!(
            resolve_named_or_default(&accs, &None, &Some("id-off".into()), "noSourceAccount")
                .is_err()
        );
    }

    #[test]
    fn test_resolve_required_named_missing_name_errors() {
        let accs = vec![make_account("id-cash", "Cash", true)];
        let err = resolve_required_named(&accs, &None, "noDestinationAccount").unwrap_err();
        assert!(matches!(err, AppError::AiError(code) if code == "noDestinationAccount"));
    }

    #[test]
    fn test_resolve_required_named_resolves_exact() {
        let accs = vec![make_account("id-cash", "Cash", true)];
        let (acc, warns) =
            resolve_required_named(&accs, &Some("Cash".into()), "noSourceAccount").unwrap();
        assert_eq!(acc.id, "id-cash");
        assert!(warns.is_empty());
    }

    #[test]
    fn test_category_warning_new_and_existing() {
        let existing = vec!["餐饮美食".to_string()];
        assert!(category_warning(&existing, "餐饮美食").is_none());
        assert!(category_warning(&existing, "餐饮").is_none());
        let warn = category_warning(&existing, "咖啡").unwrap();
        assert!(warn.contains("咖啡") && warn.contains("新建"));
        assert!(category_warning(&existing, "  ").is_none());
    }

    #[test]
    fn test_validate_account_path_kinds() {
        assert_eq!(validate_account_path("Assets/支付宝").unwrap(), "asset");
        assert_eq!(validate_account_path("assets/cash").unwrap(), "asset");
        assert_eq!(
            validate_account_path("Liabilities/信用卡").unwrap(),
            "liability"
        );
        assert!(validate_account_path("Expense/餐饮").is_err());
        assert!(validate_account_path("").is_err());
        assert!(validate_account_path("支付宝").is_err());
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
    // Command-level tests via IPC — proposals only, nothing is written
    // -----------------------------------------------------------------------

    #[test]
    fn test_ai_parse_expense_proposes_without_writing() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        let wechat_id = {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
            create_asset_account(&conn, "Assets/WeChat")
        };
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "expense",
                "categoryName": "餐饮美食",
                "sourceAccount": "WeChat",
            }))),
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
        assert_eq!(resp["action"], "expense");
        assert_eq!(resp["amount"], "12.50");
        assert_eq!(resp["sourceAccount"]["id"], wechat_id.as_str());
        assert_eq!(resp["sourceAccount"]["name"], "WeChat");
        assert_eq!(resp["categoryName"], "餐饮美食");
        // The db has no categories yet, so the proposed category is new —
        // that guess must be surfaced as a warning.
        let warnings = resp["warnings"].as_array().unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].as_str().unwrap().contains("新建"));

        // Proposal only: no transaction, no auto-created category.
        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let conn = db_state.as_ref().unwrap().get_conn().unwrap();
        let txs = transactions::list_transactions(&conn, None, None, None).unwrap();
        assert!(txs.is_empty(), "proposal must not write");
        let cats = crate::db::categories::list_categories(&conn).unwrap();
        assert!(cats.is_empty(), "proposal must not create categories");
    }

    #[test]
    fn test_ai_parse_income_defaults_destination_with_warning() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        let wechat_id = {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/Cash");
            let wechat = create_asset_account(&conn, "Assets/WeChat");
            settings::set_setting(&conn, "ai.default_source_account_id", &wechat).unwrap();
            wechat
        };
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "income",
                "categoryName": "Salary",
            }))),
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
        assert_eq!(resp["action"], "income");
        assert_eq!(resp["destinationAccount"]["id"], wechat_id.as_str());
        assert_eq!(resp["sourceAccount"], serde_json::Value::Null);
        let warnings = resp["warnings"].as_array().unwrap();
        assert!(warnings
            .iter()
            .any(|w| w.as_str().unwrap().contains("默认账户")));

        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let conn = db_state.as_ref().unwrap().get_conn().unwrap();
        assert!(transactions::list_transactions(&conn, None, None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_ai_parse_transfer_proposes_both_ends() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        let (alipay_id, wechat_id) = {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            let a = create_asset_account(&conn, "Assets/支付宝");
            let w = create_asset_account(&conn, "Assets/WeChat");
            (a, w)
        };
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "transfer",
                "amount": "500.00",
                "sourceAccount": "支付宝",
                "destinationAccount": "WeChat",
                "description": "支付宝转微信",
            }))),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let resp = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "支付宝转微信500" }),
        )
        .unwrap();
        assert_eq!(resp["action"], "transfer");
        assert_eq!(resp["amount"], "500.00");
        assert_eq!(resp["sourceAccount"]["id"], alipay_id.as_str());
        assert_eq!(resp["sourceAccount"]["name"], "支付宝");
        assert_eq!(resp["destinationAccount"]["id"], wechat_id.as_str());

        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let conn = db_state.as_ref().unwrap().get_conn().unwrap();
        assert!(transactions::list_transactions(&conn, None, None, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn test_ai_parse_transfer_missing_end_errors() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/支付宝");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "transfer",
                "sourceAccount": "支付宝",
                "destinationAccount": null,
            }))),
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
        assert!(err.as_str().unwrap().contains("无法确定目标账户"));
    }

    #[test]
    fn test_ai_parse_create_account_proposal() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
            create_asset_account(&conn, "Assets/WeChat");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "create_account",
                "amount": null,
                "accountPath": "Assets/支付宝",
                "accountKind": "asset",
                "openingBalance": "1000.00",
                "description": "新建支付宝账户",
            }))),
        )]);
        {
            let conn = sdb.get_conn().unwrap();
            settings::set_setting(&conn, "ai.base_url", &base_url).unwrap();
        }

        let (_app, webview) = mock_app(Some(sdb));
        let resp = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "新建支付宝账户，余额1000" }),
        )
        .unwrap();
        assert_eq!(resp["action"], "create_account");
        assert_eq!(resp["accountPath"], "Assets/支付宝");
        assert_eq!(resp["accountKind"], "asset");
        assert_eq!(resp["openingBalance"], "1000.00");
        assert_eq!(resp["amount"], serde_json::Value::Null);

        let state: State<AppState> = _app.state();
        let db_state = state.database.lock().unwrap();
        let conn = db_state.as_ref().unwrap().get_conn().unwrap();
        // Only the seeded WeChat account exists; the proposal added nothing.
        assert_eq!(accounts::list_accounts(&conn).unwrap().len(), 1);
    }

    #[test]
    fn test_ai_parse_create_account_invalid_path_rejected() {
        ensure_no_proxy();
        let (_sdir, sdb) = test_db();
        {
            let conn = sdb.get_conn().unwrap();
            seed_ai_settings(&conn, "PLACEHOLDER");
        }
        let base_url = spawn_mock_server(vec![(
            200,
            "OK",
            chat_body(&proposal_json(serde_json::json!({
                "action": "create_account",
                "accountPath": "支付宝",
            }))),
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
        assert!(err.as_str().unwrap().contains("Assets"));
    }

    #[test]
    fn test_ai_parse_invalid_action_from_ai() {
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
            chat_body(&proposal_json(
                serde_json::json!({ "action": "delete_everything" }),
            )),
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
        assert!(err.as_str().unwrap().contains("操作类型无效"));
    }

    #[test]
    fn test_ai_parse_invalid_amount_from_ai() {
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
            chat_body(&proposal_json(serde_json::json!({ "amount": "-1" }))),
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
        assert!(err.as_str().unwrap().contains("金额无效"));
    }

    #[test]
    fn test_ai_parse_missing_amount_from_ai() {
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
            chat_body(&proposal_json(serde_json::json!({ "amount": null }))),
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
        assert!(err.as_str().unwrap().contains("金额无效"));
    }

    #[test]
    fn test_ai_parse_missing_date_from_ai() {
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
            chat_body(&proposal_json(serde_json::json!({ "date": null }))),
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
        assert!(err.as_str().unwrap().contains("AI 未返回日期"));
    }

    #[test]
    fn test_ai_parse_locked_db_fails() {
        let (_app, webview) = mock_app(None);
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("数据库未解锁"));
    }

    #[test]
    fn test_ai_parse_missing_config_fails() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app(Some(db));
        let err = invoke(
            &webview,
            "ai_parse_transaction",
            serde_json::json!({ "text": "x" }),
        )
        .unwrap_err();
        assert!(err.as_str().unwrap().contains("未配置"));
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
        assert!(err.as_str().unwrap().contains("调用失败"));
    }
}
