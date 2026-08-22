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
