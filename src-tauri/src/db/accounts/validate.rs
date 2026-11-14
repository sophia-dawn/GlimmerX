//! Shared validation for balance-sheet endpoints.
//!
//! Extracted from `transfer.rs` (design doc 25, P0-1): both transfer write
//! paths — `db::accounts::transfer::create_transfer` and the quick-add
//! transfer branch — must enforce the same endpoint rule, otherwise the
//! weaker path lets money appear out of a P&L account.

use rusqlite::{Connection, OptionalExtension};

use crate::db::AppError;

/// A transfer moves money between balance-sheet accounts only. Posting to a
/// P&L account here would create profit out of nothing, and posting to equity
/// would bypass `validate_no_equity_accounts`.
///
/// Also rejects missing and inactive accounts.
pub(in crate::db) fn validate_endpoint(
    conn: &Connection,
    account_id: &str,
) -> Result<(), AppError> {
    let row: Option<(String, bool)> = conn
        .query_row(
            "SELECT type, is_active FROM accounts WHERE id = ?1",
            [account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    match row {
        Some((kind, true)) if kind == "asset" || kind == "liability" => Ok(()),
        _ => Err(AppError::ValidationError(
            "errors.account.transferAccountInvalid".into(),
        )),
    }
}
