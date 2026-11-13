//! Transfers between two accounts.

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::AppError;

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
