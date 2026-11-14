//! Transfers between two accounts.

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::AppError;
use crate::utils::money::MAX_AMOUNT_CENTS;

// Shared with the quick-add transfer path; see `validate.rs` (P0-1).
use super::validate::validate_endpoint;

/// Create a transfer transaction between two accounts with balanced postings.
/// Returns the transaction ID.
///
/// Legacy entry: this path hardcodes `today_date()` and takes raw cents. New
/// callers — the AI path and the ledger capability layer — must go through
/// quick-add instead, which supports a business date (design doc 25, P0-4).
pub fn create_transfer(
    conn: &Connection,
    from_id: &str,
    to_id: &str,
    amount_cents: i64,
    description: &str,
) -> Result<String, AppError> {
    if amount_cents <= 0 {
        return Err(AppError::ValidationError(
            "errors.account.transferAmountInvalid".into(),
        ));
    }
    // The same ceiling as every parsed amount: this command takes raw cents over
    // IPC, so without it a caller could store an amount that overflows every
    // later `SUM()` over the account.
    if amount_cents > MAX_AMOUNT_CENTS {
        return Err(AppError::ValidationError(
            "errors.account.transferAmountTooLarge".into(),
        ));
    }
    if from_id == to_id {
        return Err(AppError::ValidationError(
            "errors.account.transferSameAccount".into(),
        ));
    }
    validate_endpoint(conn, from_id)?;
    validate_endpoint(conn, to_id)?;

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

    // Explicit `sequence`: both rows used to land on the default 0, which left
    // the posting order in the detail view undefined.
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, ?5)",
        rusqlite::params![&posting1_id, &tx_id, to_id, amount_cents, &now],
    )?;

    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5)",
        rusqlite::params![&posting2_id, &tx_id, from_id, -amount_cents, &now],
    )?;

    Ok(tx_id)
}
