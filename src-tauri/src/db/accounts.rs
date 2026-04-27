use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::constants::account_type;
use crate::db::AppError;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Internal DB row representation for an account.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountRecord {
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
}

/// Transaction record for account transaction history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionRecord {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub amount: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Metadata key-value record for accounts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AccountMetaRecord {
    pub id: String,
    pub account_id: String,
    pub key: String,
    pub value: String,
    pub created_at: String,
}

/// Insert a single meta value for an account. Returns the new meta ID.
pub fn insert_meta(
    conn: &Connection,
    account_id: &str,
    key: &str,
    value: &str,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();

    conn.execute(
        "INSERT INTO account_meta (id, account_id, key, value, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&id, account_id, key, value, &now],
    )?;

    Ok(id)
}

/// Get all meta values for an account.
pub fn get_meta_for_account(
    conn: &Connection,
    account_id: &str,
) -> Result<Vec<AccountMetaRecord>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, key, value, created_at FROM account_meta WHERE account_id = ?1 ORDER BY key",
    )?;

    let metas = stmt
        .query_map([account_id], |row| {
            Ok(AccountMetaRecord {
                id: row.get(0)?,
                account_id: row.get(1)?,
                key: row.get(2)?,
                value: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(metas)
}

/// Get a single meta value by key. Returns None if key doesn't exist.
#[allow(dead_code)]
pub fn get_meta_value(
    conn: &Connection,
    account_id: &str,
    key: &str,
) -> Result<Option<String>, AppError> {
    let value: Option<String> = conn
        .query_row(
            "SELECT value FROM account_meta WHERE account_id = ?1 AND key = ?2",
            rusqlite::params![account_id, key],
            |row| row.get(0),
        )
        .optional()?;

    Ok(value)
}

/// Delete all meta values for an account.
pub fn delete_meta_for_account(conn: &Connection, account_id: &str) -> Result<usize, AppError> {
    let count = conn.execute(
        "DELETE FROM account_meta WHERE account_id = ?1",
        [account_id],
    )?;

    Ok(count)
}

/// Set multiple meta values for an account in a single operation.
/// Uses INSERT ... ON CONFLICT(key) DO UPDATE SET value = excluded.value
pub fn set_meta_batch(
    conn: &Connection,
    account_id: &str,
    metas: &[(String, String)],
) -> Result<usize, AppError> {
    if metas.is_empty() {
        return Ok(0);
    }

    let now = crate::utils::time::now_rfc3339();
    let mut total = 0;

    for (key, value) in metas {
        let id = Uuid::new_v4().to_string();
        let count = conn.execute(
            "INSERT INTO account_meta (id, account_id, key, value, created_at) VALUES (?1, ?2, ?3, ?4, ?5) \
             ON CONFLICT(account_id, key) DO UPDATE SET value = excluded.value, created_at = excluded.created_at",
            rusqlite::params![&id, account_id, key, value, &now],
        )?;
        total += count;
    }

    Ok(total)
}

// ---------------------------------------------------------------------------
// Valid account root types
// ---------------------------------------------------------------------------

const VALID_ROOT_TYPES: &[&str] = &["asset", "liability", "income", "expense", "equity"];

/// Map common plural forms to their singular type names.
fn normalize_root_type(segment: &str) -> Option<String> {
    let lower = segment.to_lowercase();
    if VALID_ROOT_TYPES.contains(&lower.as_str()) {
        return Some(lower);
    }
    let mapping = [
        ("assets", "asset"),
        ("liabilities", "liability"),
        ("incomes", "income"),
        ("expenses", "expense"),
        ("equities", "equity"),
    ];
    for (from, to) in mapping {
        if lower == from {
            return Some(to.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Path parsing helpers
// ---------------------------------------------------------------------------

/// Split a full path like "Assets/Bank/招商银行" into segments.
fn parse_path(full_path: &str) -> Vec<&str> {
    full_path
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Determine the account type from the first segment of the path.
fn type_from_path(segments: &[&str]) -> Result<String, AppError> {
    let first = segments
        .first()
        .ok_or_else(|| AppError::ValidationError("Account path must not be empty".into()))?;
    normalize_root_type(first).ok_or_else(|| {
        AppError::ValidationError(format!(
            "Invalid account root type '{}'. Valid types: {:?}",
            first, VALID_ROOT_TYPES
        ))
    })
}

/// Common columns for SELECT queries.
const ACCOUNT_COLUMNS: &str =
    "id, name, type, currency, description, account_number, is_system, iban, is_active, include_net_worth, created_at, updated_at";

/// Read an AccountRecord from a row.
fn row_to_account(row: &rusqlite::Row<'_>) -> rusqlite::Result<AccountRecord> {
    Ok(AccountRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        account_type: row.get(2)?,
        currency: row.get(3)?,
        description: row.get(4)?,
        account_number: row.get(5)?,
        is_system: row.get::<_, i32>(6)? != 0,
        iban: row.get(7)?,
        is_active: row.get::<_, i32>(8)? != 0,
        include_net_worth: row.get::<_, i32>(9)? != 0,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// Insert a single account record. Returns the new account ID.
pub fn insert_account(conn: &Connection, acct: &AccountRecord) -> Result<String, AppError> {
    let now = crate::utils::time::now_rfc3339();
    let id = Uuid::new_v4().to_string();

    conn.execute(
        &format!(
            "INSERT INTO accounts ({}) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            ACCOUNT_COLUMNS
        ),
        rusqlite::params![
            &id,
            &acct.name,
            &acct.account_type,
            &acct.currency,
            &acct.description,
            &acct.account_number,
            &(acct.is_system as i32),
            &acct.iban,
            &(acct.is_active as i32),
            &(acct.include_net_worth as i32),
            &now,
            &now,
        ],
    )?;

    Ok(id)
}

/// Find a single account by ID.
pub fn find_account(conn: &Connection, id: &str) -> Result<Option<AccountRecord>, AppError> {
    let row = conn
        .query_row(
            &format!("SELECT {} FROM accounts WHERE id = ?1", ACCOUNT_COLUMNS),
            [id],
            row_to_account,
        )
        .optional()?;

    Ok(row)
}

/// Find an account by type and name (composite unique key).
pub fn find_account_by_type_and_name(
    conn: &Connection,
    account_type: &str,
    name: &str,
) -> Result<Option<AccountRecord>, AppError> {
    conn.query_row(
        &format!(
            "SELECT {} FROM accounts WHERE type = ?1 AND name = ?2",
            ACCOUNT_COLUMNS
        ),
        params![account_type, name],
        row_to_account,
    )
    .optional()
    .map_err(|e| e.into())
}

/// List all accounts (excluding system accounts like Equity and Opening Balances).
pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM accounts WHERE is_system = 0 ORDER BY name",
        ACCOUNT_COLUMNS
    ))?;

    let accounts = stmt
        .query_map([], row_to_account)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(accounts)
}

/// Parameters for updating an account.
pub struct UpdateAccountParams<'a> {
    pub id: &'a str,
    pub name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub account_number: Option<Option<&'a str>>,
    pub iban: Option<Option<&'a str>>,
    pub is_active: Option<bool>,
    pub include_net_worth: Option<bool>,
}

/// Update account fields.
pub fn update_account(conn: &Connection, params: &UpdateAccountParams<'_>) -> Result<(), AppError> {
    if let Some(new_name) = params.name {
        let current_type: String = conn.query_row(
            "SELECT type FROM accounts WHERE id = ?1",
            [&params.id],
            |row| row.get(0),
        )?;
        if let Some(existing) = find_account_by_type_and_name(conn, &current_type, new_name)? {
            if existing.id != params.id {
                return Err(AppError::ValidationError(
                    "errors.accountNameAlreadyExists".to_string(),
                ));
            }
        }
    }

    let now = crate::utils::time::now_rfc3339();

    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(val) = params.name {
        sets.push("name = ?".to_string());
        sql_params.push(Box::new(val.to_string()));
    }
    if let Some(val) = params.description {
        sets.push("description = ?".to_string());
        sql_params.push(Box::new(val.to_string()));
    }
    if let Some(val) = params.account_number {
        sets.push("account_number = ?".to_string());
        if let Some(v) = val {
            sql_params.push(Box::new(v.to_string()));
        } else {
            sql_params.push(Box::new(None::<String>));
        }
    }
    if let Some(val) = params.iban {
        sets.push("iban = ?".to_string());
        if let Some(v) = val {
            sql_params.push(Box::new(v.to_string()));
        } else {
            sql_params.push(Box::new(None::<String>));
        }
    }
    if let Some(val) = params.is_active {
        sets.push("is_active = ?".to_string());
        sql_params.push(Box::new(val as i32));
    }
    if let Some(val) = params.include_net_worth {
        sets.push("include_net_worth = ?".to_string());
        sql_params.push(Box::new(val as i32));
    }

    if sets.is_empty() {
        return Ok(());
    }

    sets.push("updated_at = ?".to_string());
    sql_params.push(Box::new(now));
    sql_params.push(Box::new(params.id.to_string()));

    let sql = format!("UPDATE accounts SET {} WHERE id = ?", sets.join(", "));

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();
    stmt.execute(rusqlite::params_from_iter(param_refs))?;

    Ok(())
}

/// Delete an account permanently. Only allowed if:
/// - It has no user-created transactions (opening balance transactions are auto-cleaned)
pub fn delete_account(conn: &Connection, id: &str) -> Result<(), AppError> {
    let account = find_account(conn, id)?
        .ok_or_else(|| AppError::ValidationError("Account not found".into()))?;

    // System accounts (Equity, Opening Balances) cannot be deleted
    if account.is_system {
        return Err(AppError::ValidationError("errors.accountIsSystem".into()));
    }

    // 1. Check for user-created transactions FIRST — this is the red line
    let user_tx_count: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT t.id) FROM transactions t
         JOIN postings p ON p.transaction_id = t.id
         WHERE p.account_id = ?1 AND t.description != 'Opening Balance'",
        [id],
        |row| row.get(0),
    )?;
    if user_tx_count > 0 {
        return Err(AppError::ValidationError(
            "errors.accountHasTransactions".into(),
        ));
    }

    // 4. Safe to clean up opening balance entries
    let mut stmt = conn.prepare(
        "SELECT DISTINCT t.id FROM transactions t
         JOIN postings p ON p.transaction_id = t.id
         WHERE p.account_id = ?1 AND t.description = 'Opening Balance'",
    )?;
    let ob_tx_ids: Vec<String> = stmt
        .query_map([id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    drop(stmt);
    for tx_id in &ob_tx_ids {
        // Delete postings belonging to this opening balance transaction
        conn.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;
        // Delete the transaction itself
        conn.execute("DELETE FROM transactions WHERE id = ?1", [tx_id])?;
    }

    // 4. Safe to delete
    conn.execute("DELETE FROM accounts WHERE id = ?1", [id])?;
    Ok(())
}

/// Delete any existing opening balance transactions for an account.
fn delete_opening_balance(conn: &Connection, account_id: &str) -> Result<(), AppError> {
    let ob_tx_ids: Vec<String> = conn
        .prepare(
            "SELECT DISTINCT t.id FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             WHERE p.account_id = ?1 AND t.description = 'Opening Balance'",
        )?
        .query_map([account_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    for tx_id in &ob_tx_ids {
        conn.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;
        conn.execute("DELETE FROM transactions WHERE id = ?1", [tx_id])?;
    }
    Ok(())
}

#[allow(dead_code)]
/// Get the opening balance (amount and date) for a specific account.
/// Returns (amount_cents, date) if an Opening Balance transaction exists, otherwise None.
pub fn get_opening_balance_for_account(
    conn: &Connection,
    account_id: &str,
) -> Result<Option<(i64, String)>, AppError> {
    let result: Option<(i64, String)> = conn
        .query_row(
            "SELECT p.amount, t.date FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             WHERE t.description = 'Opening Balance' AND p.account_id = ?1
             LIMIT 1",
            [account_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    Ok(result)
}

/// Get the current balance for an account (SUM of postings.amount, in cents).
pub fn get_account_balance(conn: &Connection, id: &str) -> Result<i64, AppError> {
    let balance: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM postings WHERE account_id = ?1",
        [id],
        |row| row.get(0),
    )?;
    Ok(balance)
}

/// Get balances for multiple accounts in a single query.
/// Returns a Vec of (account_id, balance) tuples.
pub fn get_account_balances_batch(
    conn: &Connection,
    ids: &[String],
) -> Result<Vec<(String, i64)>, AppError> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT account_id, COALESCE(SUM(amount), 0) as balance 
         FROM postings 
         WHERE account_id IN ({}) 
         GROUP BY account_id",
        placeholders.join(", ")
    );

    let params: Vec<&String> = ids.iter().collect();
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

    let mut stmt = conn.prepare(&sql)?;
    let balances: Vec<(String, i64)> = stmt
        .query_map(params_refs.as_slice(), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(balances)
}

// ---------------------------------------------------------------------------
// Path-based account creation with auto-parent creation
// ---------------------------------------------------------------------------

/// Full input for creating an account with all fields.
#[derive(Default, Clone)]
pub struct CreateAccountFullInput {
    pub description: Option<String>,
    pub account_number: Option<String>,
    pub iban: Option<String>,
    pub is_active: bool,
    pub include_net_worth: bool,
}

/// Create an account from a path like "Assets/招商银行".
/// First segment determines type. Remaining segments joined by "/" become the name.
/// Returns the created account's ID.
pub fn create_account_with_path(
    conn: &Connection,
    full_path: &str,
    currency: &str,
    input: Option<&CreateAccountFullInput>,
) -> Result<String, AppError> {
    let segments = parse_path(full_path);
    if segments.is_empty() {
        return Err(AppError::ValidationError(
            "Account path must contain at least one segment".into(),
        ));
    }

    let account_type = type_from_path(&segments)?;
    // Name is all segments after the first, joined by "/"
    let name = if segments.len() > 1 {
        segments[1..].join("/")
    } else {
        segments[0].to_string()
    };

    // Check if account with same type and name already exists
    if let Some(_existing) = find_account_by_type_and_name(conn, &account_type, &name)? {
        return Err(AppError::ValidationError(
            "errors.accountNameAlreadyExists".to_string(),
        ));
    }

    let full_input = input.cloned().unwrap_or(CreateAccountFullInput {
        is_active: true,
        include_net_worth: false,
        description: None,
        account_number: None,
        iban: None,
    });
    // Income/Expense accounts are P&L accounts - never include in net worth
    // Asset/Liability can be controlled by user (e.g., some liability accounts may be excluded)
    let include_net_worth = if account_type::should_include_in_net_worth(&account_type) {
        full_input.include_net_worth
    } else {
        false
    };
    let acct = AccountRecord {
        id: String::new(),
        name,
        account_type,
        currency: currency.to_string(),
        description: full_input.description.unwrap_or_default(),
        account_number: full_input.account_number,
        is_system: false,
        iban: full_input.iban,
        is_active: full_input.is_active,
        include_net_worth,
        created_at: String::new(),
        updated_at: String::new(),
    };
    insert_account(conn, &acct)
}

// ---------------------------------------------------------------------------
// Batch account creation
// ---------------------------------------------------------------------------

/// Name for a single account in a batch creation request.
pub struct BatchAccountInput {
    pub name: String,
    pub currency: String,
    pub initial_balance: Option<i64>,
    pub description: Option<String>,
    pub account_number: Option<String>,
}

/// Create multiple accounts in a single call.
/// Returns the created account records.
pub fn batch_create_accounts(
    conn: &Connection,
    inputs: Vec<BatchAccountInput>,
) -> Result<Vec<AccountRecord>, AppError> {
    if inputs.is_empty() {
        return Ok(vec![]);
    }

    let mut created_accounts = Vec::new();
    for input in &inputs {
        let segments = parse_path(&input.name);
        let account_type = if segments.is_empty() {
            "asset".to_string()
        } else {
            type_from_path(&segments)?
        };
        let name = if segments.len() > 1 {
            segments[1..].join("/")
        } else {
            input.name.clone()
        };

        if let Some(_existing) = find_account_by_type_and_name(conn, &account_type, &name)? {
            return Err(AppError::ValidationError(
                "errors.accountNameAlreadyExists".to_string(),
            ));
        }

        let include_net_worth = if account_type::should_include_in_net_worth(&account_type) {
            true // Batch creation doesn't have user input, use default true for balance sheet accounts
        } else {
            false // P&L accounts never include in net worth
        };
        let acct = AccountRecord {
            id: String::new(),
            name,
            account_type,
            currency: input.currency.clone(),
            description: input.description.clone().unwrap_or_default(),
            account_number: input.account_number.clone(),
            is_system: false,
            iban: None,
            is_active: true,
            include_net_worth,
            created_at: String::new(),
            updated_at: String::new(),
        };
        let new_id = insert_account(conn, &acct)?;

        if let Some(balance) = input.initial_balance {
            if balance != 0 {
                create_opening_balance(conn, &new_id, balance, "Equity", "Opening Balances", None)?;
            }
        }

        if let Some(record) = find_account(conn, &new_id)? {
            created_accounts.push(record);
        }
    }

    Ok(created_accounts)
}

/// Create or update an Opening Balance transaction for the given account.
/// If an opening balance transaction already exists for this account, it is updated.
pub fn create_opening_balance(
    conn: &Connection,
    account_id: &str,
    amount_cents: i64,
    equity_name: &str,
    opening_balances_name: &str,
    date: Option<&str>,
) -> Result<(), AppError> {
    // If amount is 0, delete any existing opening balance transaction
    if amount_cents == 0 {
        delete_opening_balance(conn, account_id)?;
        return Ok(());
    }

    let tx_date = date
        .unwrap_or(&crate::utils::time::today_date())
        .to_string();
    let now = crate::utils::time::now_rfc3339();

    // Get account type to determine sign for liability accounts
    // Liability accounts store negative balances (negative = money owed)
    let account_type: String = conn.query_row(
        "SELECT type FROM accounts WHERE id = ?1",
        rusqlite::params![account_id],
        |row| row.get(0),
    )?;
    let posting_amount = if account_type == "liability" {
        -amount_cents // Liability: negative balance = debt owed
    } else {
        amount_cents // Asset/other: positive balance = money owned
    };

    let equity_id = ensure_opening_balance_equity(conn, equity_name, opening_balances_name)?;

    // Check for existing opening balance transaction
    let existing_tx: Option<String> = conn
        .query_row(
            "SELECT t.id FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             WHERE t.description = 'Opening Balance' AND p.account_id = ?1
             LIMIT 1",
            rusqlite::params![account_id],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(tx_id) = existing_tx {
        // Update the existing transaction
        conn.execute(
            "UPDATE transactions SET date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![&tx_date, &now, &tx_id],
        )?;

        // Update the posting amounts for this account
        conn.execute(
            "UPDATE postings SET amount = ?1 WHERE transaction_id = ?2 AND account_id = ?3",
            rusqlite::params![posting_amount, &tx_id, account_id],
        )?;

        // Update the equity posting (opposite sign)
        conn.execute(
            "UPDATE postings SET amount = ?1 WHERE transaction_id = ?2 AND account_id = ?3",
            rusqlite::params![-posting_amount, &tx_id, &equity_id],
        )?;
    } else {
        // Create new opening balance transaction
        let tx_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
            rusqlite::params![&tx_id, &tx_date, "Opening Balance", &now],
        )?;

        let posting1_id = Uuid::new_v4().to_string();
        let posting2_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&posting1_id, &tx_id, account_id, posting_amount, &now],
        )?;

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&posting2_id, &tx_id, &equity_id, -posting_amount, &now],
        )?;
    }

    Ok(())
}

/// Ensure a flat "Equity: Opening Balances" account exists.
/// Returns the opening balances account ID.
fn ensure_opening_balance_equity(
    conn: &Connection,
    equity_name: &str,
    opening_balances_name: &str,
) -> Result<String, AppError> {
    let full_name = format!("{}: {}", equity_name, opening_balances_name);
    if let Some(existing) = find_account_by_type_and_name(conn, "equity", &full_name)? {
        return Ok(existing.id);
    }
    let acct = AccountRecord {
        id: String::new(),
        name: full_name,
        account_type: "equity".to_string(),
        currency: "CNY".to_string(),
        description: String::new(),
        account_number: None,
        is_system: true,
        iban: None,
        is_active: true,
        include_net_worth: false,
        created_at: String::new(),
        updated_at: String::new(),
    };
    insert_account(conn, &acct)
}

/// Create a transfer transaction between two accounts with balanced postings.
/// Validates that both accounts use the same currency.
/// Returns the transaction ID.
pub fn create_transfer(
    conn: &Connection,
    from_id: &str,
    to_id: &str,
    amount_cents: i64,
    description: &str,
) -> Result<String, AppError> {
    if amount_cents <= 0 {
        return Err(AppError::ValidationError(
            "Transfer amount must be positive".into(),
        ));
    }
    if from_id == to_id {
        return Err(AppError::ValidationError(
            "Cannot transfer to the same account".into(),
        ));
    }

    // Validate currency match
    let from_currency: String = conn.query_row(
        "SELECT currency FROM accounts WHERE id = ?1",
        [from_id],
        |row| row.get(0),
    )?;
    let to_currency: String = conn.query_row(
        "SELECT currency FROM accounts WHERE id = ?1",
        [to_id],
        |row| row.get(0),
    )?;
    if from_currency != to_currency {
        return Err(AppError::ValidationError(format!(
            "Cannot transfer between accounts with different currencies ({} and {})",
            from_currency, to_currency
        )));
    }

    let tx_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    let today = crate::utils::time::today_date();

    conn.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, ?4, ?5)",
        rusqlite::params![&tx_id, &today, description, &now, &now],
    )?;

    let posting1_id = Uuid::new_v4().to_string();
    let posting2_id = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&posting1_id, &tx_id, to_id, amount_cents, &now],
    )?;

    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![&posting2_id, &tx_id, from_id, -amount_cents, &now],
    )?;

    Ok(tx_id)
}

