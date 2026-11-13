//! Core account CRUD plus balance lookups.

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

use super::types::{AccountRecord, UpdateAccountParams};

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
pub(in crate::db::accounts) fn delete_opening_balance(
    conn: &Connection,
    account_id: &str,
) -> Result<(), AppError> {
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
        "SELECT COALESCE(SUM(p.amount), 0)
         FROM postings p
         JOIN transactions t ON t.id = p.transaction_id
         WHERE p.account_id = ?1 AND t.deleted_at IS NULL",
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
        "SELECT p.account_id, COALESCE(SUM(p.amount), 0) as balance
         FROM postings p
         JOIN transactions t ON t.id = p.transaction_id
         WHERE p.account_id IN ({}) AND t.deleted_at IS NULL
         GROUP BY p.account_id",
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
