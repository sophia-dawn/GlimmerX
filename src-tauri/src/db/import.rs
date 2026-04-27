//! Import module for CSV format.
//!
//! Provides structures and functions for importing transactions from CSV files.

#![allow(dead_code)]

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

    println!(
        "[import] Processing {} transactions from CSV",
        transactions.len()
    );

    for (txn_id, txn_rows) in transactions {
        println!(
            "[import] Transaction {}: {} postings",
            txn_id,
            txn_rows.len()
        );

        if txn_rows.len() < 2 {
            println!(
                "[import] ERROR: Transaction {} has only {} postings (min 2 required)",
                txn_id,
                txn_rows.len()
            );
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
            println!(
                "[import] ERROR: Transaction {} is unbalanced, sum = {}",
                txn_id, sum
            );
            result.errors.push(ImportError {
                row_number: 0,
                transaction_id: txn_id.clone(),
                message: format!("errors.transactionUnbalanced: sum={}", sum),
            });
            result.error_count += 1;
            continue;
        }

        if txn_rows.iter().any(|r| r.account_type == "equity") {
            println!(
                "[import] ERROR: Transaction {} contains equity account (restricted)",
                txn_id
            );
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
            println!(
                "[import]   Looking for account: {} ({})",
                row.account, row.account_type
            );
            let account = find_account_by_type_and_name(conn, &row.account_type, &row.account)
                .map_err(|e| e.to_string())?;

            match account {
                Some(acc) => {
                    println!(
                        "[import]   Found account {} (id={}, is_active={})",
                        row.account, acc.id, acc.is_active
                    );
                    if !acc.is_active {
                        println!(
                            "[import] ERROR: Account {} ({}) is inactive",
                            row.account, row.account_type
                        );
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
                    println!(
                        "[import]   Account {} ({}) not found, creating...",
                        row.account, row.account_type
                    );
                    let path = format!("{}/{}", row.account_type, row.account);
                    let new_id = create_account_with_path(conn, &path, &row.currency, None)
                        .map_err(|e| e.to_string())?;
                    println!(
                        "[import]   Created account {} with id={}",
                        row.account, new_id
                    );
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
                    println!(
                        "[import] ERROR: Account {} ({}) not found",
                        row.account, row.account_type
                    );
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
            println!("[import] Transaction {} SKIPPED due to errors", txn_id);
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
                println!(
                    "[import] Transaction {} SKIPPED as duplicate (hash={})",
                    txn_id,
                    &hash[..8]
                );
                result.skipped_count += 1;
                continue;
            }
        }

        let category_id: Option<String> = match first_row.category.as_ref() {
            None => None,
            Some(cat_name) => {
                println!("[import]   Looking for category: {}", cat_name);
                let found: Option<String> = conn
                    .query_row(
                        "SELECT id FROM categories WHERE name = ?1",
                        params![cat_name],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(|e| e.to_string())?;
                match found {
                    Some(id) => {
                        println!("[import]   Found category {} with id={}", cat_name, id);
                        Some(id)
                    }
                    None => {
                        println!(
                            "[import]   Category {} not found, using null category_id",
                            cat_name
                        );
                        None
                    }
                }
            }
        };

        println!("[import] Creating transaction {}...", txn_id);
        let created_id = create_transaction(
            conn,
            &first_row.date,
            &first_row.description,
            category_id.as_deref(),
            &postings,
        )
        .map_err(|e| {
            println!(
                "[import] ERROR: Failed to create transaction {}: {}",
                txn_id, e
            );
            e.to_string()
        })?;

        println!(
            "[import] Transaction {} created successfully (new_id={})",
            txn_id, created_id
        );

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

    println!("[import] ===== IMPORT SUMMARY =====");
    println!("[import] Imported: {} transactions", result.imported_count);
    println!(
        "[import] Skipped (duplicates): {} transactions",
        result.skipped_count
    );
    println!("[import] Errors: {} transactions", result.error_count);
    if result.error_count > 0 {
        println!("[import] Error details:");
        for err in &result.errors {
            println!("[import]   - {}: {}", err.transaction_id, err.message);
        }
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
