//! Import module for CSV format.
//!
//! Provides structures and functions for importing transactions from CSV files.

use csv::ReaderBuilder;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;

use crate::db::accounts::{create_account_with_path, find_account_by_type_and_name};
use crate::db::schema::ensure_transaction_hashes_table;
use crate::db::transactions::{create_transaction, PostingInput as DbPostingInput};
use crate::utils::time::now_rfc3339;

/// Row from a CSV import file.
#[derive(Debug, Deserialize)]
pub struct ImportRow {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub currency: String,
    pub account: String,
    pub account_type: String,
    pub amount: i64,
    pub category: Option<String>,
    pub reconciled: Option<bool>,
}

/// Result of an import operation.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported_count: u32,
    pub skipped_count: u32,
    pub error_count: u32,
    pub created_accounts: Vec<String>,
    pub errors: Vec<ImportError>,
}

/// Error that occurred during import.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportError {
    pub row_number: u32,
    pub transaction_id: String,
    pub message: String,
}

/// Options for import behavior.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    pub create_missing_accounts: bool,
    pub skip_duplicates: bool,
}

/// Input for hash computation (uses account name for canonical representation).
#[derive(Debug)]
pub struct HashPostingInput {
    pub account: String,
    pub amount: i64,
}

/// Compute SHA-256 hash for duplicate detection.
pub fn compute_transaction_hash(
    date: &str,
    description: &str,
    postings: &[HashPostingInput],
) -> String {
    let mut sorted_postings: Vec<_> = postings
        .iter()
        .map(|p| format!("{}:{}", p.account, p.amount))
        .collect();
    sorted_postings.sort();

    let canonical = format!("{}|{}|{}", date, description, sorted_postings.join("|"));
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

/// Main CSV import function.
pub fn import_csv(
    conn: &mut Connection,
    input_path: &Path,
    options: &ImportOptions,
) -> Result<ImportResult, String> {
    ensure_transaction_hashes_table(conn).map_err(|e| e.to_string())?;
    ensure_existing_hashes(conn)?;

    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(input_path)
        .map_err(|e| format!("Cannot read file: {}", e))?;

    let rows: Vec<ImportRow> = rdr
        .deserialize()
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("CSV parse error: {}", e))?;

    if rows.is_empty() {
        return Err("errors.import.emptyFile".to_string());
    }

    let mut transactions: HashMap<String, Vec<ImportRow>> = HashMap::new();
    for row in rows {
        transactions
            .entry(row.transaction_id.clone())
            .or_default()
            .push(row);
    }

    let mut result = ImportResult::default();
    let mut pending_reconciled: Vec<(String, bool)> = Vec::new();
    for (txn_id, txn_rows) in transactions {
        if txn_rows.len() < 2 {
            result.errors.push(ImportError {
                row_number: 0,
                transaction_id: txn_id.clone(),
                message: "errors.transactionMinPostings".to_string(),
            });
            result.error_count += 1;
            continue;
        }

        let sum: i64 = txn_rows.iter().map(|r| r.amount).sum();
        if sum != 0 {
            result.errors.push(ImportError {
                row_number: 0,
                transaction_id: txn_id.clone(),
                message: format!("errors.transactionUnbalanced: sum={}", sum),
            });
            result.error_count += 1;
            continue;
        }

        if txn_rows.iter().any(|r| r.account_type == "equity") {
            result.errors.push(ImportError {
                row_number: 0,
                transaction_id: txn_id.clone(),
                message: "errors.transaction.equityAccountRestricted".to_string(),
            });
            result.error_count += 1;
            continue;
        }

        let mut postings: Vec<DbPostingInput> = Vec::new();
        let mut hash_postings: Vec<HashPostingInput> = Vec::new();
        let mut failed = false;

        for row in &txn_rows {
            let account = find_account_by_type_and_name(conn, &row.account_type, &row.account)
                .map_err(|e| e.to_string())?;

            match account {
                Some(acc) => {
                    if !acc.is_active {
                        result.errors.push(ImportError {
                            row_number: 0,
                            transaction_id: txn_id.clone(),
                            message: format!(
                                "errors.account.inactive: {} ({})",
                                row.account, row.account_type
                            ),
                        });
                        result.error_count += 1;
                        failed = true;
                        break;
                    }
                    postings.push(DbPostingInput {
                        account_id: acc.id.clone(),
                        amount: row.amount,
                    });
                    hash_postings.push(HashPostingInput {
                        account: row.account.clone(),
                        amount: row.amount,
                    });
                }
                None if options.create_missing_accounts => {
                    let path = format!("{}/{}", row.account_type, row.account);
                    let new_id = create_account_with_path(conn, &path, &row.currency, None)
                        .map_err(|e| e.to_string())?;
                    postings.push(DbPostingInput {
                        account_id: new_id,
                        amount: row.amount,
                    });
                    hash_postings.push(HashPostingInput {
                        account: row.account.clone(),
                        amount: row.amount,
                    });
                    result.created_accounts.push(row.account.clone());
                }
                None => {
                    result.errors.push(ImportError {
                        row_number: 0,
                        transaction_id: txn_id.clone(),
                        message: format!(
                            "errors.accountNotFound: {} ({})",
                            row.account, row.account_type
                        ),
                    });
                    result.error_count += 1;
                    failed = true;
                    break;
                }
            }
        }

        if failed {
            continue;
        }

        let first_row = &txn_rows[0];
        let hash =
            compute_transaction_hash(&first_row.date, &first_row.description, &hash_postings);

        if options.skip_duplicates {
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM transaction_hashes WHERE content_hash = ?1",
                    params![&hash],
                    |_| Ok(true),
                )
                .unwrap_or(false);

            if exists {
                result.skipped_count += 1;
                continue;
            }
        }

        let category_id: Option<String> = match first_row.category.as_ref() {
            None => None,
            Some(cat_name) => {
                let found: Option<String> = conn
                    .query_row(
                        "SELECT id FROM categories WHERE name = ?1",
                        params![cat_name],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                found
            }
        };
        let created_id = create_transaction(
            conn,
            &first_row.date,
            &first_row.description,
            category_id.as_deref(),
            &postings,
        )
        .map_err(|e| e.to_string())?;
        let now = now_rfc3339();
        conn.execute(
            "INSERT INTO transaction_hashes (transaction_id, content_hash, import_source, imported_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                &created_id,
                &hash,
                input_path.to_string_lossy().to_string(),
                &now
            ],
        )
        .map_err(|e| e.to_string())?;

        if first_row.reconciled == Some(true) {
            pending_reconciled.push((created_id, true));
        }

        result.imported_count += 1;
    }

    if !pending_reconciled.is_empty() {
        update_reconciled_status(conn, &pending_reconciled)?;
    }

    Ok(result)
}

