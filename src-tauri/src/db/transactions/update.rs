//! Updating an existing transaction.

use rusqlite::Connection;
use uuid::Uuid;

use crate::db::AppError;

use super::query::get_transaction_detail;
use super::types::{PostingInput, TransactionDetail, UpdateTransactionInput};
use super::validate::{validate_accounts_active, validate_no_equity_accounts};

#[allow(dead_code)]
pub fn update_transaction(
    conn: &mut Connection,
    tx_id: &str,
    input: &UpdateTransactionInput,
) -> Result<TransactionDetail, AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
        rusqlite::params![tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound(
            "errors.transaction.notFound".to_string(),
        ));
    }

    // A reconciled transaction is frozen: changing it would silently break the
    // statement it was matched against.
    crate::db::reconcile::assert_not_reconciled(conn, tx_id)?;

    if input.date.is_none() && input.description.is_none() && input.postings.is_none() {
        return Err(AppError::ValidationError(
            "errors.transaction.noChanges".to_string(),
        ));
    }

    let parsed_postings: Option<Vec<PostingInput>> = if let Some(ref postings) = input.postings {
        if postings.len() < 2 {
            return Err(AppError::ValidationError(
                "errors.transactionMinPostings".to_string(),
            ));
        }

        let total: i64 = postings.iter().map(|p| p.amount).sum();
        if total != 0 {
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

        Some(postings.clone())
    } else {
        None
    };

    let tx = conn.transaction()?;
    let now = crate::utils::time::now_rfc3339();

    if let Some(ref date) = input.date {
        tx.execute(
            "UPDATE transactions SET date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![date, &now, tx_id],
        )?;
    }

    if let Some(ref desc) = input.description {
        tx.execute(
            "UPDATE transactions SET description = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![desc, &now, tx_id],
        )?;
    }

    if let Some(ref new_postings) = parsed_postings {
        tx.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;

        for (seq, posting) in new_postings.iter().enumerate() {
            let posting_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &posting_id,
                    tx_id,
                    &posting.account_id,
                    posting.amount,
                    seq as i32,
                    &now
                ],
            )?;
        }
    }

    tx.execute(
        "UPDATE transactions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, tx_id],
    )?;

    tx.commit()?;

    get_transaction_detail(conn, tx_id)
}

// ---------------------------------------------------------------------------
// Delete Transaction Operations
// ---------------------------------------------------------------------------
