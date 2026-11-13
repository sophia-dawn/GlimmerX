//! Free-form key/value metadata attached to accounts.

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

use super::types::AccountMetaRecord;

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
