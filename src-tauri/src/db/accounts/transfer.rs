//! Transfers between two accounts.

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::AppError;

/// Create a transfer transaction between two accounts with balanced postings.
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

    let tx_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    let today = crate::utils::time::today_date();

    conn.execute(
        "INSERT INTO transactions (id, date, description, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
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
