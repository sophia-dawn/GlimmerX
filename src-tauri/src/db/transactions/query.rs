//! Read paths: listings, single transaction lookups and detail views.

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension};

use crate::db::AppError;

use super::types::{
    row_to_posting, row_to_transaction, PostingDetail, PostingRecord, TransactionDetail,
    TransactionRecord, TransactionWithPostings, POSTING_COLUMNS, TRANSACTION_COLUMNS,
};
use super::validate::format_amount_display;

/// Batch-load all postings for given transaction IDs.
/// Returns a HashMap mapping transaction_id -> Vec<PostingRecord>.
fn batch_load_postings(
    conn: &Connection,
    transaction_ids: &[String],
) -> Result<HashMap<String, Vec<PostingRecord>>, AppError> {
    if transaction_ids.is_empty() {
        return Ok(HashMap::new());
    }

    // Build IN clause with placeholders
    let placeholders: Vec<String> = transaction_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT {} FROM postings WHERE transaction_id IN ({}) ORDER BY transaction_id, sequence",
        POSTING_COLUMNS,
        placeholders.join(",")
    );

    let mut stmt = conn.prepare(&sql)?;

    let params: Vec<&dyn rusqlite::types::ToSql> = transaction_ids
        .iter()
        .map(|id| id as &dyn rusqlite::types::ToSql)
        .collect();

    let postings: Vec<PostingRecord> = stmt
        .query_map(rusqlite::params_from_iter(params), row_to_posting)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut map: HashMap<String, Vec<PostingRecord>> = HashMap::new();
    for posting in postings {
        map.entry(posting.transaction_id.clone())
            .or_default()
            .push(posting);
    }

    Ok(map)
}

/// List all non-deleted transactions with their postings.
///
/// Optionally filter by account_id or date range.
#[allow(dead_code)]
pub fn list_transactions(
    conn: &Connection,
    account_id: Option<&str>,
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<Vec<TransactionWithPostings>, AppError> {
    // `WHERE 1 = 1` keeps the optional filters below appendable as plain "AND".
    let mut sql_parts: Vec<String> = vec![format!(
        "SELECT {} FROM transactions WHERE 1 = 1",
        TRANSACTION_COLUMNS
    )];
    let mut param_values: Vec<String> = Vec::new();

    if let Some(acc) = account_id {
        sql_parts.push(
            " AND id IN (SELECT DISTINCT transaction_id FROM postings WHERE account_id = ?)"
                .to_string(),
        );
        param_values.push(acc.to_string());
    }
    if let Some(from) = from_date {
        sql_parts.push(" AND date >= ?".to_string());
        param_values.push(from.to_string());
    }
    if let Some(to) = to_date {
        sql_parts.push(" AND date <= ?".to_string());
        param_values.push(to.to_string());
    }
    sql_parts.push(" ORDER BY date DESC, id DESC".to_string());

    let sql = sql_parts.join("");
    let mut stmt = conn.prepare(&sql)?;

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    let transactions: Vec<TransactionRecord> = stmt
        .query_map(rusqlite::params_from_iter(param_refs), row_to_transaction)?
        .collect::<Result<Vec<_>, _>>()?;

    let tx_ids: Vec<String> = transactions.iter().map(|t| t.id.clone()).collect();
    let postings_map = batch_load_postings(conn, &tx_ids)?;

    let result: Vec<TransactionWithPostings> = transactions
        .into_iter()
        .map(|tx| {
            let postings = postings_map.get(&tx.id).cloned().unwrap_or_default();
            TransactionWithPostings {
                transaction: tx,
                postings,
            }
        })
        .collect();

    Ok(result)
}

/// Get a single transaction with all its postings by ID.
///
/// Returns None if transaction doesn't exist or is deleted.
#[allow(dead_code)]
pub fn get_transaction_with_postings(
    conn: &Connection,
    id: &str,
) -> Result<Option<TransactionWithPostings>, AppError> {
    let tx = conn
        .query_row(
            &format!(
                "SELECT {} FROM transactions WHERE id = ?1",
                TRANSACTION_COLUMNS
            ),
            [id],
            row_to_transaction,
        )
        .optional()?;

    match tx {
        Some(transaction) => {
            let postings = get_postings_for_transaction(conn, &transaction.id)?;
            Ok(Some(TransactionWithPostings {
                transaction,
                postings,
            }))
        }
        None => Ok(None),
    }
}

/// Get all postings for a transaction, ordered by sequence.
#[allow(dead_code)]
fn get_postings_for_transaction(
    conn: &Connection,
    transaction_id: &str,
) -> Result<Vec<PostingRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM postings WHERE transaction_id = ?1 ORDER BY sequence",
        POSTING_COLUMNS
    ))?;

    let postings = stmt
        .query_map([transaction_id], row_to_posting)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(postings)
}

/// Get transaction detail with full posting information.
#[allow(dead_code)]
pub fn get_transaction_detail(
    conn: &Connection,
    tx_id: &str,
) -> Result<TransactionDetail, AppError> {
    let tx_data = conn
        .query_row(
            "SELECT date, description, category_id, created_at, updated_at 
             FROM transactions WHERE id = ?1",
            [tx_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional();

    let (date, description, category_id, created_at, updated_at) = match tx_data {
        Ok(Some(data)) => data,
        Ok(None) => {
            return Err(AppError::NotFound(
                "errors.transaction.notFound".to_string(),
            ));
        }
        Err(e) => {
            return Err(e.into());
        }
    };

    let category_name = if let Some(ref cat_id) = category_id {
        conn.query_row(
            "SELECT name FROM categories WHERE id = ?1",
            [cat_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
    } else {
        None
    };

    let postings: Vec<PostingDetail> = conn
        .prepare(
            "SELECT p.id, p.transaction_id, p.account_id, a.name, a.type, p.amount, p.sequence, p.created_at
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             WHERE p.transaction_id = ?1
             ORDER BY p.amount ASC",
        )?
        .query_map([tx_id], |row| {
            let amount: i64 = row.get(5)?;
            let is_debit = amount > 0;
            let amount_display = format_amount_display(amount);

            Ok(PostingDetail {
                id: row.get(0)?,
                transaction_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,
                account_type: row.get(4)?,
                amount,
                amount_display,
                is_debit,
                sequence: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let debit_total: i64 = postings
        .iter()
        .filter(|p| p.is_debit)
        .map(|p| p.amount)
        .sum();
    let credit_total: i64 = postings
        .iter()
        .filter(|p| !p.is_debit)
        .map(|p| p.amount)
        .sum();
    let is_balanced = debit_total + credit_total == 0;
    let posting_count = postings.len() as u32;
    Ok(TransactionDetail {
        id: tx_id.to_string(),
        date,
        description,
        category_id,
        category_name,
        postings,
        created_at,
        updated_at,
        is_balanced,
        posting_count,
        debit_total,
        credit_total,
    })
}
