use serde::Deserialize;
use std::collections::HashMap;
use tauri::State;

use crate::db::accounts;
use crate::db::AppError;
use crate::AppState;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Parses a decimal string amount to cents (i64).
fn parse_decimal_to_cents(s: &str) -> Result<i64, AppError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }

    let (sign, num_str) = if let Some(rest) = trimmed.strip_prefix('-') {
        (-1, rest.trim())
    } else if let Some(rest) = trimmed.strip_prefix('+') {
        (1, rest.trim())
    } else {
        (1, trimmed)
    };

    let parts: Vec<&str> = num_str.split('.').collect();
    if parts.len() > 2 {
        return Err(AppError::ValidationError(
            "Invalid amount format: multiple decimal points".into(),
        ));
    }

    let integer_part: i64 = if parts.is_empty() || parts[0].is_empty() {
        0
    } else {
        parts[0]
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid amount format".into()))?
    };

    let fractional_part: i64 = if parts.len() > 1 {
        let frac = parts[1];
        if frac.len() > 2 {
            return Err(AppError::ValidationError(
                "Amount has more than 2 decimal places".into(),
            ));
        }
        let frac_str = if frac.len() == 1 {
            format!("{}0", frac)
        } else {
            frac.to_string()
        };
        frac_str
            .parse()
            .map_err(|_| AppError::ValidationError("Invalid amount format".into()))?
    } else {
        0
    };

    let total = integer_part * 100 + fractional_part;
    Ok(sign * total)
}

// ---------------------------------------------------------------------------
// Command input types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateAccountInput {
    pub name: String,
    pub currency: Option<String>,
    pub initial_balance: Option<String>,
    pub equity_account_name: Option<String>,
    pub opening_balance_name: Option<String>,
    pub description: Option<String>,
    pub account_number: Option<String>,
    pub initial_balance_date: Option<String>,
    pub iban: Option<String>,
    pub is_active: Option<bool>,
    pub include_net_worth: Option<bool>,
    pub meta: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Debug)]
pub struct UpdateAccountInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub account_number: Option<String>,
    pub initial_balance: Option<String>,
    pub initial_balance_date: Option<String>,
    pub iban: Option<String>,
    pub is_active: Option<bool>,
    pub include_net_worth: Option<bool>,
    pub meta: Option<HashMap<String, String>>,
}

// ---------------------------------------------------------------------------
// DTO types (serializable for Tauri IPC)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
pub struct AccountDto {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub currency: String,
    pub description: String,
    pub account_number: Option<String>,
    pub is_system: bool,
    pub iban: Option<String>,
    pub is_active: bool,
    pub include_net_worth: bool,
    pub created_at: String,
    pub updated_at: String,
    pub meta: Vec<accounts::AccountMetaRecord>,
    pub initial_balance: Option<i64>,
    pub initial_balance_date: Option<String>,
}

#[derive(serde::Serialize)]
pub struct AccountMetaSchema {
    pub valid_account_roles: Vec<String>,
    pub valid_liability_types: Vec<String>,
}

#[derive(serde::Serialize)]
pub struct AccountBalanceItem {
    pub id: String,
    pub balance: i64,
}

fn to_dto_with_opening_balance(
    record: accounts::AccountRecord,
    conn: &rusqlite::Connection,
) -> Result<AccountDto, AppError> {
    let meta = accounts::get_meta_for_account(conn, &record.id)?;
    let (initial_balance, initial_balance_date) =
        accounts::get_opening_balance_for_account(conn, &record.id)?
            .map(|(amt, date)| (Some(amt), Some(date)))
            .unwrap_or((None, None));

    Ok(AccountDto {
        id: record.id,
        name: record.name,
        account_type: record.account_type,
        currency: record.currency,
        description: record.description,
        account_number: record.account_number,
        is_system: record.is_system,
        iban: record.iban,
        is_active: record.is_active,
        include_net_worth: record.include_net_worth,
        created_at: record.created_at,
        updated_at: record.updated_at,
        meta,
        initial_balance,
        initial_balance_date,
    })
}

