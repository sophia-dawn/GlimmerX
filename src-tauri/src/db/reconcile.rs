//! Reconciliation: matching an account's postings against a statement.
//!
//! The unit is a **posting**, not a transaction. A transfer out of the current
//! account may already be ticked on the other side's statement, so marking the
//! whole transaction reconciled — which is what `transactions.is_reconciled`
//! did before v2 — is wrong by construction.
//!
//! Invariant: `SUM(amount) over cleared postings == closing balance` after a
//! reconciliation. `opening_balance` is recorded as the cleared sum *before*
//! the run, so the history stays readable even after later reconciliations.

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// One unreconciled posting, as offered to the user for ticking.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileCandidate {
    pub posting_id: String,
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    /// Signed: positive increases the account balance.
    pub amount: i64,
}

/// Everything the reconciliation screen needs before the user ticks anything.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcilePreview {
    pub account_id: String,
    pub statement_date: String,
    /// Balance already reconciled for this account.
    pub opening_balance: i64,
    /// Postings with `cleared = 0` and `date <= statement_date`, oldest first.
    pub candidates: Vec<ReconcileCandidate>,
}

/// Running totals for the ticking screen.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileTotals {
    pub opening_balance: i64,
    pub selected_total: i64,
    pub expected_balance: i64,
    pub difference: i64,
    pub balanced: bool,
}

/// A completed reconciliation run.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationRecord {
    pub id: String,
    pub account_id: String,
    pub statement_date: String,
    pub opening_balance: i64,
    pub closing_balance: i64,
    pub created_at: String,
}

/// Result of pre-ticking entries against a pasted list of statement amounts.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoMatchResult {
    /// Posting ids to pre-tick, in statement-date order.
    pub matched: Vec<String>,
    /// Statement amounts that found no posting.
    pub unmatched_targets: usize,
}

// ---------------------------------------------------------------------------
// Read
// ---------------------------------------------------------------------------

/// Balance already reconciled for the account (sum of cleared postings).
pub fn cleared_balance(conn: &Connection, account_id: &str) -> Result<i64, AppError> {
    let total = conn.query_row(
        "SELECT COALESCE(SUM(p.amount), 0)
           FROM postings p
          WHERE p.account_id = ?1 AND p.cleared = 1",
        [account_id],
        |row| row.get(0),
    )?;
    Ok(total)
}

