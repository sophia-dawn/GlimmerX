//! Export module for CSV and Beancount formats.
//!
//! **Performance optimizations**:
//! - CSV export: streaming (row-by-row) to avoid loading all data into memory
//! - Beancount export: lightweight query for headers, then streaming for transactions
//! - BufWriter for all file writes to reduce syscalls
//! - Leverages SQL ORDER BY for transaction grouping (no redundant in-memory sort)

#![allow(dead_code)]

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Result of an export operation.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub transaction_count: u32,
    pub posting_count: u32,
}

/// Transaction row for CSV/Beancount export.
#[derive(Debug)]
pub struct ExportRow {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub is_reconciled: bool,
    pub currency: String,
    pub account_name: String,
    pub account_type: String,
    pub amount: i64,
    pub category_name: Option<String>,
}

/// Export transactions to CSV format.
/// Uses streaming: writes each row immediately without loading all into memory.
pub fn export_csv(
    conn: &Connection,
    output_path: &Path,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<ExportResult, String> {
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let buf = BufWriter::new(file);
    let mut wtr = csv::Writer::from_writer(buf);

    wtr.write_record([
        "transaction_id",
        "date",
        "description",
        "currency",
        "account",
        "account_type",
        "amount",
        "category",
        "reconciled",
    ])
    .map_err(|e| e.to_string())?;

    let sql = "
        SELECT
            t.id as transaction_id,
            t.date,
            t.description,
            t.is_reconciled,
            a.currency,
            a.name as account_name,
            a.type as account_type,
            p.amount,
            c.name as category_name
        FROM transactions t
        JOIN postings p ON p.transaction_id = t.id
        JOIN accounts a ON a.id = p.account_id
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.deleted_at IS NULL
          AND (?1 IS NULL OR t.date >= ?1)
          AND (?2 IS NULL OR t.date <= ?2)
        ORDER BY t.date ASC, t.created_at ASC, p.sequence ASC
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![start_date, end_date], |row| {
            Ok(ExportRow {
                transaction_id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                is_reconciled: row.get::<_, i32>(3)? == 1,
                currency: row.get(4)?,
                account_name: row.get(5)?,
                account_type: row.get(6)?,
                amount: row.get(7)?,
                category_name: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut transaction_ids: HashSet<String> = HashSet::new();
    let mut posting_count: u32 = 0;

    for row_result in rows {
        let row = row_result.map_err(|e| e.to_string())?;
        transaction_ids.insert(row.transaction_id.clone());
        posting_count += 1;

        wtr.write_record([
            &row.transaction_id,
            &row.date,
            &row.description,
            &row.currency,
            &row.account_name,
            &row.account_type,
            &row.amount.to_string(),
            row.category_name.as_deref().unwrap_or(""),
            &row.is_reconciled.to_string(),
        ])
        .map_err(|e| e.to_string())?;
    }

    wtr.flush().map_err(|e| e.to_string())?;

    Ok(ExportResult {
        transaction_count: transaction_ids.len() as u32,
        posting_count,
    })
}

/// Export transactions to Beancount format.
/// Uses streaming with BufWriter: lightweight header query + streaming transaction body.
pub fn export_beancount(
    conn: &Connection,
    output_path: &Path,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> Result<ExportResult, String> {
    let file = std::fs::File::create(output_path).map_err(|e| e.to_string())?;
    let mut buf = BufWriter::new(file);

    let now = crate::utils::time::now_rfc3339();
    writeln!(buf, "; GlimmerX Export - Generated at {}", now).map_err(|e| e.to_string())?;

    let accounts_sql = "
        SELECT DISTINCT
            a.currency,
            a.name as account_name,
            a.type as account_type
        FROM transactions t
        JOIN postings p ON p.transaction_id = t.id
        JOIN accounts a ON a.id = p.account_id
        WHERE t.deleted_at IS NULL
          AND (?1 IS NULL OR t.date >= ?1)
          AND (?2 IS NULL OR t.date <= ?2)
    ";

    let mut accounts_stmt = conn.prepare(accounts_sql).map_err(|e| e.to_string())?;
    let accounts_rows = accounts_stmt
        .query_map(params![start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut currencies: HashSet<String> = HashSet::new();
    let mut accounts: HashSet<(String, String)> = HashSet::new();

    for row_result in accounts_rows {
        let (currency, account_name, account_type) = row_result.map_err(|e| e.to_string())?;
        currencies.insert(currency);
        accounts.insert((account_name, account_type));
    }

    let first_date_sql = "
        SELECT MIN(t.date)
        FROM transactions t
        WHERE t.deleted_at IS NULL
          AND (?1 IS NULL OR t.date >= ?1)
          AND (?2 IS NULL OR t.date <= ?2)
    ";

    let first_date: Option<String> = conn
        .query_row(first_date_sql, params![start_date, end_date], |row| {
            row.get(0)
        })
        .optional()
        .map_err(|e| e.to_string())?
        .flatten();

    for currency in &currencies {
        writeln!(buf, "option \"operating_currency\" \"{}\"", currency)
            .map_err(|e| e.to_string())?;
    }
    writeln!(buf).map_err(|e| e.to_string())?;

    writeln!(buf, "; === Accounts ===").map_err(|e| e.to_string())?;
    let default_date = first_date.unwrap_or_else(|| "2024-01-01".to_string());
    for (account_name, account_type) in &accounts {
        let beancount_account = convert_account_to_beancount(account_name, account_type);
        writeln!(buf, "{} open {}", default_date, beancount_account).map_err(|e| e.to_string())?;
    }
    writeln!(buf).map_err(|e| e.to_string())?;

    writeln!(buf, "; === Transactions ===").map_err(|e| e.to_string())?;

    let txn_sql = "
        SELECT
            t.id as transaction_id,
            t.date,
            t.description,
            t.is_reconciled,
            a.currency,
            a.name as account_name,
            a.type as account_type,
            p.amount,
            c.name as category_name
        FROM transactions t
        JOIN postings p ON p.transaction_id = t.id
        JOIN accounts a ON a.id = p.account_id
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE t.deleted_at IS NULL
          AND (?1 IS NULL OR t.date >= ?1)
          AND (?2 IS NULL OR t.date <= ?2)
        ORDER BY t.date ASC, t.created_at ASC, p.sequence ASC
    ";

    let mut txn_stmt = conn.prepare(txn_sql).map_err(|e| e.to_string())?;
    let txn_rows = txn_stmt
        .query_map(params![start_date, end_date], |row| {
            Ok(ExportRow {
                transaction_id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                is_reconciled: row.get::<_, i32>(3)? == 1,
                currency: row.get(4)?,
                account_name: row.get(5)?,
                account_type: row.get(6)?,
                amount: row.get(7)?,
                category_name: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;

    let mut transaction_ids: HashSet<String> = HashSet::new();
    let mut posting_count: u32 = 0;
    let mut current_txn_id: Option<String> = None;

    for row_result in txn_rows {
        let row = row_result.map_err(|e| e.to_string())?;
        transaction_ids.insert(row.transaction_id.clone());
        posting_count += 1;

        if current_txn_id.as_ref() != Some(&row.transaction_id) {
            current_txn_id = Some(row.transaction_id.clone());
            let flag = if row.is_reconciled { "*" } else { "!" };
            writeln!(buf, "{} {} \"{}\"", row.date, flag, row.description)
                .map_err(|e| e.to_string())?;
        }

        let beancount_account = convert_account_to_beancount(&row.account_name, &row.account_type);
        let amount_yuan = row.amount as f64 / 100.0;
        writeln!(
            buf,
            "  {}      {:.2} {}",
            beancount_account, amount_yuan, row.currency
        )
        .map_err(|e| e.to_string())?;
    }

    if posting_count > 0 {
        writeln!(buf).map_err(|e| e.to_string())?;
    }

    buf.flush().map_err(|e| e.to_string())?;

    Ok(ExportResult {
        transaction_count: transaction_ids.len() as u32,
        posting_count,
    })
}

/// Convert GlimmerX account name to Beancount format using account type.
/// e.g., "餐饮" with type "expense" -> "Expenses:餐饮"
fn convert_account_to_beancount(account_name: &str, account_type: &str) -> String {
    let type_prefix = match account_type {
        "asset" => "Assets",
        "liability" => "Liabilities",
        "income" => "Income",
        "expense" => "Expenses",
        "equity" => "Equity",
        other => other,
    };

    let parts: Vec<&str> = account_name.split('/').collect();
    if parts.is_empty() {
        return format!("{}:{}", type_prefix, account_name);
    }

    let rest: Vec<String> = parts.iter().map(|s| capitalize_first(s)).collect();

    format!("{}:{}", type_prefix, rest.join(":"))
}

fn capitalize_first(s: &str) -> String {
    s.chars()
        .next()
        .map(|c| {
            let mut result = c.to_uppercase().collect::<String>();
            let rest: String = s.chars().skip(1).collect();
            result.push_str(&rest);
            result
        })
        .unwrap_or_else(|| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_convert_account_to_beancount() {
        assert_eq!(convert_account_to_beancount("bank", "asset"), "Assets:Bank");
        assert_eq!(
            convert_account_to_beancount("food", "expense"),
            "Expenses:Food"
        );
        assert_eq!(
            convert_account_to_beancount("salary", "income"),
            "Income:Salary"
        );
        assert_eq!(
            convert_account_to_beancount("credit", "liability"),
            "Liabilities:Credit"
        );
        assert_eq!(
            convert_account_to_beancount("opening", "equity"),
            "Equity:Opening"
        );
    }

    #[test]
    fn test_convert_account_to_beancount_chinese() {
        assert_eq!(
            convert_account_to_beancount("餐饮", "expense"),
            "Expenses:餐饮"
        );
        assert_eq!(
            convert_account_to_beancount("我的资产", "asset"),
            "Assets:我的资产"
        );
        assert_eq!(
            convert_account_to_beancount("娱乐", "expense"),
            "Expenses:娱乐"
        );
    }

    #[test]
    fn test_convert_account_to_beancount_with_slash() {
        assert_eq!(
            convert_account_to_beancount("sub/account", "asset"),
            "Assets:Sub:Account"
        );
        assert_eq!(
            convert_account_to_beancount("food/lunch", "expense"),
            "Expenses:Food:Lunch"
        );
    }

    #[test]
    fn test_export_csv_empty_db() {
        let conn = setup_test_db();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.csv");

        let result = export_csv(&conn, &path, None, None);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_count, 0);
        assert_eq!(r.posting_count, 0);
    }

    #[test]
    fn test_export_beancount_empty_db() {
        let conn = setup_test_db();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.beancount");

        let result = export_beancount(&conn, &path, None, None);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_count, 0);
        assert_eq!(r.posting_count, 0);
    }

    #[test]
    fn test_export_csv_with_data() {
        let conn = setup_test_db();
        let now = crate::utils::time::now_rfc3339();
        let today = crate::utils::time::today_date();

        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('acc1', 'Bank', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('acc2', 'Salary', 'income', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES ('txn1', ?1, 'Salary payment', ?2, ?2)",
            [&today, &now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p1', 'txn1', 'acc1', 500000, 0, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p2', 'txn1', 'acc2', -500000, 1, ?1)",
            [&now],
        )
        .unwrap();

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.csv");

        let result = export_csv(&conn, &path, None, None);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_count, 1);
        assert_eq!(r.posting_count, 2);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("txn1"));
        assert!(content.contains("Salary payment"));
        assert!(content.contains("500000"));
        assert!(content.contains("-500000"));
    }

    #[test]
    fn test_export_beancount_with_data() {
        let conn = setup_test_db();
        let now = crate::utils::time::now_rfc3339();
        let today = crate::utils::time::today_date();

        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('acc1', 'Bank', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('acc2', 'Salary', 'income', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES ('txn1', ?1, 'Salary payment', ?2, ?2)",
            [&today, &now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p1', 'txn1', 'acc1', 500000, 0, ?1)",
            [&now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p2', 'txn1', 'acc2', -500000, 1, ?1)",
            [&now],
        )
        .unwrap();

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.beancount");

        let result = export_beancount(&conn, &path, None, None);
        assert!(result.is_ok());
        let r = result.unwrap();
        assert_eq!(r.transaction_count, 1);
        assert_eq!(r.posting_count, 2);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("option \"operating_currency\" \"CNY\""));
        assert!(content.contains("open Assets:Bank"));
        assert!(content.contains("open Income:Salary"));
        assert!(content.contains("Salary payment"));
        assert!(content.contains("5000.00 CNY"));
    }
}