// ---------------------------------------------------------------------------
// Meta Validation
// ---------------------------------------------------------------------------

const VALID_ACCOUNT_ROLES: &[&str] = &[
    "defaultAsset",
    "sharedAsset",
    "savingAsset",
    "ccAsset",
    "cashWalletAsset",
    "receivableAsset",
];

const VALID_LIABILITY_TYPES: &[&str] = &["loan", "debt", "mortgage"];

fn validate_account_meta(
    account_type: &str,
    meta: &HashMap<String, String>,
) -> Result<(), AppError> {
    if account_type == "asset" {
        if let Some(role) = meta.get("account_role") {
            if !VALID_ACCOUNT_ROLES.contains(&role.as_str()) {
                return Err(AppError::ValidationError(format!(
                    "Invalid account_role: '{}'. Valid values: {}",
                    role,
                    VALID_ACCOUNT_ROLES.join(", ")
                )));
            }
        }
    } else if account_type == "liability" {
        if let Some(liability_type) = meta.get("liability_type") {
            if !VALID_LIABILITY_TYPES.contains(&liability_type.as_str()) {
                return Err(AppError::ValidationError(format!(
                    "Invalid liability_type: '{}'. Valid values: {}",
                    liability_type,
                    VALID_LIABILITY_TYPES.join(", ")
                )));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn account_create(
    input: CreateAccountInput,
    state: State<'_, AppState>,
) -> Result<AccountDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    // Start transaction for atomicity
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let currency = input.currency.unwrap_or_else(|| "CNY".to_string());

    // Reject equity account creation (matches Firefly III behavior)
    let segments: Vec<&str> = input
        .name
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(first) = segments.first() {
        let lower = first.to_lowercase();
        if lower == "equity" || lower == "equities" {
            return Err("errors.accountSystemManaged".to_string());
        }
    }

    let full_input = accounts::CreateAccountFullInput {
        description: input.description,
        account_number: input.account_number,
        iban: input.iban,
        is_active: input.is_active.unwrap_or(true),
        include_net_worth: input.include_net_worth.unwrap_or(true),
    };

    let id = accounts::create_account_with_path(&tx, &input.name, &currency, Some(&full_input))
        .map_err(|e| e.to_string())?;

    let created_account = accounts::find_account(&tx, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Created account not found".to_string())?;

    if let Some(ref meta) = &input.meta {
        validate_account_meta(&created_account.account_type, meta).map_err(|e| e.to_string())?;
    }

    if let Some(balance_str) = &input.initial_balance {
        let balance = parse_decimal_to_cents(balance_str).map_err(|e| e.to_string())?;
        if balance != 0 {
            let equity_name = input.equity_account_name.as_deref().unwrap_or("Equity");
            let opening_name = input
                .opening_balance_name
                .as_deref()
                .unwrap_or("Opening Balances");
            accounts::create_opening_balance(
                &tx,
                &id,
                balance,
                equity_name,
                opening_name,
                input.initial_balance_date.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Insert meta if provided
    if let Some(ref meta) = input.meta {
        let metas_vec: Vec<(String, String)> =
            meta.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        accounts::set_meta_batch(&tx, &id, &metas_vec).map_err(|e| e.to_string())?;
    }

    // Commit transaction
    tx.commit().map_err(|e| e.to_string())?;

    // Read result (outside transaction, no need for atomicity)
    let record = accounts::find_account(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Created account not found".to_string())?;

    let meta = accounts::get_meta_for_account(&conn, &id).map_err(|e| e.to_string())?;

    let (initial_balance, initial_balance_date) =
        accounts::get_opening_balance_for_account(&conn, &id)
            .map_err(|e| e.to_string())?
            .map(|(amt, date)| (Some(amt), Some(date)))
            .unwrap_or((None, None));

    Ok(AccountDto {
        id: record.id,
        name: record.name,
        account_type: record.account_type,
        currency: record.currency,
        description: record.description,
        account_number: record.account_number,
        is_system: record.is_system,
        iban: record.iban,
        is_active: record.is_active,
        include_net_worth: record.include_net_worth,
        created_at: record.created_at,
        updated_at: record.updated_at,
        meta,
        initial_balance,
        initial_balance_date,
    })
}

#[tauri::command]
pub async fn account_list(state: State<'_, AppState>) -> Result<Vec<AccountDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let accounts_list = accounts::list_accounts(&conn).map_err(|e| e.to_string())?;
    accounts_list
        .into_iter()
        .map(|record| to_dto_with_opening_balance(record, &conn).map_err(|e| e.to_string()))
        .collect()
}

#[tauri::command]
pub async fn account_update(
    id: String,
    input: UpdateAccountInput,
    state: State<'_, AppState>,
) -> Result<AccountDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    // Start transaction for atomicity
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    accounts::update_account(
        &tx,
        &accounts::UpdateAccountParams {
            id: &id,
            name: input.name.as_deref(),
            description: input.description.as_deref(),
            account_number: input.account_number.as_ref().map(|s| Some(s.as_str())),
            iban: input.iban.as_ref().map(|s| Some(s.as_str())),
            is_active: input.is_active,
            include_net_worth: input.include_net_worth,
        },
    )
    .map_err(|e| e.to_string())?;

    let existing_account = accounts::find_account(&tx, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", id))?;

    if let Some(ref meta) = &input.meta {
        if !meta.is_empty() {
            validate_account_meta(&existing_account.account_type, meta)
                .map_err(|e| e.to_string())?;
        }
    }

    if input.meta.as_ref().is_some_and(|m| !m.is_empty()) {
        accounts::delete_meta_for_account(&tx, &id).map_err(|e| e.to_string())?;
        let metas_vec: Vec<(String, String)> = input
            .meta
            .as_ref()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        accounts::set_meta_batch(&tx, &id, &metas_vec).map_err(|e| e.to_string())?;
    }

    // Update opening balance if provided
    if let Some(balance_str) = &input.initial_balance {
        let record = accounts::find_account(&tx, &id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Account not found: {}", id))?;

        if record.account_type == "asset" || record.account_type == "liability" {
            let balance = parse_decimal_to_cents(balance_str).map_err(|e| e.to_string())?;
            accounts::create_opening_balance(
                &tx,
                &id,
                balance,
                "Equity",
                "Opening Balances",
                input.initial_balance_date.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Commit transaction
    tx.commit().map_err(|e| e.to_string())?;

    // Read result (outside transaction)
    let record = accounts::find_account(&conn, &id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Account not found: {}", id))?;

    let meta = accounts::get_meta_for_account(&conn, &id).map_err(|e| e.to_string())?;

    let (initial_balance, initial_balance_date) =
        accounts::get_opening_balance_for_account(&conn, &id)
            .map_err(|e| e.to_string())?
            .map(|(amt, date)| (Some(amt), Some(date)))
            .unwrap_or((None, None));

    Ok(AccountDto {
        id: record.id,
        name: record.name,
        account_type: record.account_type,
        currency: record.currency,
        description: record.description,
        account_number: record.account_number,
        is_system: record.is_system,
        iban: record.iban,
        is_active: record.is_active,
        include_net_worth: record.include_net_worth,
        created_at: record.created_at,
        updated_at: record.updated_at,
        meta,
        initial_balance,
        initial_balance_date,
    })
}

#[tauri::command]
pub async fn account_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    // Start transaction for atomicity
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    accounts::delete_account(&tx, &id).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn account_balance(id: String, state: State<'_, AppState>) -> Result<i64, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    accounts::get_account_balance(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_balances_batch(
    ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Vec<AccountBalanceItem>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let balances = accounts::get_account_balances_batch(&conn, &ids).map_err(|e| e.to_string())?;

    Ok(balances
        .into_iter()
        .map(|(id, balance)| AccountBalanceItem { id, balance })
        .collect())
}

#[tauri::command]
pub async fn account_transfer(
    from_id: String,
    to_id: String,
    amount: i64,
    description: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    // Start transaction for atomicity (double-entry integrity)
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let tx_id = accounts::create_transfer(&tx, &from_id, &to_id, amount, &description)
        .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    Ok(tx_id)
}

#[tauri::command]
pub async fn account_batch_create(
    inputs: Vec<CreateAccountInput>,
    state: State<'_, AppState>,
) -> Result<Vec<AccountDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    // Start transaction for atomicity
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    // Convert inputs to batch format - parse decimal strings to cents
    let mut batch_inputs: Vec<accounts::BatchAccountInput> = Vec::new();
    for input in inputs {
        let initial_balance = match &input.initial_balance {
            Some(s) => Some(parse_decimal_to_cents(s).map_err(|e| e.to_string())?),
            None => None,
        };
        batch_inputs.push(accounts::BatchAccountInput {
            name: input.name,
            currency: input.currency.unwrap_or_else(|| "CNY".to_string()),
            initial_balance,
            description: input.description,
            account_number: input.account_number,
        });
    }

    let records = accounts::batch_create_accounts(&tx, batch_inputs).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    // Convert to DTOs (outside transaction)
    records
        .into_iter()
        .map(|record| to_dto_with_opening_balance(record, &conn).map_err(|e| e.to_string()))
        .collect()
}

#[tauri::command]
pub async fn account_transactions(
    id: String,
    from_date: Option<String>,
    to_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<accounts::TransactionRecord>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    accounts::get_account_transactions(&conn, &id, from_date.as_deref(), to_date.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_meta_get(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<accounts::AccountMetaRecord>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    accounts::get_meta_for_account(&conn, &account_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn account_meta_set(
    account_id: String,
    key: String,
    value: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    accounts::insert_meta(&conn, &account_id, &key, &value).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn account_meta_batch_set(
    account_id: String,
    metas: Vec<(String, String)>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    accounts::set_meta_batch(&conn, &account_id, &metas).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn account_meta_schema() -> Result<AccountMetaSchema, String> {
    Ok(AccountMetaSchema {
        valid_account_roles: VALID_ACCOUNT_ROLES.iter().map(|s| s.to_string()).collect(),
        valid_liability_types: VALID_LIABILITY_TYPES
            .iter()
            .map(|s| s.to_string())
            .collect(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts::{insert_account, AccountRecord};
    use rusqlite::Connection;
    use tempfile::TempDir;

    fn test_conn() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE accounts (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                type                TEXT NOT NULL CHECK (type IN ('asset', 'liability', 'income', 'expense', 'equity')),
                currency            TEXT NOT NULL DEFAULT 'CNY',
                description         TEXT NOT NULL DEFAULT '',
                account_number      TEXT,
                is_system           INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
                iban                TEXT,
                is_active           INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
                include_net_worth   INTEGER NOT NULL DEFAULT 1 CHECK (include_net_worth IN (0, 1)),
                created_at          TEXT NOT NULL,
                updated_at          TEXT NOT NULL
            );

            CREATE TABLE account_meta (
                id          TEXT PRIMARY KEY,
                account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                key         TEXT NOT NULL,
                value       TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE postings (
                id             TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                account_id     TEXT NOT NULL REFERENCES accounts(id),
                amount         INTEGER NOT NULL,
                created_at     TEXT NOT NULL
            );

            CREATE UNIQUE INDEX idx_accounts_type_name
                ON accounts(type, name);

            CREATE UNIQUE INDEX idx_account_meta_account_key
                ON account_meta(account_id, key);
        "#,
        )
        .unwrap();
        (dir, conn)
    }

    fn make_account_record(name: &str, account_type: &str) -> AccountRecord {
        AccountRecord {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            account_type: account_type.to_string(),
            currency: "CNY".to_string(),
            description: String::new(),
            account_number: None,
            is_system: false,
            iban: None,
            is_active: true,
            include_net_worth: true,
            created_at: crate::utils::time::now_rfc3339(),
            updated_at: crate::utils::time::now_rfc3339(),
        }
    }

    #[test]
    fn test_to_dto_returns_meta() {
        let (_dir, conn) = test_conn();

        let acct = make_account_record("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        accounts::set_meta_batch(
            &conn,
            &account_id,
            &[
                ("account_role".to_string(), "ccAsset".to_string()),
                ("notes".to_string(), "test notes".to_string()),
            ],
        )
        .unwrap();

        let record = accounts::find_account(&conn, &account_id)
            .unwrap()
            .expect("Account should exist");

        let dto = to_dto_with_opening_balance(record, &conn).unwrap();

        assert!(!dto.meta.is_empty(), "meta should not be empty");

        let meta_map: HashMap<&str, &str> = dto
            .meta
            .iter()
            .map(|m| (m.key.as_str(), m.value.as_str()))
            .collect();

        assert_eq!(
            meta_map.get("account_role"),
            Some(&"ccAsset"),
            "account_role meta should be present"
        );
        assert_eq!(
            meta_map.get("notes"),
            Some(&"test notes"),
            "notes meta should be present"
        );
    }

    #[test]
    fn test_empty_meta_map_should_not_delete_existing_meta() {
        let (_dir, conn) = test_conn();

        let acct = make_account_record("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        accounts::set_meta_batch(
            &conn,
            &account_id,
            &[("account_role".to_string(), "ccAsset".to_string())],
        )
        .unwrap();

        let existing_meta = accounts::get_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(existing_meta.len(), 1);

        let empty_meta: Option<HashMap<String, String>> = Some(HashMap::new());

        if empty_meta.as_ref().is_some_and(|m| !m.is_empty()) {
            accounts::delete_meta_for_account(&conn, &account_id).unwrap();
            let metas_vec: Vec<(String, String)> = empty_meta
                .as_ref()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            accounts::set_meta_batch(&conn, &account_id, &metas_vec).unwrap();
        }

        let meta_after = accounts::get_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(
            meta_after.len(),
            1,
            "empty HashMap should not delete existing meta"
        );
    }

    #[test]
    fn test_validate_meta_account_role_invalid() {
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("account_role".to_string(), "invalidRole".to_string());

        let result = validate_account_meta("asset", &meta);
        assert!(result.is_err(), "invalid account_role should return error");
    }

    #[test]
    fn test_validate_meta_account_role_valid() {
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("account_role".to_string(), "ccAsset".to_string());

        let result = validate_account_meta("asset", &meta);
        assert!(result.is_ok(), "valid account_role should pass");
    }

    #[test]
    fn test_validate_meta_liability_type_invalid() {
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("liability_type".to_string(), "invalidType".to_string());

        let result = validate_account_meta("liability", &meta);
        assert!(
            result.is_err(),
            "invalid liability_type should return error"
        );
    }

    #[test]
    fn test_validate_meta_liability_type_valid() {
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("liability_type".to_string(), "mortgage".to_string());

        let result = validate_account_meta("liability", &meta);
        assert!(result.is_ok(), "valid liability_type should pass");
    }

    #[test]
    fn test_validate_meta_wrong_key_for_type() {
        let mut meta: HashMap<String, String> = HashMap::new();
        meta.insert("liability_type".to_string(), "loan".to_string());

        let result = validate_account_meta("asset", &meta);
        assert!(
            result.is_ok(),
            "liability_type on asset account should be ignored"
        );
    }

    #[test]
    fn test_parse_decimal_to_cents_large_amount() {
        let result = parse_decimal_to_cents("999999999.99");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 99_999_999_999);
    }

    #[test]
    fn test_parse_decimal_to_cents_fractional() {
        let result = parse_decimal_to_cents("0.01");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_parse_decimal_to_cents_negative() {
        let result = parse_decimal_to_cents("-123.45");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), -12_345);
    }

    #[test]
    fn test_parse_decimal_to_cents_zero() {
        let result = parse_decimal_to_cents("0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_parse_decimal_to_cents_zero_decimal() {
        let result = parse_decimal_to_cents("0.0");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn test_parse_decimal_to_cents_empty_string() {
        assert_eq!(parse_decimal_to_cents("").unwrap(), 0);
        assert_eq!(parse_decimal_to_cents("   ").unwrap(), 0);
    }

    #[test]
    fn test_parse_decimal_to_cents_dot_only() {
        let result = parse_decimal_to_cents(".5");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_parse_decimal_to_cents_positive_sign() {
        let result = parse_decimal_to_cents("+123.45");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 12_345);
    }

    #[test]
    fn test_parse_decimal_to_cents_invalid_string() {
        let result = parse_decimal_to_cents("abc");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_to_cents_multiple_dots() {
        let result = parse_decimal_to_cents("1..5");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_decimal_to_cents_many_dots() {
        let result = parse_decimal_to_cents("1.000.50");
        assert!(result.is_err());
    }

    #[test]
    fn test_account_meta_schema_returns_all_values() {
        let schema = AccountMetaSchema {
            valid_account_roles: VALID_ACCOUNT_ROLES.iter().map(|s| s.to_string()).collect(),
            valid_liability_types: VALID_LIABILITY_TYPES
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };

        assert_eq!(schema.valid_account_roles.len(), 6);
        assert!(schema
            .valid_account_roles
            .contains(&"defaultAsset".to_string()));
        assert!(schema
            .valid_account_roles
            .contains(&"cashWalletAsset".to_string()));
        assert_eq!(schema.valid_liability_types.len(), 3);
        assert!(schema.valid_liability_types.contains(&"loan".to_string()));
        assert!(schema
            .valid_liability_types
            .contains(&"mortgage".to_string()));
    }
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn mock_app_with_db(
        db: Database,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(Some(db)),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![
                account_create,
                account_list,
                account_update,
                account_delete,
                account_balance,
                account_balances_batch,
                account_transfer,
                account_batch_create,
                account_transactions,
                account_meta_set,
                account_meta_get,
                account_meta_batch_set,
                account_meta_schema,
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

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    fn create_account(
        webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        name: &str,
        initial_balance: Option<&str>,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({ "input": { "name": name } });
        if let Some(balance) = initial_balance {
            body["input"]["initial_balance"] = serde_json::json!(balance);
        }
        invoke(webview, "account_create", body).expect("account_create should succeed")
    }

    #[test]
    fn test_account_crud_and_balance_via_ipc() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        // create asset account with opening balance (decimal string)
        let created = create_account(&webview, "Assets/Cash", Some("1000.00"));
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["name"], serde_json::json!("Cash"));
        assert_eq!(created["account_type"], serde_json::json!("asset"));

        // opening balance is reflected in account_balance (cents)
        let balance: i64 = serde_json::from_value(
            invoke(
                &webview,
                "account_balance",
                serde_json::json!({ "id": id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(balance, 100000);

        // batch balance
        let balances: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_balances_batch",
                serde_json::json!({ "ids": [id] }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(balances.len(), 1);

        // meta set/get
        invoke(
            &webview,
            "account_meta_set",
            serde_json::json!({ "accountId": id, "key": "bank", "value": "ICBC" }),
        )
        .unwrap();
        let metas: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_meta_get",
                serde_json::json!({ "accountId": id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(metas.len(), 1);
        assert_eq!(metas[0]["value"], serde_json::json!("ICBC"));

        // update name
        let updated = invoke(
            &webview,
            "account_update",
            serde_json::json!({ "id": id, "input": { "name": "Assets/Wallet" } }),
        )
        .expect("update should succeed");
        assert_eq!(updated["name"], serde_json::json!("Assets/Wallet"));

        // transfer between two accounts (amount in cents)
        let to = create_account(&webview, "Assets/Bank", None);
        let to_id = to["id"].as_str().unwrap().to_string();
        let tx_id = invoke(
            &webview,
            "account_transfer",
            serde_json::json!({
                "fromId": id,
                "toId": to_id,
                "amount": 20000,
                "description": "ATM transfer"
            }),
        )
        .expect("transfer should succeed");
        assert!(!tx_id.as_str().unwrap().is_empty());

        let from_balance: i64 = serde_json::from_value(
            invoke(
                &webview,
                "account_balance",
                serde_json::json!({ "id": id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(from_balance, 80000);

        // list contains both accounts
        let list: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "account_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert_eq!(list.len(), 2);

        // batch create
        let batch: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_batch_create",
                serde_json::json!({
                    "inputs": [
                        { "name": "Expenses/Food", "initialBalance": "50.00" },
                        { "name": "Expenses/Transport", "initialBalance": "20.00" }
                    ]
                }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(batch.len(), 2);

        // delete account with no transactions fails
        let err = invoke(
            &webview,
            "account_delete",
            serde_json::json!({ "id": to_id }),
        )
        .expect_err("deleting account with transactions must fail");
        assert!(!err.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_account_create_rejects_equity_path() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Equity/Owner" } }),
        )
        .expect_err("equity account creation must be rejected");
        assert_eq!(err.as_str().unwrap(), "errors.accountSystemManaged");
    }

    #[test]
    fn test_account_delete_empty_account() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = create_account(&webview, "Assets/Empty", None);
        let id = created["id"].as_str().unwrap().to_string();

        invoke(
            &webview,
            "account_delete",
            serde_json::json!({ "id": id }),
        )
        .unwrap();

        let list: Vec<serde_json::Value> = serde_json::from_value(
            invoke(&webview, "account_list", serde_json::json!({})).unwrap(),
        )
        .unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn test_account_create_with_meta_and_initial_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = invoke(
            &webview,
            "account_create",
            serde_json::json!({
                "input": {
                    "name": "Assets/Savings",
                    "initial_balance": "1000.00",
                    "equity_account_name": "Equity",
                    "opening_balance_name": "Opening Balances",
                    "initial_balance_date": "2024-01-01",
                    "meta": { "account_role": "ccAsset" },
                    "isActive": true,
                    "include_net_worth": true
                }
            }),
        )
        .expect("account_create with meta and balance should succeed");
        assert_eq!(created["name"], serde_json::json!("Savings"));
        assert_eq!(created["initial_balance"], serde_json::json!(100000));
        assert_eq!(created["initial_balance_date"], serde_json::json!("2024-01-01"));
        assert_eq!(created["meta"].as_array().unwrap().len(), 1);
        assert_eq!(created["meta"][0]["key"], serde_json::json!("account_role"));

        // Balance includes the opening balance
        let balance: i64 = serde_json::from_value(
            invoke(
                &webview,
                "account_balance",
                serde_json::json!({ "id": created["id"] }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(balance, 100000);
    }

    #[test]
    fn test_account_update_with_meta_and_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Assets/Card" } }),
        )
        .expect("account_create should succeed");
        let id = created["id"].as_str().unwrap().to_string();

        let updated = invoke(
            &webview,
            "account_update",
            serde_json::json!({
                "id": id,
                "input": {
                    "name": "Assets/Card2",
                    "initial_balance": "500.00",
                    "meta": { "account_role": "ccAsset" }
                }
            }),
        )
        .expect("account_update with meta and balance should succeed");
        assert_eq!(updated["name"], serde_json::json!("Assets/Card2"));
        assert_eq!(updated["initial_balance"], serde_json::json!(50000));
    }

    #[test]
    fn test_account_batch_create_without_initial_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let batch: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_batch_create",
                serde_json::json!({
                    "inputs": [
                        { "name": "Expenses/Books" },
                        { "name": "Expenses/Games" }
                    ]
                }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0]["name"], serde_json::json!("Books"));
    }

    #[test]
    fn test_parse_decimal_to_cents_too_many_decimals() {
        let result = parse_decimal_to_cents("1.234");
        assert!(result.is_err());
    }

    #[test]
    fn test_account_create_name_with_only_equity_prefix() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let err = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Equities/X" } }),
        )
        .expect_err("equities prefix must be rejected");
        assert_eq!(err.as_str().unwrap(), "errors.accountSystemManaged");
    }

    #[test]
    fn test_account_create_zero_initial_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Assets/Zero", "initial_balance": "0.00" } }),
        )
        .expect("zero initial balance should succeed");
        assert_eq!(created["initial_balance"], serde_json::json!(null));
    }

    #[test]
    fn test_account_update_empty_meta_map() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = create_account(&webview, "Assets/WithMeta", None);
        let id = created["id"].as_str().unwrap().to_string();

        let updated = invoke(
            &webview,
            "account_update",
            serde_json::json!({ "id": id, "input": { "meta": {} } }),
        )
        .expect("update with empty meta should succeed");
        assert_eq!(updated["name"], serde_json::json!("WithMeta"));
    }

    #[test]
    fn test_account_update_income_with_initial_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let created = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Income/Salary" } }),
        )
        .expect("create income account should succeed");
        let id = created["id"].as_str().unwrap().to_string();

        let updated = invoke(
            &webview,
            "account_update",
            serde_json::json!({ "id": id, "input": { "initial_balance": "100.00" } }),
        )
        .expect("update income with initial_balance should succeed but skip opening balance");
        assert_eq!(updated["initial_balance"], serde_json::json!(null));
    }

    #[test]
    fn test_account_batch_create_with_initial_balance() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let batch: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_batch_create",
                serde_json::json!({
                    "inputs": [
                        { "name": "Assets/Savings", "initialBalance": "500.00" },
                        { "name": "Expenses/Rent", "initialBalance": "0" }
                    ]
                }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(batch.len(), 2);
        let names: Vec<&str> = batch.iter().map(|a| a["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"Savings"));
        assert!(names.contains(&"Rent"));
    }

    #[test]
    fn test_account_transactions_and_meta_commands() {
        let (_dir, db) = test_db();
        let (_app, webview) = mock_app_with_db(db);

        let from = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Assets/From" } }),
        )
        .unwrap();
        let to = invoke(
            &webview,
            "account_create",
            serde_json::json!({ "input": { "name": "Assets/To" } }),
        )
        .unwrap();
        let from_id = from["id"].as_str().unwrap().to_string();
        let to_id = to["id"].as_str().unwrap().to_string();

        invoke(
            &webview,
            "account_transfer",
            serde_json::json!({
                "fromId": from_id,
                "toId": to_id,
                "amount": 20000,
                "description": "Move"
            }),
        )
        .unwrap();

        // account_transactions with date range
        let txns: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_transactions",
                serde_json::json!({
                    "id": from_id,
                    "fromDate": "2024-01-01",
                    "toDate": "2099-12-31"
                }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(txns.len(), 1);

        // account_meta_batch_set then get
        invoke(
            &webview,
            "account_meta_batch_set",
            serde_json::json!({
                "accountId": from_id,
                "metas": [["bank", "ICBC"], ["notes", "hi"]]
            }),
        )
        .unwrap();
        let metas: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "account_meta_get",
                serde_json::json!({ "accountId": from_id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(metas.len(), 2);

        // account_meta_schema
        let schema = invoke(&webview, "account_meta_schema", serde_json::json!({})).unwrap();
        assert!(!schema["valid_account_roles"].as_array().unwrap().is_empty());
        assert!(!schema["valid_liability_types"].as_array().unwrap().is_empty());
    }
}