// ---------------------------------------------------------------------------
// Account Transaction History
// ---------------------------------------------------------------------------

/// Get all transactions for an account, optionally filtered by date range.
pub fn get_account_transactions(
    conn: &Connection,
    account_id: &str,
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<Vec<TransactionRecord>, AppError> {
    let base_sql =
        "SELECT t.id, t.date, t.description, t.category_id, p.amount, t.created_at, t.updated_at
                    FROM transactions t
                    JOIN postings p ON p.transaction_id = t.id AND p.account_id = ?1";

    let mut sql = base_sql.to_string();

    if from_date.is_some() {
        sql.push_str(" AND t.date >= ?2");
    }
    if to_date.is_some() {
        sql.push_str(" AND t.date <= ?3");
    }
    sql.push_str(" ORDER BY t.date DESC, t.id DESC");

    let mut stmt = conn.prepare(&sql)?;

    let transactions: Vec<TransactionRecord> = match (from_date, to_date) {
        (Some(from), Some(to)) => stmt
            .query_map(rusqlite::params![account_id, from, to], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    category_id: row.get(3)?,
                    amount: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (Some(from), None) => stmt
            .query_map(rusqlite::params![account_id, from], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    category_id: row.get(3)?,
                    amount: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (None, Some(to)) => stmt
            .query_map(rusqlite::params![account_id, to], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    category_id: row.get(3)?,
                    amount: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (None, None) => stmt
            .query_map(rusqlite::params![account_id], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    category_id: row.get(3)?,
                    amount: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(transactions)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
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

            CREATE UNIQUE INDEX idx_accounts_type_name
                ON accounts(type, name);

            CREATE UNIQUE INDEX idx_account_meta_account_key
                ON account_meta(account_id, key);

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
        "#,
        )
        .unwrap();
        (dir, conn)
    }

    fn make_account(name: &str, account_type: &str) -> AccountRecord {
        AccountRecord {
            id: String::new(),
            name: name.to_string(),
            account_type: account_type.to_string(),
            currency: "CNY".to_string(),
            description: String::new(),
            account_number: None,
            is_system: false,
            iban: None,
            is_active: true,
            include_net_worth: true,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_insert_account() {
        let (_dir, conn) = test_conn();
        let acct = make_account("招商银行", "asset");
        let id = insert_account(&conn, &acct).unwrap();
        assert!(!id.is_empty());

        let found = find_account(&conn, &id).unwrap().unwrap();
        assert_eq!(found.name, "招商银行");
    }

    #[test]
    fn test_list_accounts_by_type() {
        let (_dir, conn) = test_conn();

        let _id1 = insert_account(&conn, &make_account("招商银行", "asset")).unwrap();
        let _id2 = insert_account(&conn, &make_account("现金", "asset")).unwrap();
        let _id3 = insert_account(&conn, &make_account("花呗", "liability")).unwrap();

        let accounts = list_accounts(&conn).unwrap();
        assert_eq!(accounts.len(), 3);

        // Flat structure - all accounts are roots
    }

    #[test]
    fn test_update_account() {
        let (_dir, conn) = test_conn();

        let acct = make_account("OldName", "asset");
        let id = insert_account(&conn, &acct).unwrap();

        update_account(
            &conn,
            &UpdateAccountParams {
                id: &id,
                name: Some("NewName"),
                description: None,
                account_number: None,
                iban: None,
                is_active: None,
                include_net_worth: None,
            },
        )
        .unwrap();

        let updated = find_account(&conn, &id).unwrap().unwrap();
        assert_eq!(updated.name, "NewName");
    }

    #[test]
    fn test_delete_account() {
        let (_dir, conn) = test_conn();

        let acct = make_account("ToDelete", "expense");
        let id = insert_account(&conn, &acct).unwrap();
        delete_account(&conn, &id).unwrap();

        assert!(find_account(&conn, &id).unwrap().is_none());
    }

    #[test]
    fn test_delete_account_with_transactions_blocked() {
        let (_dir, conn) = test_conn();

        let acct = make_account("WithTx", "asset");
        let acct_id = insert_account(&conn, &acct).unwrap();

        // Create a transaction with a posting to this account
        let tx_id = "test-tx-1";
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test Tx', NULL, ?2, ?2)",
            rusqlite::params![tx_id, crate::utils::time::now_rfc3339()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 1000, ?4)",
            rusqlite::params![
                "posting-1",
                tx_id,
                acct_id,
                crate::utils::time::now_rfc3339()
            ],
        )
        .unwrap();

        let result = delete_account(&conn, &acct_id);
        assert!(
            result.is_err(),
            "Should block deletion of account with transactions"
        );
    }

    #[test]
    fn test_create_account_with_path_flat() {
        let (_dir, conn) = test_conn();

        let id = create_account_with_path(&conn, "Assets/招商银行", "CNY", None).unwrap();
        let acct = find_account(&conn, &id).unwrap().unwrap();
        assert_eq!(acct.name, "招商银行");
        assert_eq!(acct.account_type, "asset");

        // Sub-path becomes name
        let id2 = create_account_with_path(&conn, "Liabilities/信用卡/花呗", "CNY", None).unwrap();
        let acct2 = find_account(&conn, &id2).unwrap().unwrap();
        assert_eq!(acct2.name, "信用卡/花呗");
        assert_eq!(acct2.account_type, "liability");
    }

    #[test]
    fn test_schema_unique_type_name() {
        let (_dir, conn) = test_conn();

        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('1', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        // Duplicate (type, name) should fail
        let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('2', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_err(),
            "Duplicate (type, name) should violate unique index"
        );

        // Same name but different type should succeed
        let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('3', 'Test', 'liability', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_ok(),
            "Same name with different type should be allowed"
        );
    }

    #[test]
    fn test_accounts_check_type_constraint() {
        let (_dir, conn) = test_conn();

        let now = crate::utils::time::now_rfc3339();
        let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('1', 'Bad', 'invalid_type', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_err(),
            "Invalid type should violate CHECK constraint"
        );
    }

    #[test]
    fn test_balance_aggregation_no_sign_flip() {
        let (_dir, conn) = test_conn();

        // Create a liability account
        let acct = make_account("花呗", "liability");
        let id = insert_account(&conn, &acct).unwrap();

        // Add a posting
        let tx_id = "test-tx";
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test', NULL, ?2, ?2)",
            rusqlite::params![tx_id, crate::utils::time::now_rfc3339()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 5000, ?4)",
            rusqlite::params!["posting-1", tx_id, id, crate::utils::time::now_rfc3339()],
        )
        .unwrap();

        let balance = get_account_balance(&conn, &id).unwrap();
        // Raw balance should be returned as-is for liability (no sign flip)
        assert_eq!(balance, 5000);

        // Negative posting should stay negative
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                "posting-2",
                tx_id,
                id,
                -2000i64,
                crate::utils::time::now_rfc3339()
            ],
        )
        .unwrap();

        let balance = get_account_balance(&conn, &id).unwrap();
        assert_eq!(balance, 3000); // 5000 - 2000 = 3000
    }

    #[test]
    fn test_insert_meta() {
        let (_dir, conn) = test_conn();
        let acct = make_account("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        let meta_id = insert_meta(&conn, &account_id, "account_role", "ccAsset").unwrap();
        assert!(!meta_id.is_empty());

        let value = get_meta_value(&conn, &account_id, "account_role")
            .unwrap()
            .unwrap();
        assert_eq!(value, "ccAsset");
    }

    #[test]
    fn test_get_meta_for_account() {
        let (_dir, conn) = test_conn();
        let acct = make_account("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        insert_meta(&conn, &account_id, "bar", "value_bar").unwrap();
        insert_meta(&conn, &account_id, "foo", "value_foo").unwrap();

        let metas = get_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].key, "bar");
        assert_eq!(metas[1].key, "foo");
    }

    #[test]
    fn test_meta_unique_constraint() {
        let (_dir, conn) = test_conn();
        let acct = make_account("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        insert_meta(&conn, &account_id, "unique_key", "value1").unwrap();

        let result = insert_meta(&conn, &account_id, "unique_key", "value2");
        assert!(
            result.is_err(),
            "Duplicate meta key should violate unique constraint"
        );
    }

    #[test]
    fn test_delete_meta_for_account() {
        let (_dir, conn) = test_conn();
        let acct = make_account("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        insert_meta(&conn, &account_id, "key1", "val1").unwrap();
        insert_meta(&conn, &account_id, "key2", "val2").unwrap();

        let deleted = delete_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(deleted, 2);

        let metas = get_meta_for_account(&conn, &account_id).unwrap();
        assert!(metas.is_empty());
    }

    #[test]
    fn test_set_meta_batch() {
        let (_dir, conn) = test_conn();
        let acct = make_account("TestAccount", "asset");
        let account_id = insert_account(&conn, &acct).unwrap();

        let metas = vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
            ("key3".to_string(), "val3".to_string()),
        ];

        let count = set_meta_batch(&conn, &account_id, &metas).unwrap();
        assert_eq!(count, 3);

        let result = get_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(result.len(), 3);
        let values: std::collections::HashMap<&str, &str> = result
            .iter()
            .map(|m| (m.key.as_str(), m.value.as_str()))
            .collect();
        assert_eq!(values.get("key1"), Some(&"val1"));
        assert_eq!(values.get("key2"), Some(&"val2"));
        assert_eq!(values.get("key3"), Some(&"val3"));

        // Second call with same keys but different values - upsert behavior
        let metas2 = vec![
            ("key1".to_string(), "updated1".to_string()),
            ("key2".to_string(), "updated2".to_string()),
            ("key3".to_string(), "updated3".to_string()),
        ];

        let count2 = set_meta_batch(&conn, &account_id, &metas2).unwrap();
        assert_eq!(count2, 3);

        let result2 = get_meta_for_account(&conn, &account_id).unwrap();
        assert_eq!(result2.len(), 3);
        let values2: std::collections::HashMap<&str, &str> = result2
            .iter()
            .map(|m| (m.key.as_str(), m.value.as_str()))
            .collect();
        assert_eq!(values2.get("key1"), Some(&"updated1"));
        assert_eq!(values2.get("key2"), Some(&"updated2"));
        assert_eq!(values2.get("key3"), Some(&"updated3"));
    }

    #[test]
    fn test_get_account_transactions_multiple() {
        let (_dir, conn) = test_conn();

        // Create account
        let acct = make_account("TestAccount", "asset");
        let acct_id = insert_account(&conn, &acct).unwrap();

        // Create 3 transactions with postings
        for i in 1..=3 {
            let tx_id = format!("tx-{}", i);
            let now = crate::utils::time::now_rfc3339();
            conn.execute(
                "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
                rusqlite::params![&tx_id, format!("2024-0{}-01", i), format!("Transaction {}", i), &now],
            ).unwrap();
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![format!("posting-{}", i), &tx_id, &acct_id, i * 1000, &now],
            )
            .unwrap();
        }

        let transactions = get_account_transactions(&conn, &acct_id, None, None).unwrap();
        assert_eq!(transactions.len(), 3);

        // Verify amounts are correct
        let amounts: Vec<i64> = transactions.iter().map(|t| t.amount).collect();
        assert!(amounts.contains(&1000));
        assert!(amounts.contains(&2000));
        assert!(amounts.contains(&3000));
    }

    #[test]
    fn test_update_account_duplicate_type_name_blocked() {
        let (_dir, conn) = test_conn();

        // 创建两个同类型账户
        let acct1 = make_account("账户A", "asset");
        let _id1 = insert_account(&conn, &acct1).unwrap();
        let acct2 = make_account("账户B", "asset");
        let id2 = insert_account(&conn, &acct2).unwrap();

        // 尝试将账户B的名称改为账户A的名称（同类型）
        let result = update_account(
            &conn,
            &UpdateAccountParams {
                id: &id2,
                name: Some("账户A"),
                description: None,
                account_number: None,
                iban: None,
                is_active: None,
                include_net_worth: None,
            },
        );

        // 应返回 ValidationError（同类型同名冲突）
        assert!(
            result.is_err(),
            "Should block duplicate (type, name) on update"
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, AppError::ValidationError(msg) if msg == "errors.accountNameAlreadyExists"),
            "Error should be ValidationError with correct key"
        );
    }

    #[test]
    fn test_update_account_same_name_different_type_allowed() {
        let (_dir, conn) = test_conn();

        // 创建不同类型的账户
        let acct1 = make_account("账户X", "asset");
        let _id1 = insert_account(&conn, &acct1).unwrap();
        let acct2 = make_account("账户Y", "liability");
        let id2 = insert_account(&conn, &acct2).unwrap();

        // 尝试将 liability 账户名称改为与 asset 账户相同（不同类型）
        let result = update_account(
            &conn,
            &UpdateAccountParams {
                id: &id2,
                name: Some("账户X"),
                description: None,
                account_number: None,
                iban: None,
                is_active: None,
                include_net_worth: None,
            },
        );

        // 应成功（不同类型可以同名）
        assert!(
            result.is_ok(),
            "Same name with different type should be allowed on update"
        );
    }

    #[test]
    fn test_update_account_same_name_allowed() {
        let (_dir, conn) = test_conn();

        // 创建一个账户
        let acct = make_account("账户A", "asset");
        let id = insert_account(&conn, &acct).unwrap();

        // 更新账户的其他字段，名称保持不变
        let result = update_account(
            &conn,
            &UpdateAccountParams {
                id: &id,
                name: Some("账户A"), // 同名
                description: Some("新描述"),
                account_number: None,
                iban: None,
                is_active: None,
                include_net_worth: None,
            },
        );

        // 应成功
        assert!(result.is_ok(), "Should allow update with same name");

        // 验证描述已更新
        let updated = find_account(&conn, &id).unwrap().unwrap();
        assert_eq!(updated.description, "新描述");
    }

    #[test]
    fn test_get_account_balances_batch_empty() {
        let (_dir, conn) = test_conn();

        let result = get_account_balances_batch(&conn, &[]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_get_account_balances_batch_multiple() {
        let (_dir, conn) = test_conn();

        let acct1 = make_account("银行", "asset");
        let acct2 = make_account("现金", "asset");
        let id1 = insert_account(&conn, &acct1).unwrap();
        let id2 = insert_account(&conn, &acct2).unwrap();

        // Accounts without postings won't appear in batch results
        let balances = get_account_balances_batch(&conn, &[id1.clone(), id2.clone()]).unwrap();
        assert_eq!(balances.len(), 0);

        let tx_id = "test-tx";
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test', NULL, ?2, ?2)",
            rusqlite::params![tx_id, &now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 1000, ?4)",
            rusqlite::params!["posting-1", tx_id, &id1, &now],
        )
        .unwrap();

        let balances = get_account_balances_batch(&conn, &[id1.clone(), id2.clone()]).unwrap();
        assert_eq!(balances.len(), 1);
        assert_eq!(balances[0].0, id1);
        assert_eq!(balances[0].1, 1000);
    }
}
