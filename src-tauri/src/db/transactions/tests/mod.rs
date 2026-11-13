//! Shared fixtures for the transaction tests, plus one module per area.

use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Uuid;

use crate::db::transactions::paginate::{
    format_date_display, parse_category_names, parse_postings_summary,
};
use crate::db::transactions::validate::{
    format_amount_display, validate_accounts_active, validate_no_equity_accounts,
};
use crate::db::transactions::*;
use crate::db::AppError;

fn test_conn() -> (TempDir, Connection) {
    // The real schema, not a hand-written copy: a stale copy is exactly how the
    // v2 column drops went unnoticed by the test suite.
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    crate::db::init_schema(&conn).unwrap();
    (dir, conn)
}

fn make_account(conn: &Connection, name: &str, account_type: &str) -> String {
    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'CNY', '', 0, ?4, ?4)",
            rusqlite::params![&id, name, account_type, &now],
        )
        .unwrap();
    id
}

fn make_category(conn: &Connection, name: &str, category_type: &str) -> String {
    // Go through the real API: a category and its mirror P&L account are
    // always created together, and quick-add resolves the account through
    // `categories.account_id`.
    let kind = if category_type == "income" {
        crate::db::categories::CategoryType::Income
    } else {
        crate::db::categories::CategoryType::Expense
    };
    crate::db::categories::create_category(conn, name, &kind, None).unwrap()
}

fn inject_second_posting_failure(conn: &Connection) {
    conn.execute_batch(
        "CREATE TRIGGER fail_second_posting BEFORE INSERT ON postings
             WHEN (SELECT count(*) FROM postings WHERE transaction_id = NEW.transaction_id) >= 1
             BEGIN SELECT RAISE(ABORT, 'injected failure'); END;",
    )
    .unwrap();
}

fn count_rows(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |row| row.get(0)).unwrap()
}

mod create;
mod paginate;
mod query;
mod quick_add;
mod update;
mod validate;
