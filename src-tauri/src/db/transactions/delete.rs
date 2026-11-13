//! Hard-deleting a transaction and previewing the impact.

use rusqlite::{Connection, OptionalExtension};

use crate::db::AppError;

use super::types::DeletePreview;

/// Get delete preview information.
#[allow(dead_code)]
pub fn get_delete_preview(conn: &Connection, tx_id: &str) -> Result<DeletePreview, AppError> {
    let (description, date): (String, String) = conn
        .query_row(
            "SELECT description, date FROM transactions WHERE id = ?1",
            [tx_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("errors.transaction.notFound".to_string()))?;

    let posting_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
        [tx_id],
        |row| row.get(0),
    )?;

    // `delete_transaction` refuses a reconciled transaction; saying "can delete"
    // and then failing after the confirmation is a worse answer than saying so up
    // front.
    let locked = crate::db::reconcile::assert_not_reconciled(conn, tx_id).is_err();

    Ok(DeletePreview {
        transaction_id: tx_id.to_string(),
        description,
        date,
        posting_count,
        can_delete: !locked,
        warning_message: locked
            .then(|| crate::errors::message("errors.reconcile.transactionLocked").to_string()),
    })
}

/// Delete transaction permanently.
#[allow(dead_code)]
pub fn delete_transaction(conn: &mut Connection, tx_id: &str) -> Result<(), AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
        [tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound(
            "errors.transaction.notFound".to_string(),
        ));
    }

    // Same rule as editing: the reconciliation it belongs to is a statement,
    // and deleting entries behind its back would leave it internally wrong.
    crate::db::reconcile::assert_not_reconciled(conn, tx_id)?;

    // Use atomic transaction so all deletions succeed or fail together
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;
    tx.execute("DELETE FROM transactions WHERE id = ?1", [tx_id])?;
    tx.commit()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Paginated Transaction List Implementation
// ---------------------------------------------------------------------------
