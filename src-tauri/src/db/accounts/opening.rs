//! Opening-balance transactions for accounts.

use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::AppError;

use super::crud::{delete_opening_balance, find_account_by_type_and_name, insert_account};
use super::types::AccountRecord;

/// Create or update an Opening Balance transaction for the given account.
/// If an opening balance transaction already exists for this account, it is updated.
pub fn create_opening_balance(
    conn: &Connection,
    account_id: &str,
    amount_cents: i64,
    equity_name: &str,
    opening_balances_name: &str,
    date: Option<&str>,
) -> Result<(), AppError> {
    // If amount is 0, delete any existing opening balance transaction
    if amount_cents == 0 {
        delete_opening_balance(conn, account_id)?;
        return Ok(());
    }

    let tx_date = date
        .unwrap_or(&crate::utils::time::today_date())
        .to_string();
    let now = crate::utils::time::now_rfc3339();

    // Get account type to determine sign for liability accounts
    // Liability accounts store negative balances (negative = money owed)
    let account_type: String = conn.query_row(
        "SELECT type FROM accounts WHERE id = ?1",
        rusqlite::params![account_id],
        |row| row.get(0),
    )?;
    let posting_amount = if account_type == "liability" {
        -amount_cents // Liability: negative balance = debt owed
    } else {
        amount_cents // Asset/other: positive balance = money owned
    };

    let equity_id = ensure_opening_balance_equity(conn, equity_name, opening_balances_name)?;

    // Check for existing opening balance transaction
    let existing_tx: Option<String> = conn
        .query_row(
            "SELECT t.id FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             WHERE t.description = 'Opening Balance' AND p.account_id = ?1
             LIMIT 1",
            rusqlite::params![account_id],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(tx_id) = existing_tx {
        // Update the existing transaction
        conn.execute(
            "UPDATE transactions SET date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![&tx_date, &now, &tx_id],
        )?;

        // Update the posting amounts for this account
        conn.execute(
            "UPDATE postings SET amount = ?1 WHERE transaction_id = ?2 AND account_id = ?3",
            rusqlite::params![posting_amount, &tx_id, account_id],
        )?;

        // Update the equity posting (opposite sign)
        conn.execute(
            "UPDATE postings SET amount = ?1 WHERE transaction_id = ?2 AND account_id = ?3",
            rusqlite::params![-posting_amount, &tx_id, &equity_id],
        )?;
    } else {
        // Create new opening balance transaction
        let tx_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![&tx_id, &tx_date, "Opening Balance", &now],
        )?;

        let posting1_id = Uuid::new_v4().to_string();
        let posting2_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&posting1_id, &tx_id, account_id, posting_amount, &now],
        )?;

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&posting2_id, &tx_id, &equity_id, -posting_amount, &now],
        )?;
    }

    Ok(())
}

/// Ensure a flat "Equity: Opening Balances" account exists.
/// Returns the opening balances account ID.
fn ensure_opening_balance_equity(
    conn: &Connection,
    equity_name: &str,
    opening_balances_name: &str,
) -> Result<String, AppError> {
    let full_name = format!("{}: {}", equity_name, opening_balances_name);
    if let Some(existing) = find_account_by_type_and_name(conn, "equity", &full_name)? {
        return Ok(existing.id);
    }
    let acct = AccountRecord {
        id: String::new(),
        name: full_name,
        account_type: "equity".to_string(),
        description: String::new(),
        account_number: None,
        is_system: true,
        iban: None,
        is_active: true,
        include_net_worth: false,
        created_at: String::new(),
        updated_at: String::new(),
    };
    insert_account(conn, &acct)
}