/// Ensure existing transactions have hashes for duplicate detection.
pub fn ensure_existing_hashes(conn: &mut Connection) -> Result<u32, String> {
    let hash_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM transaction_hashes", [], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?;

    if hash_count > 0 {
        return Ok(0);
    }

    let total_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    const BATCH_SIZE: i32 = 500;
    let mut migrated: u32 = 0;

    for offset in (0..total_count as i32).step_by(BATCH_SIZE as usize) {
        let txns: Vec<(String, String, String)> = conn
            .prepare(
                "SELECT id, date, description FROM transactions WHERE deleted_at IS NULL ORDER BY created_at LIMIT ?1 OFFSET ?2",
            )
            .and_then(|mut stmt| {
                let rows = stmt.query_map(params![BATCH_SIZE, offset], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?;
                rows.collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| e.to_string())?;

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        for (txn_id, date, description) in txns {
            let postings: Vec<HashPostingInput> = tx
                .prepare(
                    "SELECT a.name, p.amount 
                     FROM postings p 
                     JOIN accounts a ON a.id = p.account_id 
                     WHERE p.transaction_id = ?1 
                     ORDER BY p.sequence",
                )
                .and_then(|mut stmt| {
                    let rows = stmt.query_map(params![&txn_id], |row| {
                        Ok(HashPostingInput {
                            account: row.get::<_, String>(0)?,
                            amount: row.get::<_, i64>(1)?,
                        })
                    })?;
                    rows.collect::<Result<Vec<_>, _>>()
                })
                .map_err(|e| e.to_string())?;

            let hash = compute_transaction_hash(&date, &description, &postings);
            let now = now_rfc3339();

            tx.execute(
                "INSERT INTO transaction_hashes (transaction_id, content_hash, import_source, imported_at, created_at)
                 VALUES (?1, ?2, NULL, ?3, ?3)",
                params![&txn_id, &hash, &now],
            )
            .map_err(|e| e.to_string())?;

            migrated += 1;
        }
        tx.commit().map_err(|e| e.to_string())?;
    }

    Ok(migrated)
}

fn update_reconciled_status(conn: &Connection, updates: &[(String, bool)]) -> Result<(), String> {
    let now = now_rfc3339();
    for (txn_id, is_reconciled) in updates {
        conn.execute(
            "UPDATE transactions SET is_reconciled = ?1, updated_at = ?2 WHERE id = ?3",
            params![*is_reconciled as i32, &now, txn_id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts::{create_account_with_path, find_account_by_type_and_name};
    use crate::db::categories::{create_category, CategoryType};
    use crate::db::transactions::PostingInput;
    use crate::db::Database;
    use rusqlite::params;
    use std::fs;
    use tempfile::TempDir;

    const HEADER: &str =
        "transaction_id,date,description,currency,account,account_type,amount,category,reconciled\n";

    fn setup_db() -> Database {
        let dir = tempfile::TempDir::new().unwrap();
        Database::create(&dir.path().join("test.db"), "secret").unwrap()
    }

    fn write_csv(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn valid_txn_csv() -> String {
        format!(
            "{}txn-1,2024-02-01,Lunch,CNY,Cash,asset,-2500,,false\ntxn-1,2024-02-01,Lunch,CNY,Food,expense,2500,,false\n",
            HEADER
        )
    }

    fn import_options(create_missing: bool, skip_duplicates: bool) -> ImportOptions {
        ImportOptions {
            create_missing_accounts: create_missing,
            skip_duplicates,
        }
    }

    #[test]
    fn test_compute_transaction_hash_sorted() {
        let h1 = compute_transaction_hash(
            "2024-02-01",
            "Lunch",
            &[
                HashPostingInput {
                    account: "Cash".into(),
                    amount: -2500,
                },
                HashPostingInput {
                    account: "Food".into(),
                    amount: 2500,
                },
            ],
        );
        let h2 = compute_transaction_hash(
            "2024-02-01",
            "Lunch",
            &[
                HashPostingInput {
                    account: "Food".into(),
                    amount: 2500,
                },
                HashPostingInput {
                    account: "Cash".into(),
                    amount: -2500,
                },
            ],
        );
        assert_eq!(h1, h2, "hash must not depend on posting order");

        let h3 = compute_transaction_hash(
            "2024-02-01",
            "Dinner",
            &[
                HashPostingInput {
                    account: "Cash".into(),
                    amount: -2500,
                },
                HashPostingInput {
                    account: "Food".into(),
                    amount: 2500,
                },
            ],
        );
        assert_ne!(h1, h3, "different description must change hash");
    }

    #[test]
    fn test_import_csv_creates_missing_accounts() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "in.csv", &valid_txn_csv());

        let result = import_csv(&mut conn, &path, &import_options(true, true)).unwrap();
        assert_eq!(result.imported_count, 1);
        assert_eq!(result.skipped_count, 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.created_accounts.len(), 2);

        // Accounts were auto-created
        assert!(find_account_by_type_and_name(&conn, "asset", "Cash")
            .unwrap()
            .is_some());
        assert!(find_account_by_type_and_name(&conn, "expense", "Food")
            .unwrap()
            .is_some());

        // Transaction was created with a hash entry
        let tx_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tx_count, 1);
        let hash_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transaction_hashes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(hash_count, 1);
    }

    #[test]
    fn test_import_csv_existing_accounts_category_and_reconciled() {
        let db = setup_db();
        {
            let conn = db.get_conn().unwrap();
            create_account_with_path(&conn, "Assets/Cash", "CNY", None).unwrap();
            create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
            create_category(&conn, "Dining", &CategoryType::Expense, None).unwrap();
        }
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(
            &dir,
            "in.csv",
            &format!(
                "{}txn-1,2024-02-01,Lunch,CNY,Cash,asset,-2500,Dining,true\ntxn-1,2024-02-01,Lunch,CNY,Food,expense,2500,Dining,true\n",
                HEADER
            ),
        );

        let result = import_csv(&mut conn, &path, &import_options(false, false)).unwrap();
        assert_eq!(result.imported_count, 1);
        assert!(result.created_accounts.is_empty());

        let (category_id, is_reconciled): (Option<String>, i64) = conn
            .query_row(
                "SELECT category_id, is_reconciled FROM transactions WHERE description = 'Lunch'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(category_id.is_some());
        assert_eq!(is_reconciled, 1);
    }

    #[test]
    fn test_import_csv_skip_duplicates() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "in.csv", &valid_txn_csv());

        let first = import_csv(&mut conn, &path, &import_options(true, true)).unwrap();
        assert_eq!(first.imported_count, 1);

        let second = import_csv(&mut conn, &path, &import_options(true, true)).unwrap();
        assert_eq!(second.imported_count, 0);
        assert_eq!(second.skipped_count, 1);
        assert_eq!(second.error_count, 0);
    }

    #[test]
    fn test_import_csv_without_skip_duplicates() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "in.csv", &valid_txn_csv());

        let first = import_csv(&mut conn, &path, &import_options(true, false)).unwrap();
        assert_eq!(first.imported_count, 1);
        // Without skip_duplicates the second import still fails on the unique
        // content hash constraint.
        let err = import_csv(&mut conn, &path, &import_options(true, false)).unwrap_err();
        assert!(err.contains("UNIQUE constraint"));
        let tx_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM transactions", [], |r| r.get(0))
            .unwrap();
        // The failed hash insert leaves the already-created transaction row.
        assert_eq!(tx_count, 2);
    }

    #[test]
    fn test_import_csv_empty_file() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "empty.csv", HEADER);

        let err = import_csv(&mut conn, &path, &import_options(true, false)).unwrap_err();
        assert_eq!(err, "errors.import.emptyFile");
    }

    #[test]
    fn test_import_csv_parse_error() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(
            &dir,
            "bad.csv",
            "transaction_id,date,description,currency,account,account_type,amount,category,reconciled\n\
             txn-1,2024-02-01,Lunch,CNY,Cash,asset,not-a-number,,false\n",
        );

        let err = import_csv(&mut conn, &path, &import_options(true, false)).unwrap_err();
        assert!(err.contains("CSV parse error"));
    }

    #[test]
    fn test_import_csv_unreadable_file() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.csv");

        let err = import_csv(&mut conn, &path, &import_options(true, false)).unwrap_err();
        assert!(err.contains("Cannot read file"));
    }

    #[test]
    fn test_import_csv_min_postings_error() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(
            &dir,
            "in.csv",
            &format!(
                "{}txn-1,2024-02-01,Lunch,CNY,Cash,asset,-2500,,false\n",
                HEADER
            ),
        );

        let result = import_csv(&mut conn, &path, &import_options(true, false)).unwrap();
        assert_eq!(result.imported_count, 0);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.errors[0].message, "errors.transactionMinPostings");
    }

    #[test]
    fn test_import_csv_unbalanced_error() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(
            &dir,
            "in.csv",
            &format!(
                "{}txn-1,2024-02-01,Lunch,CNY,Cash,asset,-2500,,false\ntxn-1,2024-02-01,Lunch,CNY,Food,expense,1000,,false\n",
                HEADER
            ),
        );

        let result = import_csv(&mut conn, &path, &import_options(true, false)).unwrap();
        assert_eq!(result.error_count, 1);
        assert_eq!(result.imported_count, 0);
        assert!(result.errors[0]
            .message
            .contains("errors.transactionUnbalanced"));
    }

    #[test]
    fn test_import_csv_equity_restricted() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(
            &dir,
            "in.csv",
            &format!(
                "{}txn-1,2024-02-01,Opening,CNY,Owner,equity,-10000,,false\ntxn-1,2024-02-01,Opening,CNY,Cash,asset,10000,,false\n",
                HEADER
            ),
        );

        let result = import_csv(&mut conn, &path, &import_options(true, false)).unwrap();
        assert_eq!(result.error_count, 1);
        assert!(result.errors[0]
            .message
            .contains("errors.transaction.equityAccountRestricted"));
    }

    #[test]
    fn test_import_csv_account_not_found() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "in.csv", &valid_txn_csv());

        let result = import_csv(&mut conn, &path, &import_options(false, false)).unwrap();
        assert_eq!(result.error_count, 1);
        assert_eq!(result.imported_count, 0);
        assert!(result.errors[0].message.contains("errors.accountNotFound"));
    }

    #[test]
    fn test_import_csv_inactive_account() {
        let db = setup_db();
        {
            let conn = db.get_conn().unwrap();
            create_account_with_path(&conn, "Assets/Cash", "CNY", None).unwrap();
            create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
            conn.execute("UPDATE accounts SET is_active = 0 WHERE name = 'Cash'", [])
                .unwrap();
        }
        let mut conn = db.get_conn().unwrap();
        let dir = TempDir::new().unwrap();
        let path = write_csv(&dir, "in.csv", &valid_txn_csv());

        let result = import_csv(&mut conn, &path, &import_options(false, false)).unwrap();
        assert_eq!(result.error_count, 1);
        assert_eq!(result.imported_count, 0);
        assert!(result.errors[0].message.contains("errors.account.inactive"));
    }

    #[test]
    fn test_ensure_existing_hashes_empty() {
        let db = setup_db();
        let mut conn = db.get_conn().unwrap();
        assert_eq!(ensure_existing_hashes(&mut conn).unwrap(), 0);
    }

    #[test]
    fn test_ensure_existing_hashes_migrates_in_batches() {
        let db = setup_db();
        {
            let conn = db.get_conn().unwrap();
            let cash = create_account_with_path(&conn, "Assets/Cash", "CNY", None).unwrap();
            let food = create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
            for i in 0..501 {
                crate::db::transactions::create_transaction(
                    &conn,
                    "2024-01-01",
                    &format!("Tx {}", i),
                    None,
                    &[
                        PostingInput {
                            account_id: cash.clone(),
                            amount: -100,
                        },
                        PostingInput {
                            account_id: food.clone(),
                            amount: 100,
                        },
                    ],
                )
                .unwrap();
            }
        }

        let mut conn = db.get_conn().unwrap();
        let migrated = ensure_existing_hashes(&mut conn).unwrap();
        assert_eq!(migrated, 501);

        // Second call finds hashes already present
        assert_eq!(ensure_existing_hashes(&mut conn).unwrap(), 0);
    }
}
