//! Shared fixtures for the transaction tests, plus one module per area.

use rusqlite::Connection;
use tempfile::TempDir;
use uuid::Uuid;

use crate::db::transactions::paginate::{format_date_display, parse_postings_summary};
use crate::db::transactions::validate::{
    format_amount_display, validate_accounts_active, validate_no_equity_accounts,
};
use crate::db::transactions::*;
use crate::db::AppError;

fn test_conn() -> (TempDir, Connection) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE accounts (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                type                TEXT NOT NULL CHECK (type IN ('asset', 'liability', 'income', 'expense', 'equity')),
                currency            TEXT NOT NULL DEFAULT 'CNY',
                description         TEXT NOT NULL DEFAULT '',
                account_number      TEXT,
                is_system           INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
                iban                TEXT,
                is_active           INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
                include_net_worth   INTEGER NOT NULL DEFAULT 1 CHECK (include_net_worth IN (0, 1)),
                created_at          TEXT NOT NULL,
                updated_at          TEXT NOT NULL
            );

            CREATE TABLE categories (
                id         TEXT PRIMARY KEY,
                name       TEXT NOT NULL,
                type       TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                icon       TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT REFERENCES categories(id),
                is_reconciled INTEGER NOT NULL DEFAULT 0 CHECK (is_reconciled IN (0, 1)),
                deleted_at  TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE postings (
                id             TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                account_id     TEXT NOT NULL REFERENCES accounts(id),
                amount         INTEGER NOT NULL,
                sequence       INTEGER NOT NULL DEFAULT 0,
                created_at     TEXT NOT NULL
            );

            CREATE UNIQUE INDEX idx_accounts_type_name ON accounts(type, name);
            CREATE INDEX idx_postings_transaction ON postings(transaction_id);
            CREATE INDEX idx_postings_account ON postings(account_id);
            CREATE INDEX idx_transactions_date ON transactions(date);
        "#,
        )
        .unwrap();
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
    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
        "INSERT INTO categories (id, name, type, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
        rusqlite::params![&id, name, category_type, &now],
    )
    .unwrap();
    id
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
