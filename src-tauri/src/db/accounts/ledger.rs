//! Per-account transaction listing.

use rusqlite::Connection;

use crate::db::AppError;

use super::types::TransactionRecord;

/// Get all transactions for an account, optionally filtered by date range.
pub fn get_account_transactions(
    conn: &Connection,
    account_id: &str,
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<Vec<TransactionRecord>, AppError> {
    let base_sql = "SELECT t.id, t.date, t.description, p.amount, t.created_at, t.updated_at
                    FROM transactions t
                    JOIN postings p ON p.transaction_id = t.id AND p.account_id = ?1";

    let mut sql = base_sql.to_string();

    if from_date.is_some() {
        sql.push_str(" AND t.date >= ?2");
    }
    if to_date.is_some() {
        sql.push_str(if from_date.is_some() {
            " AND t.date <= ?3"
        } else {
            " AND t.date <= ?2"
        });
    }
    sql.push_str(" ORDER BY t.date DESC, t.id DESC");

    let mut stmt = conn.prepare(&sql)?;

    let transactions: Vec<TransactionRecord> = match (from_date, to_date) {
        (Some(from), Some(to)) => stmt
            .query_map(rusqlite::params![account_id, from, to], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    amount: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (Some(from), None) => stmt
            .query_map(rusqlite::params![account_id, from], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    amount: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (None, Some(to)) => stmt
            .query_map(rusqlite::params![account_id, to], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    amount: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
        (None, None) => stmt
            .query_map(rusqlite::params![account_id], |row| {
                Ok(TransactionRecord {
                    id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    amount: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?,
    };

    Ok(transactions)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