/// Postings still open for reconciliation up to `statement_date`.
pub fn candidates(
    conn: &Connection,
    account_id: &str,
    statement_date: &str,
) -> Result<Vec<ReconcileCandidate>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.transaction_id, t.date, t.description, p.amount
           FROM postings p
           JOIN transactions t ON t.id = p.transaction_id
          WHERE p.account_id = ?1
            AND p.cleared = 0
            AND t.date <= ?2
          ORDER BY t.date, p.id",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![account_id, statement_date], |row| {
            Ok(ReconcileCandidate {
                posting_id: row.get(0)?,
                transaction_id: row.get(1)?,
                date: row.get(2)?,
                description: row.get(3)?,
                amount: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Build the preview payload for one account.
pub fn preview(
    conn: &Connection,
    account_id: &str,
    statement_date: &str,
) -> Result<ReconcilePreview, AppError> {
    ensure_reconcilable(conn, account_id)?;
    Ok(ReconcilePreview {
        account_id: account_id.to_string(),
        statement_date: statement_date.to_string(),
        opening_balance: cleared_balance(conn, account_id)?,
        candidates: candidates(conn, account_id, statement_date)?,
    })
}

/// Past reconciliations for an account, newest first.
pub fn list_reconciliations(
    conn: &Connection,
    account_id: &str,
) -> Result<Vec<ReconciliationRecord>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, account_id, statement_date, opening_balance, closing_balance, created_at
           FROM reconciliations
          WHERE account_id = ?1
          ORDER BY statement_date DESC",
    )?;
    let rows = stmt
        .query_map([account_id], |row| {
            Ok(ReconciliationRecord {
                id: row.get(0)?,
                account_id: row.get(1)?,
                statement_date: row.get(2)?,
                opening_balance: row.get(3)?,
                closing_balance: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------------------------------------------------------------------------
// Auto-match
// ---------------------------------------------------------------------------

/// Distance in days between two `YYYY-MM-DD` dates, plus whether `date` is on
/// or before `reference`. Unparseable input sorts last instead of panicking.
fn date_key(date: &str, reference: &str) -> (u8, i64) {
    use chrono::NaiveDate;
    let parsed = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok();
    let target = NaiveDate::parse_from_str(reference, "%Y-%m-%d").ok();
    match (parsed, target) {
        (Some(d), Some(r)) => {
            let delta = (d - r).num_days();
            // On/before the statement date first: a posting dated after the
            // statement cannot be on it.
            (if delta <= 0 { 0 } else { 1 }, delta.abs())
        }
        _ => (2, i64::MAX),
    }
}

/// Pre-select postings for a list of statement amounts.
///
/// Rules, in order: equal amount, never the same posting twice, then the
/// closest date preferring on-or-before the statement date. Pure function so
/// the greedy behaviour is testable without a database.
pub fn auto_match(
    candidates: &[ReconcileCandidate],
    targets: &[i64],
    statement_date: &str,
) -> AutoMatchResult {
    let mut used = vec![false; candidates.len()];
    let mut matched_indices: Vec<usize> = Vec::new();
    let mut unmatched_targets = 0usize;

    for target in targets {
        let best = candidates
            .iter()
            .enumerate()
            .filter(|(index, candidate)| !used[*index] && candidate.amount == *target)
            .min_by(|(_, a), (_, b)| {
                date_key(&a.date, statement_date)
                    .cmp(&date_key(&b.date, statement_date))
                    .then_with(|| a.posting_id.cmp(&b.posting_id))
            })
            .map(|(index, _)| index);

        match best {
            Some(index) => {
                used[index] = true;
                matched_indices.push(index);
            }
            None => unmatched_targets += 1,
        }
    }

    // Return in date order so the UI can render the ticks top-down.
    matched_indices.sort_by(|a, b| {
        candidates[*a]
            .date
            .cmp(&candidates[*b].date)
            .then_with(|| candidates[*a].posting_id.cmp(&candidates[*b].posting_id))
    });

    AutoMatchResult {
        matched: matched_indices
            .into_iter()
            .map(|index| candidates[index].posting_id.clone())
            .collect(),
        unmatched_targets,
    }
}

// ---------------------------------------------------------------------------
// Write
// ---------------------------------------------------------------------------

fn ensure_reconcilable(conn: &Connection, account_id: &str) -> Result<(), AppError> {
    let account_type: Option<String> = conn
        .query_row(
            "SELECT type FROM accounts WHERE id = ?1",
            [account_id],
            |row| row.get(0),
        )
        .optional()?;
    match account_type.as_deref() {
        None => Err(AppError::NotFound("errors.accountNotFound".to_string())),
        Some("asset") | Some("liability") => Ok(()),
        Some(_) => Err(AppError::ValidationError(
            "errors.reconcile.accountNotReconcilable".to_string(),
        )),
    }
}

/// Validate a candidate selection and return its signed total.
///
/// A posting is only acceptable here if it belongs to the account and is still
/// open; anything else means the screen is stale.
fn selected_total(
    conn: &Connection,
    account_id: &str,
    posting_ids: &[String],
) -> Result<i64, AppError> {
    let mut total: i64 = 0;
    for id in posting_ids {
        let row: Option<(i64, i64)> = conn
            .query_row(
                "SELECT amount, cleared FROM postings WHERE id = ?1 AND account_id = ?2",
                rusqlite::params![id, account_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        match row {
            Some((amount, 0)) => total += amount,
            _ => {
                return Err(AppError::ValidationError(
                    "errors.reconcile.postingNotAvailable".to_string(),
                ))
            }
        }
    }
    Ok(total)
}

/// Running totals for a tentative selection.
///
/// Exists as a backend call so the UI never adds money itself: every tick
/// change re-asks, and the difference that gates submission is computed once,
/// from the same integers the write will use.
pub fn totals(
    conn: &Connection,
    account_id: &str,
    posting_ids: &[String],
    closing_balance: i64,
) -> Result<ReconcileTotals, AppError> {
    let opening_balance = cleared_balance(conn, account_id)?;
    let selected_total = selected_total(conn, account_id, posting_ids)?;
    let expected_balance = opening_balance + selected_total;
    let difference = expected_balance - closing_balance;
    Ok(ReconcileTotals {
        opening_balance,
        selected_total,
        expected_balance,
        difference,
        balanced: difference == 0,
    })
}

/// Record a reconciliation and tick the selected postings.
///
/// All-or-nothing: the statement row and the `cleared` flags share one
/// transaction, so a failure can never leave postings marked against a
/// statement that does not exist.
pub fn create_reconciliation(
    conn: &Connection,
    account_id: &str,
    statement_date: &str,
    closing_balance: i64,
    posting_ids: &[String],
) -> Result<String, AppError> {
    if posting_ids.is_empty() {
        return Err(AppError::ValidationError(
            "errors.reconcile.emptySelection".to_string(),
        ));
    }
    ensure_reconcilable(conn, account_id)?;

    let tx = conn.unchecked_transaction()?;

    let duplicate: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM reconciliations WHERE account_id = ?1 AND statement_date = ?2)",
        rusqlite::params![account_id, statement_date],
        |row| row.get(0),
    )?;
    if duplicate {
        return Err(AppError::ValidationError(
            "errors.reconcile.duplicateDate".to_string(),
        ));
    }

    let opening_balance = cleared_balance(&tx, account_id)?;
    let selected = selected_total(&tx, account_id, posting_ids)?;
    if opening_balance + selected != closing_balance {
        return Err(AppError::ValidationError(
            "errors.reconcile.differenceNotZero".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    tx.execute(
        "INSERT INTO reconciliations
             (id, account_id, statement_date, opening_balance, closing_balance, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &id,
            account_id,
            statement_date,
            opening_balance,
            closing_balance,
            &now
        ],
    )?;

    for posting_id in posting_ids {
        tx.execute(
            "UPDATE postings SET cleared = 1, reconcile_id = ?1 WHERE id = ?2",
            rusqlite::params![&id, posting_id],
        )?;
    }

    tx.commit()?;
    Ok(id)
}

/// Undo a reconciliation: release its postings and delete the statement row.
pub fn undo_reconciliation(conn: &Connection, reconciliation_id: &str) -> Result<(), AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM reconciliations WHERE id = ?1)",
        [reconciliation_id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(AppError::NotFound("errors.reconcile.notFound".to_string()));
    }

    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE postings SET cleared = 0, reconcile_id = NULL WHERE reconcile_id = ?1",
        [reconciliation_id],
    )?;
    tx.execute(
        "DELETE FROM reconciliations WHERE id = ?1",
        [reconciliation_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// Guard for edit/delete paths: a reconciled transaction is frozen.
pub fn assert_not_reconciled(conn: &Connection, transaction_id: &str) -> Result<(), AppError> {
    let reconciled: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM postings WHERE transaction_id = ?1 AND reconcile_id IS NOT NULL)",
        [transaction_id],
        |row| row.get(0),
    )?;
    if reconciled {
        return Err(AppError::ValidationError(
            "errors.reconcile.transactionLocked".to_string(),
        ));
    }
    Ok(())
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
        crate::db::init_schema(&conn).unwrap();
        (dir, conn)
    }

    fn make_account(conn: &Connection, name: &str, kind: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system,
                                   is_active, include_net_worth, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'CNY', '', 0, 1, 0, ?4, ?4)",
            rusqlite::params![&id, name, kind, &now],
        )
        .unwrap();
        id
    }

    /// Insert a transaction with one posting on `account_id` (plus an offsetting
    /// posting so the ledger stays balanced) and return the posting id.
    fn make_posting(
        conn: &Connection,
        account_id: &str,
        counter_account: &str,
        date: &str,
        amount: i64,
    ) -> String {
        let now = crate::utils::time::now_rfc3339();
        let tx_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES (?1, ?2, 'test', ?3, ?3)",
            rusqlite::params![&tx_id, date, &now],
        )
        .unwrap();
        let posting_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES (?1, ?2, ?3, ?4, 0, ?5)",
            rusqlite::params![&posting_id, &tx_id, account_id, amount, &now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            rusqlite::params![
                Uuid::new_v4().to_string(),
                &tx_id,
                counter_account,
                -amount,
                &now
            ],
        )
        .unwrap();
        posting_id
    }

    fn candidate(id: &str, date: &str, amount: i64) -> ReconcileCandidate {
        ReconcileCandidate {
            posting_id: id.to_string(),
            transaction_id: format!("tx-{id}"),
            date: date.to_string(),
            description: "x".to_string(),
            amount,
        }
    }

    #[test]
    fn test_auto_match_pairs_equal_amounts_by_nearest_date() {
        let candidates = vec![
            candidate("a", "2024-01-05", 1000),
            candidate("b", "2024-01-01", 1000),
            candidate("c", "2024-01-03", 2000),
        ];

        // `b` wins over the closer `a` only because it is on or before the
        // statement date — a posting dated afterwards cannot be on it.
        let result = auto_match(&candidates, &[1000, 2000], "2024-01-04");
        assert_eq!(result.matched, vec!["b".to_string(), "c".to_string()]);
        assert_eq!(result.unmatched_targets, 0);
    }

    #[test]
    fn test_auto_match_prefers_date_before_statement() {
        let candidates = vec![
            candidate("after", "2024-01-06", 500),
            candidate("before", "2024-01-02", 500),
        ];
        let result = auto_match(&candidates, &[500], "2024-01-05");
        assert_eq!(result.matched, vec!["before".to_string()]);
    }

    #[test]
    fn test_auto_match_never_reuses_a_posting() {
        let candidates = vec![candidate("only", "2024-01-02", 700)];
        let result = auto_match(&candidates, &[700, 700], "2024-01-05");
        assert_eq!(result.matched, vec!["only".to_string()]);
        assert_eq!(result.unmatched_targets, 1);
    }

    #[test]
    fn test_auto_match_ignores_unmatched_amounts() {
        let candidates = vec![candidate("a", "2024-01-02", 300)];
        let result = auto_match(&candidates, &[999], "2024-01-05");
        assert!(result.matched.is_empty());
        assert_eq!(result.unmatched_targets, 1);
    }

    #[test]
    fn test_auto_match_handles_bad_date_input() {
        let candidates = vec![candidate("bad", "not-a-date", 100)];
        let result = auto_match(&candidates, &[100], "2024-01-05");
        assert_eq!(result.matched, vec!["bad".to_string()]);
    }

    #[test]
    fn test_preview_lists_open_postings_only() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");

        make_posting(&conn, &bank, &food, "2024-01-02", -1000);
        make_posting(&conn, &bank, &food, "2024-01-10", -2000);
        make_posting(&conn, &bank, &food, "2024-02-01", -3000);

        let preview = preview(&conn, &bank, "2024-01-31").unwrap();
        assert_eq!(preview.opening_balance, 0);
        assert_eq!(
            preview.candidates.len(),
            2,
            "postings after the date are excluded"
        );
        assert_eq!(preview.candidates[0].amount, -1000);
    }

    #[test]
    fn test_reconcile_create_and_undo() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");

        let first = make_posting(&conn, &bank, &food, "2024-01-02", -1000);
        make_posting(&conn, &bank, &food, "2024-01-05", -2000);

        // Opening 0 + (-1000) = -1000.
        let id = create_reconciliation(&conn, &bank, "2024-01-31", -1000, &[first]).unwrap();

        assert_eq!(cleared_balance(&conn, &bank).unwrap(), -1000);
        let rows = list_reconciliations(&conn, &bank).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].opening_balance, 0);
        assert_eq!(rows[0].closing_balance, -1000);

        undo_reconciliation(&conn, &id).unwrap();
        assert_eq!(cleared_balance(&conn, &bank).unwrap(), 0);
        assert!(list_reconciliations(&conn, &bank).unwrap().is_empty());
        let cleared: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM postings WHERE cleared = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(cleared, 0);
    }

    #[test]
    fn test_reconcile_requires_zero_difference() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");
        let first = make_posting(&conn, &bank, &food, "2024-01-02", -1000);

        let err =
            create_reconciliation(&conn, &bank, "2024-01-31", -900, &[first.clone()]).unwrap_err();
        assert!(format!("{err}").contains("差额"));

        // Nothing was written.
        assert!(list_reconciliations(&conn, &bank).unwrap().is_empty());
        assert_eq!(cleared_balance(&conn, &bank).unwrap(), 0);
    }

    #[test]
    fn test_reconcile_rejects_empty_and_duplicate() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");
        let first = make_posting(&conn, &bank, &food, "2024-01-02", -1000);

        assert!(create_reconciliation(&conn, &bank, "2024-01-31", 0, &[]).is_err());

        create_reconciliation(&conn, &bank, "2024-01-31", -1000, &[first]).unwrap();
        let err = create_reconciliation(&conn, &bank, "2024-01-31", -1000, &["whatever".into()])
            .unwrap_err();
        assert!(format!("{err}").contains("对账单"));
    }

    #[test]
    fn test_reconcile_rejects_foreign_or_already_cleared_posting() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let wallet = make_account(&conn, "钱包", "asset");
        let food = make_account(&conn, "餐饮", "expense");

        let bank_posting = make_posting(&conn, &bank, &food, "2024-01-02", -1000);
        let wallet_posting = make_posting(&conn, &wallet, &food, "2024-01-02", -500);

        // Posting of another account.
        let err =
            create_reconciliation(&conn, &bank, "2024-01-31", -500, &[wallet_posting]).unwrap_err();
        assert!(format!("{err}").contains("分录"));

        // Posting already reconciled.
        create_reconciliation(&conn, &bank, "2024-01-31", -1000, &[bank_posting.clone()]).unwrap();
        let err =
            create_reconciliation(&conn, &bank, "2024-02-29", -1000, &[bank_posting]).unwrap_err();
        assert!(format!("{err}").contains("分录"));
    }

    #[test]
    fn test_reconcile_only_for_balance_sheet_accounts() {
        let (_dir, conn) = test_conn();
        let food = make_account(&conn, "餐饮", "expense");
        let bank = make_account(&conn, "银行", "asset");
        let posting = make_posting(&conn, &food, &bank, "2024-01-02", 1000);

        let err = create_reconciliation(&conn, &food, "2024-01-31", 1000, &[posting]).unwrap_err();
        assert!(format!("{err}").contains("资产"));

        assert!(preview(&conn, &food, "2024-01-31").is_err());
        assert!(preview(&conn, "missing", "2024-01-31").is_err());
    }

    #[test]
    fn test_second_reconciliation_opens_at_previous_closing() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");

        let jan = make_posting(&conn, &bank, &food, "2024-01-02", -1000);
        create_reconciliation(&conn, &bank, "2024-01-31", -1000, &[jan]).unwrap();

        let feb = make_posting(&conn, &bank, &food, "2024-02-02", -500);
        let preview_feb = preview(&conn, &bank, "2024-02-29").unwrap();
        assert_eq!(preview_feb.opening_balance, -1000);

        create_reconciliation(&conn, &bank, "2024-02-29", -1500, &[feb]).unwrap();
        let rows = list_reconciliations(&conn, &bank).unwrap();
        assert_eq!(rows[0].opening_balance, -1000);
        assert_eq!(rows[0].closing_balance, -1500);
        assert_eq!(rows[1].closing_balance, -1000);
    }

    #[test]
    fn test_assert_not_reconciled_blocks_edits() {
        let (_dir, conn) = test_conn();
        let bank = make_account(&conn, "银行", "asset");
        let food = make_account(&conn, "餐饮", "expense");
        let posting = make_posting(&conn, &bank, &food, "2024-01-02", -1000);

        let transaction_id: String = conn
            .query_row(
                "SELECT transaction_id FROM postings WHERE id = ?1",
                [&posting],
                |row| row.get(0),
            )
            .unwrap();

        assert!(assert_not_reconciled(&conn, &transaction_id).is_ok());
        create_reconciliation(&conn, &bank, "2024-01-31", -1000, &[posting]).unwrap();
        let err = assert_not_reconciled(&conn, &transaction_id).unwrap_err();
        assert!(format!("{err}").contains("已核对"));
    }

    #[test]
    fn test_undo_missing_reconciliation() {
        let (_dir, conn) = test_conn();
        assert!(undo_reconciliation(&conn, "nope").is_err());
    }
}
