//! Explicit transaction creation from a list of postings.

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::AppError;

use super::types::{PostingInput, POSTING_COLUMNS, TRANSACTION_COLUMNS};
use super::validate::{validate_accounts_active, validate_no_equity_accounts};

/// Create a new transaction with balanced postings.
///
/// Validates:
/// - At least 2 postings required
/// - Sum of postings.amount must equal 0 (double-entry rule)
/// - All account_ids must exist and be active
/// - No equity accounts allowed (system transactions only)
///
/// Returns the new transaction ID.
#[allow(dead_code)]
pub fn create_transaction(
    conn: &Connection,
    date: &str,
    description: &str,
    postings: &[PostingInput],
) -> Result<String, AppError> {
    if postings.len() < 2 {
        return Err(AppError::ValidationError(
            "errors.transactionMinPostings".to_string(),
        ));
    }

    // i128 avoids release-mode wrapping addition if a malformed amount ever
    // reaches this point (amounts are bounded by utils::money).
    let sum: i128 = postings.iter().map(|p| i128::from(p.amount)).sum();
    if sum != 0 {
        return Err(AppError::ValidationError(
            "errors.transactionUnbalanced".to_string(),
        ));
    }

    let account_ids: Vec<String> = postings.iter().map(|p| p.account_id.clone()).collect();

    validate_accounts_active(conn, &account_ids)?;
    validate_no_equity_accounts(conn, &account_ids)?;

    let unique_account_ids: std::collections::HashSet<&str> =
        postings.iter().map(|p| p.account_id.as_str()).collect();
    if unique_account_ids.len() != postings.len() {
        return Err(AppError::ValidationError(
            "errors.transaction.duplicateAccount".to_string(),
        ));
    }

    let tx_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();

    conn.execute(
        &format!(
            "INSERT INTO transactions ({}) VALUES (?1, ?2, ?3, ?4, ?4)",
            TRANSACTION_COLUMNS
        ),
        rusqlite::params![&tx_id, date, description, &now],
    )?;

    for (seq, posting) in postings.iter().enumerate() {
        let posting_id = Uuid::new_v4().to_string();
        conn.execute(
            &format!(
                "INSERT INTO postings ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                POSTING_COLUMNS
            ),
            rusqlite::params![
                &posting_id,
                &tx_id,
                &posting.account_id,
                posting.amount,
                seq as i32,
                &now
            ],
        )?;
    }

    Ok(tx_id)
}

// ---------------------------------------------------------------------------
// Read operations (to be implemented)
// ---------------------------------------------------------------------------
