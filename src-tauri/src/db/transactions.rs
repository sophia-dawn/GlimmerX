use itertools::Itertools;
use rusqlite::{Connection, OptionalExtension};
use std::collections::HashMap;
use uuid::Uuid;

use crate::db::AppError;

// ---------------------------------------------------------------------------
// Data structures (existing)
// ---------------------------------------------------------------------------

/// Internal DB row representation for a transaction.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct TransactionRecord {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub is_reconciled: bool,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Internal DB row representation for a posting.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct PostingRecord {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: i64,
    pub sequence: i32,
    pub created_at: String,
}

/// Transaction with all its postings (for display/API).
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct TransactionWithPostings {
    pub transaction: TransactionRecord,
    pub postings: Vec<PostingRecord>,
}

/// Input for creating a posting.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PostingInput {
    pub account_id: String,
    pub amount: i64,
}

/// QuickAdd input structure for simplified transaction creation.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct QuickAddInput {
    pub mode: String,
    pub amount: String,
    pub source_account_id: Option<String>,
    pub destination_account_id: Option<String>,
    pub category_id: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
}

/// Transaction detail with full posting information.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TransactionDetail {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub postings: Vec<PostingDetail>,
    pub created_at: String,
    pub updated_at: String,
    pub is_balanced: bool,
    pub posting_count: u32,
    pub debit_total: i64,
    pub credit_total: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PostingDetail {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub amount: i64,
    pub amount_display: String,
    pub is_debit: bool,
    pub sequence: u32,
    pub created_at: String,
}

/// Update transaction input (partial update, all fields optional).
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UpdateTransactionInput {
    pub date: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub postings: Option<Vec<PostingInput>>,
}

/// Delete preview information.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DeletePreview {
    pub transaction_id: String,
    pub description: String,
    pub date: String,
    pub posting_count: u32,
    pub can_delete: bool,
    pub warning_message: Option<String>,
}

// ---------------------------------------------------------------------------
// Transaction List Data structures (for paginated list view)
// ---------------------------------------------------------------------------

/// Transaction list item (optimized for frontend display).
///
/// Unlike full TransactionWithPostings:
/// - postings simplified to account name + amount pairs
/// - includes aggregated display_amount (debit total)
/// - category name included for display
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionListItem {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    /// Postings summary: (account_name, amount) pairs
    pub postings_summary: Vec<(String, i64)>,
    /// User-visible amount (debit total, always positive)
    pub display_amount: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Pagination metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub page_size: u32,
    pub total_count: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Transaction list response with pagination and date groups.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionListResponse {
    pub items: Vec<TransactionListItem>,
    pub pagination: PaginationInfo,
    /// Date groups with day_total calculated by backend
    pub date_groups: Vec<TransactionDateGroup>,
}

/// Transactions grouped by date (for frontend display).
///
/// **Design constraint**: day_total is calculated by backend,
/// frontend does NOT compute any amounts (AGENTS.md rule).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionDateGroup {
    pub date: String,
    /// Formatted date display (e.g., "2024年3月15日")
    pub date_display: String,
    pub items: Vec<TransactionListItem>,
    /// Day total (sum of display_amount for all items)
    pub day_total: i64,
}

/// Enhanced transaction filter with all filter options and pagination.
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct TransactionFilter {
    /// Date range filter
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    /// Amount range filter (cents)
    #[allow(dead_code)]
    pub min_amount: Option<i64>,
    #[allow(dead_code)]
    pub max_amount: Option<i64>,
    /// Filter by account (transaction involves this account)
    pub account_id: Option<String>,
    /// Filter by category
    pub category_id: Option<String>,
    /// Description search (LIKE match)
    pub description_query: Option<String>,
    /// Pagination
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// Sorting
    pub sort_by: Option<String>, // "date" | "amount" | "description"
    pub sort_order: Option<String>, // "asc" | "desc"
}

#[allow(dead_code)]
const TRANSACTION_COLUMNS: &str =
    "id, date, description, category_id, is_reconciled, deleted_at, created_at, updated_at";

#[allow(dead_code)]
const POSTING_COLUMNS: &str = "id, transaction_id, account_id, amount, sequence, created_at";

#[allow(dead_code)]
fn row_to_transaction(row: &rusqlite::Row<'_>) -> rusqlite::Result<TransactionRecord> {
    Ok(TransactionRecord {
        id: row.get(0)?,
        date: row.get(1)?,
        description: row.get(2)?,
        category_id: row.get(3)?,
        is_reconciled: row.get::<_, i32>(4)? != 0,
        deleted_at: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[allow(dead_code)]
fn row_to_posting(row: &rusqlite::Row<'_>) -> rusqlite::Result<PostingRecord> {
    Ok(PostingRecord {
        id: row.get(0)?,
        transaction_id: row.get(1)?,
        account_id: row.get(2)?,
        amount: row.get(3)?,
        sequence: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn validate_accounts_active(conn: &Connection, account_ids: &[String]) -> Result<(), AppError> {
    if account_ids.is_empty() {
        return Ok(());
    }

    let placeholders: Vec<String> = account_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, is_active FROM accounts WHERE id IN ({})",
        placeholders.join(",")
    );

    let params: Vec<&String> = account_ids.iter().collect();
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| *p as &dyn rusqlite::ToSql).collect();

    let mut stmt = conn.prepare(&sql)?;
    let results: Vec<(String, bool)> = stmt
        .query_map(rusqlite::params_from_iter(params_refs), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)? != 0))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for account_id in account_ids {
        let found = results.iter().find(|(id, _)| id == account_id);
        match found {
            None => {
                return Err(AppError::NotFound("errors.accountNotFound".to_string()));
            }
            Some((_, is_active)) => {
                if !is_active {
                    return Err(AppError::ValidationError(
                        "errors.account.inactive".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

fn validate_no_equity_accounts(conn: &Connection, account_ids: &[String]) -> Result<(), AppError> {
    if account_ids.is_empty() {
        return Ok(());
    }

    let placeholders: Vec<String> = account_ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT id, type FROM accounts WHERE id IN ({})",
        placeholders.join(",")
    );

    let params: Vec<&String> = account_ids.iter().collect();
    let params_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|p| *p as &dyn rusqlite::ToSql).collect();

    let mut stmt = conn.prepare(&sql)?;
    let results: Vec<(String, String)> = stmt
        .query_map(rusqlite::params_from_iter(params_refs), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for account_id in account_ids {
        let found = results.iter().find(|(id, _)| id == account_id);
        match found {
            None => {
                return Err(AppError::NotFound("errors.accountNotFound".to_string()));
            }
            Some((_, account_type)) => {
                if account_type == "equity" {
                    return Err(AppError::ValidationError(
                        "errors.transaction.equityAccountRestricted".to_string(),
                    ));
                }
            }
        }
    }

    Ok(())
}

#[allow(dead_code)]
fn parse_amount_to_cents(amount_str: &str) -> Result<i64, AppError> {
    let value = amount_str
        .parse::<f64>()
        .map_err(|_| AppError::ValidationError("errors.transaction.invalidAmount".to_string()))?;
    let cents = (value * 100.0).round() as i64;
    Ok(cents)
}

/// Format cents to display string (e.g., "+¥35.00" or "-¥35.00").
#[allow(dead_code)]
fn format_amount_display(amount: i64) -> String {
    let abs_amount = amount.abs();
    let yuan = abs_amount / 100;
    let fen = abs_amount % 100;

    let prefix = if amount > 0 { "+¥" } else { "-¥" };

    if fen == 0 {
        format!("{}{}.00", prefix, yuan)
    } else {
        format!("{}{}.{:02}", prefix, yuan, fen)
    }
}

#[allow(dead_code)]
fn validate_quick_add_input(input: &QuickAddInput) -> Result<i64, AppError> {
    match input.mode.as_str() {
        "expense" | "income" | "transfer" => {}
        _ => {
            return Err(AppError::ValidationError(
                "errors.transaction.invalidMode".to_string(),
            ))
        }
    }

    let amount_cents = parse_amount_to_cents(&input.amount)?;
    if amount_cents <= 0 {
        return Err(AppError::ValidationError(
            "errors.transaction.amountMustBePositive".to_string(),
        ));
    }

    match input.mode.as_str() {
        "expense" => {
            if input.source_account_id.is_none() {
                return Err(AppError::ValidationError(
                    "errors.transaction.expenseRequiresSource".to_string(),
                ));
            }
            if input.destination_account_id.is_none() && input.category_id.is_none() {
                return Err(AppError::ValidationError(
                    "errors.transaction.expenseRequiresDestinationOrCategory".to_string(),
                ));
            }
        }
        "income" if input.destination_account_id.is_none() && input.category_id.is_none() => {
            return Err(AppError::ValidationError(
                "errors.transaction.incomeRequiresDestination".to_string(),
            ));
        }
        "transfer"
            if input.source_account_id.is_none() || input.destination_account_id.is_none() =>
        {
            return Err(AppError::ValidationError(
                "errors.transaction.transferRequiresBothAccounts".to_string(),
            ));
        }
        _ => {}
    }

    Ok(amount_cents)
}

#[allow(dead_code)]
pub fn quick_add_transaction(
    conn: &Connection,
    input: &QuickAddInput,
) -> Result<TransactionWithPostings, AppError> {
    let amount_cents = validate_quick_add_input(input)?;
    let postings = derive_postings(conn, input, amount_cents)?;

    let date = input
        .date
        .clone()
        .unwrap_or_else(crate::utils::time::today_date);
    let description = input.description.clone().unwrap_or_default();

    let tx_id = create_transaction(
        conn,
        &date,
        &description,
        input.category_id.as_deref(),
        &postings,
    )?;

    get_transaction_with_postings(conn, &tx_id)?
        .ok_or_else(|| AppError::DatabaseError("Created transaction not found".to_string()))
}

fn ensure_category_account(conn: &Connection, category_id: &str) -> Result<String, AppError> {
    let (cat_type, cat_name): (String, String) = conn
        .query_row(
            "SELECT type, name FROM categories WHERE id = ?1",
            [category_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("errors.categoryNotFound".to_string()))?;

    let existing: Option<String> = conn
        .query_row(
            "SELECT id FROM accounts WHERE name = ?1 AND type = ?2",
            rusqlite::params![&cat_name, &cat_type],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let account_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
        "INSERT INTO accounts (id, name, type, currency, description, is_active, include_net_worth, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'CNY', '', 1, 0, ?4, ?4)",
        rusqlite::params![&account_id, &cat_name, &cat_type, &now],
    )?;

    Ok(account_id)
}

#[allow(dead_code)]
fn derive_postings(
    conn: &Connection,
    input: &QuickAddInput,
    amount_cents: i64,
) -> Result<Vec<PostingInput>, AppError> {
    match input.mode.as_str() {
        "expense" => {
            let debit_account = if let Some(ref dest_id) = input.destination_account_id {
                dest_id.clone()
            } else if let Some(ref cat_id) = input.category_id {
                ensure_category_account(conn, cat_id)?
            } else {
                return Err(AppError::ValidationError(
                    "errors.transaction.expenseRequiresDestinationOrCategory".to_string(),
                ));
            };

            let credit_account = input.source_account_id.clone().ok_or_else(|| {
                AppError::ValidationError("errors.transaction.expenseRequiresSource".to_string())
            })?;

            Ok(vec![
                PostingInput {
                    account_id: debit_account,
                    amount: amount_cents,
                },
                PostingInput {
                    account_id: credit_account,
                    amount: -amount_cents,
                },
            ])
        }

        "income" => {
            let debit_account = input.destination_account_id.clone().ok_or_else(|| {
                AppError::ValidationError(
                    "errors.transaction.incomeRequiresDestination".to_string(),
                )
            })?;

            let credit_account = if let Some(ref src_id) = input.source_account_id {
                src_id.clone()
            } else if let Some(ref cat_id) = input.category_id {
                ensure_category_account(conn, cat_id)?
            } else {
                return Err(AppError::ValidationError(
                    "errors.transaction.incomeRequiresAccount".to_string(),
                ));
            };

            Ok(vec![
                PostingInput {
                    account_id: debit_account,
                    amount: amount_cents,
                },
                PostingInput {
                    account_id: credit_account,
                    amount: -amount_cents,
                },
            ])
        }

        "transfer" => {
            let debit_account = input.destination_account_id.clone().unwrap();
            let credit_account = input.source_account_id.clone().unwrap();

            if debit_account == credit_account {
                return Err(AppError::ValidationError(
                    "errors.transaction.transferToSelf".to_string(),
                ));
            }

            Ok(vec![
                PostingInput {
                    account_id: debit_account,
                    amount: amount_cents,
                },
                PostingInput {
                    account_id: credit_account,
                    amount: -amount_cents,
                },
            ])
        }

        _ => Err(AppError::ValidationError(
            "errors.transaction.invalidMode".to_string(),
        )),
    }
}

/// Create a new transaction with balanced postings.
///
/// Validates:
/// - At least 2 postings required
/// - Sum of postings.amount must equal 0 (double-entry rule)
/// - All account_ids must exist and be active
/// - No equity accounts allowed (system transactions only)
///
/// Returns the new transaction ID.
#[allow(dead_code)]
pub fn create_transaction(
    conn: &Connection,
    date: &str,
    description: &str,
    category_id: Option<&str>,
    postings: &[PostingInput],
) -> Result<String, AppError> {
    if postings.len() < 2 {
        return Err(AppError::ValidationError(
            "errors.transactionMinPostings".to_string(),
        ));
    }

    let sum: i64 = postings.iter().map(|p| p.amount).sum();
    if sum != 0 {
        return Err(AppError::ValidationError(
            "errors.transactionUnbalanced".to_string(),
        ));
    }

    let account_ids: Vec<String> = postings.iter().map(|p| p.account_id.clone()).collect();

    validate_accounts_active(conn, &account_ids)?;
    validate_no_equity_accounts(conn, &account_ids)?;

    let unique_account_ids: std::collections::HashSet<&str> =
        postings.iter().map(|p| p.account_id.as_str()).collect();
    if unique_account_ids.len() != postings.len() {
        return Err(AppError::ValidationError(
            "errors.transaction.duplicateAccount".to_string(),
        ));
    }

    let tx_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();

    conn.execute(
        &format!(
            "INSERT INTO transactions ({}) VALUES (?1, ?2, ?3, ?4, 0, NULL, ?5, ?5)",
            TRANSACTION_COLUMNS
        ),
        rusqlite::params![&tx_id, date, description, category_id, &now],
    )?;

    for (seq, posting) in postings.iter().enumerate() {
        let posting_id = Uuid::new_v4().to_string();
        conn.execute(
            &format!(
                "INSERT INTO postings ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                POSTING_COLUMNS
            ),
            rusqlite::params![
                &posting_id,
                &tx_id,
                &posting.account_id,
                posting.amount,
                seq as i32,
                &now
            ],
        )?;
    }

    Ok(tx_id)
}

// ---------------------------------------------------------------------------
// Read operations (to be implemented)
// ---------------------------------------------------------------------------

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
    let mut sql_parts: Vec<String> = vec![format!(
        "SELECT {} FROM transactions WHERE deleted_at IS NULL",
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
                "SELECT {} FROM transactions WHERE id = ?1 AND deleted_at IS NULL",
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
    eprintln!("[db::get_transaction_detail] start, tx_id: {}", tx_id);

    let tx_data = conn
        .query_row(
            "SELECT date, description, category_id, created_at, updated_at 
             FROM transactions WHERE id = ?1 AND deleted_at IS NULL",
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
        Ok(Some(data)) => {
            eprintln!("[db::get_transaction_detail] transaction found");
            data
        }
        Ok(None) => {
            eprintln!("[db::get_transaction_detail] NOT FOUND, tx_id: {}", tx_id);
            return Err(AppError::NotFound(
                "errors.transaction.notFound".to_string(),
            ));
        }
        Err(e) => {
            eprintln!("[db::get_transaction_detail] query error: {}", e);
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

    eprintln!("[db::get_transaction_detail] success, tx_id: {}", tx_id);
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

#[allow(dead_code)]
pub fn update_transaction(
    conn: &mut Connection,
    tx_id: &str,
    input: &UpdateTransactionInput,
) -> Result<TransactionDetail, AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1 AND deleted_at IS NULL)",
        rusqlite::params![tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound(
            "errors.transaction.notFound".to_string(),
        ));
    }

    if input.date.is_none()
        && input.description.is_none()
        && input.category_id.is_none()
        && input.postings.is_none()
    {
        return Err(AppError::ValidationError(
            "errors.transaction.noChanges".to_string(),
        ));
    }

    let parsed_postings: Option<Vec<PostingInput>> = if let Some(ref postings) = input.postings {
        if postings.len() < 2 {
            return Err(AppError::ValidationError(
                "errors.transactionMinPostings".to_string(),
            ));
        }

        let total: i64 = postings.iter().map(|p| p.amount).sum();
        if total != 0 {
            return Err(AppError::ValidationError(
                "errors.transactionUnbalanced".to_string(),
            ));
        }

        let account_ids: Vec<String> = postings.iter().map(|p| p.account_id.clone()).collect();
        validate_accounts_active(conn, &account_ids)?;
        validate_no_equity_accounts(conn, &account_ids)?;

        let unique_account_ids: std::collections::HashSet<&str> =
            postings.iter().map(|p| p.account_id.as_str()).collect();
        if unique_account_ids.len() != postings.len() {
            return Err(AppError::ValidationError(
                "errors.transaction.duplicateAccount".to_string(),
            ));
        }

        Some(postings.clone())
    } else {
        None
    };

    let tx = conn.transaction()?;
    let now = crate::utils::time::now_rfc3339();

    if let Some(ref date) = input.date {
        tx.execute(
            "UPDATE transactions SET date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![date, &now, tx_id],
        )?;
    }

    if let Some(ref desc) = input.description {
        tx.execute(
            "UPDATE transactions SET description = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![desc, &now, tx_id],
        )?;
    }

    if let Some(ref cat_id) = input.category_id {
        if cat_id == "__clear__" {
            tx.execute(
                "UPDATE transactions SET category_id = NULL, updated_at = ?1 WHERE id = ?2",
                rusqlite::params![&now, tx_id],
            )?;
        } else {
            tx.execute(
                "UPDATE transactions SET category_id = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![cat_id, &now, tx_id],
            )?;
        }
    }

    if let Some(ref new_postings) = parsed_postings {
        tx.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;

        for (seq, posting) in new_postings.iter().enumerate() {
            let posting_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    &posting_id,
                    tx_id,
                    &posting.account_id,
                    posting.amount,
                    seq as i32,
                    &now
                ],
            )?;
        }
    }

    tx.execute(
        "UPDATE transactions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, tx_id],
    )?;

    tx.commit()?;

    get_transaction_detail(conn, tx_id)
}

// ---------------------------------------------------------------------------
// Delete Transaction Operations
// ---------------------------------------------------------------------------

/// Get delete preview information.
#[allow(dead_code)]
pub fn get_delete_preview(conn: &Connection, tx_id: &str) -> Result<DeletePreview, AppError> {
    let (description, date): (String, String) = conn
        .query_row(
            "SELECT description, date FROM transactions WHERE id = ?1 AND deleted_at IS NULL",
            [tx_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::NotFound("errors.transaction.notFound".to_string()))?;

    let posting_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
        [tx_id],
        |row| row.get(0),
    )?;

    Ok(DeletePreview {
        transaction_id: tx_id.to_string(),
        description,
        date,
        posting_count,
        can_delete: true,
        warning_message: None,
    })
}

/// Delete transaction permanently.
#[allow(dead_code)]
pub fn delete_transaction(conn: &mut Connection, tx_id: &str) -> Result<(), AppError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1 AND deleted_at IS NULL)",
        [tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound(
            "errors.transaction.notFound".to_string(),
        ));
    }

    // Use atomic transaction so all deletions succeed or fail together
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;
    tx.execute("DELETE FROM transactions WHERE id = ?1", [tx_id])?;
    tx.commit()?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Paginated Transaction List Implementation
// ---------------------------------------------------------------------------

/// Get paginated transaction list with filtering and date grouping.
///
/// Uses single JOIN query to avoid N+1 problem.
/// All amounts are calculated on backend per AGENTS.md constraint.
pub fn list_transactions_paginated(
    conn: &Connection,
    filter: &TransactionFilter,
) -> Result<TransactionListResponse, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    #[allow(clippy::manual_clamp)]
    let page_size = filter.page_size.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * page_size;

    let (where_clause, having_clause, params) = build_filter_where_clause(filter);

    let has_having = !having_clause.is_empty();

    let total_count: u32 = if has_having {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM (
                    SELECT t.id
                    FROM transactions t
                    JOIN postings p ON p.transaction_id = t.id
                    WHERE {} AND t.deleted_at IS NULL
                    GROUP BY t.id
                    HAVING {}
                )",
                if where_clause.is_empty() {
                    "1=1"
                } else {
                    &where_clause
                },
                having_clause
            ),
            rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::types::ToSql)),
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT t.id) FROM transactions t
                 LEFT JOIN postings p ON p.transaction_id = t.id
                 WHERE {} AND t.deleted_at IS NULL",
                if where_clause.is_empty() {
                    "1=1"
                } else {
                    &where_clause
                }
            ),
            rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::types::ToSql)),
            |row| row.get(0),
        )?
    };

    let total_pages = total_count.div_ceil(page_size).max(1);

    let sort_by = filter.sort_by.as_deref().unwrap_or("date");
    let sort_order = filter.sort_order.as_deref().unwrap_or("desc");
    let sort_column = match sort_by {
        "amount" => "display_amount",
        "description" => "t.description",
        _ => "t.date",
    };
    let sort_dir = if sort_order == "asc" { "ASC" } else { "DESC" };
    let secondary_sort = if sort_column == "t.date" {
        ", t.created_at DESC"
    } else {
        ""
    };

    let having_sql = if has_having {
        format!("HAVING {}", having_clause)
    } else {
        "".to_string()
    };

    let items_sql = format!(
        "SELECT
            t.id, t.date, t.description, t.category_id, t.created_at, t.updated_at,
            c.name as category_name,
            c.icon as category_icon,
            COALESCE(SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END), 0) as display_amount,
            GROUP_CONCAT(
                a.name || ':' || CASE
                    WHEN p.amount > 0 THEN '+' || CAST(ABS(p.amount) AS TEXT)
                    ELSE '-' || CAST(ABS(p.amount) AS TEXT)
                END,
                '|'
            ) as postings_data
         FROM transactions t
         LEFT JOIN categories c ON c.id = t.category_id
         JOIN postings p ON p.transaction_id = t.id
         JOIN accounts a ON a.id = p.account_id
         WHERE {} AND t.deleted_at IS NULL
         GROUP BY t.id
         {}
         ORDER BY {} {}{}
         LIMIT ? OFFSET ?",
        if where_clause.is_empty() {
            "1=1"
        } else {
            &where_clause
        },
        having_sql,
        sort_column,
        sort_dir,
        secondary_sort
    );

    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = params;
    sql_params.push(Box::new(page_size as i32));
    sql_params.push(Box::new(offset as i32));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();

    let items: Vec<TransactionListItem> = conn
        .prepare(&items_sql)?
        .query_map(rusqlite::params_from_iter(param_refs), |row| {
            let postings_data: Option<String> = row.get::<_, Option<String>>(9)?;

            let postings_summary = postings_data
                .and_then(|data| parse_postings_summary(&data))
                .unwrap_or_default();

            Ok(TransactionListItem {
                id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                category_id: row.get(3)?,
                category_name: row.get(6)?,
                category_icon: row.get(7)?,
                postings_summary,
                display_amount: row.get(8)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let date_groups = build_date_groups(&items);

    Ok(TransactionListResponse {
        items: items.clone(),
        pagination: PaginationInfo {
            page,
            page_size,
            total_count,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
        date_groups,
    })
}

// ---------------------------------------------------------------------------
// Helper functions for paginated list
// ---------------------------------------------------------------------------

fn build_filter_where_clause(
    filter: &TransactionFilter,
) -> (String, String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut having_conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref from) = filter.from_date {
        conditions.push("t.date >= ?".to_string());
        params.push(Box::new(from.clone()));
    }

    if let Some(ref to) = filter.to_date {
        conditions.push("t.date <= ?".to_string());
        params.push(Box::new(to.clone()));
    }

    if let Some(ref acct_id) = filter.account_id {
        conditions.push("p.account_id = ?".to_string());
        params.push(Box::new(acct_id.clone()));
    }

    if let Some(ref cat_id) = filter.category_id {
        conditions.push("t.category_id = ?".to_string());
        params.push(Box::new(cat_id.clone()));
    }

    if let Some(ref query) = filter.description_query {
        let escaped = escape_like_pattern(query);
        conditions.push("t.description LIKE ? ESCAPE '\\'".to_string());
        params.push(Box::new(format!("%{}%", escaped)));
    }

    if let Some(min) = filter.min_amount {
        having_conditions
            .push("SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END) >= ?".to_string());
        params.push(Box::new(min));
    }

    if let Some(max) = filter.max_amount {
        having_conditions
            .push("SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END) <= ?".to_string());
        params.push(Box::new(max));
    }

    let where_clause = conditions.join(" AND ");
    let having_clause = having_conditions.join(" AND ");
    (where_clause, having_clause, params)
}

fn build_date_groups(items: &[TransactionListItem]) -> Vec<TransactionDateGroup> {
    let mut groups: HashMap<String, TransactionDateGroup> = HashMap::new();

    for item in items {
        if !groups.contains_key(&item.date) {
            groups.insert(
                item.date.clone(),
                TransactionDateGroup {
                    date: item.date.clone(),
                    date_display: format_date_display(&item.date),
                    items: Vec::new(),
                    day_total: 0,
                },
            );
        }

        let group = groups.get_mut(&item.date).unwrap();
        group.items.push(item.clone());
        group.day_total += item.display_amount;
    }

    groups
        .into_values()
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect()
}

fn format_date_display(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let year: i32 = parts[0].parse().unwrap_or(0);
        let month: u32 = parts[1].parse().unwrap_or(0);
        let day: u32 = parts[2].parse().unwrap_or(0);
        format!("{}年{}月{}日", year, month, day)
    } else {
        date_str.to_string()
    }
}

fn parse_postings_summary(data: &str) -> Option<Vec<(String, i64)>> {
    let postings: Vec<(String, i64)> = data
        .split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() == 2 {
                let account = parts[0].trim();
                let amount_str = parts[1].trim();
                let amount: i64 = amount_str.parse().ok()?;
                Some((account.to_string(), amount))
            } else {
                None
            }
        })
        .collect();

    // Sort: negative amounts first (outflow), then positive (inflow)
    // Within each group, sort by absolute value descending
    let sorted = postings
        .into_iter()
        .sorted_by(|a, b| {
            match (a.1 < 0, b.1 < 0) {
                (true, false) => std::cmp::Ordering::Less, // negative first
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.1.abs().cmp(&a.1.abs()), // same sign: by abs desc
            }
        })
        .collect();

    Some(sorted)
}

fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
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

    /// Test 1: Balanced transaction creation should succeed.
    /// Two postings with opposite amounts (e.g., +1000 and -1000) sum to 0.
    #[test]
    fn test_create_transaction_balanced() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let postings = [
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000, // money leaving asset
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000, // money entering expense
            },
        ];

        let tx_id =
            create_transaction(&conn, "2024-01-15", "Grocery shopping", None, &postings).unwrap();

        assert!(!tx_id.is_empty());

        // Verify transaction was created correctly
        let tx_with_postings = get_transaction_with_postings(&conn, &tx_id)
            .unwrap()
            .expect("Transaction should exist");

        assert_eq!(tx_with_postings.transaction.date, "2024-01-15");
        assert_eq!(tx_with_postings.transaction.description, "Grocery shopping");
        assert_eq!(tx_with_postings.postings.len(), 2);

        // Verify postings are balanced (sum = 0)
        let sum: i64 = tx_with_postings.postings.iter().map(|p| p.amount).sum();
        assert_eq!(sum, 0);
    }

    /// Test 2: Unbalanced transaction should be blocked.
    /// Postings that don't sum to 0 violate double-entry rule.
    #[test]
    fn test_create_transaction_unbalanced_blocked() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let postings = [
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 500, // Does NOT balance! Sum = -500
            },
        ];

        let result = create_transaction(
            &conn,
            "2024-01-15",
            "Incomplete transaction",
            None,
            &postings,
        );

        assert!(result.is_err(), "Unbalanced transaction should be rejected");

        // Verify error message is i18n key
        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.transactionUnbalanced");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    /// Test 3: Transaction with fewer than 2 postings should be blocked.
    #[test]
    fn test_create_transaction_single_posting_blocked() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");

        let postings = [PostingInput {
            account_id: asset_id.clone(),
            amount: 1000,
        }];

        let result = create_transaction(
            &conn,
            "2024-01-15",
            "Invalid single posting",
            None,
            &postings,
        );

        assert!(
            result.is_err(),
            "Single posting transaction should be rejected"
        );

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.transactionMinPostings");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    /// Test 4: List transactions should return all non-deleted transactions.
    #[test]
    fn test_list_transactions() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        // Create two transactions
        let _tx1 = create_transaction(
            &conn,
            "2024-01-10",
            "Transaction 1",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -500,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 500,
                },
            ],
        )
        .unwrap();

        let _tx2 = create_transaction(
            &conn,
            "2024-01-20",
            "Transaction 2",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let transactions = list_transactions(&conn, None, None, None).unwrap();
        assert_eq!(transactions.len(), 2);

        // Verify ordering (by date descending, then id descending)
        assert_eq!(transactions[0].transaction.date, "2024-01-20");
        assert_eq!(transactions[1].transaction.date, "2024-01-10");
    }

    /// Test 5: Get transaction by ID should return transaction with postings.
    #[test]
    fn test_get_transaction_by_id() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Test transaction",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -2000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 2000,
                },
            ],
        )
        .unwrap();

        let result = get_transaction_with_postings(&conn, &tx_id).unwrap();
        assert!(result.is_some());

        let tx = result.unwrap();
        assert_eq!(tx.transaction.id, tx_id);
        assert_eq!(tx.transaction.description, "Test transaction");
        assert_eq!(tx.postings.len(), 2);
    }

    /// Test 6: Get non-existent transaction should return None.
    #[test]
    fn test_get_transaction_nonexistent() {
        let (_dir, conn) = test_conn();

        let result = get_transaction_with_postings(&conn, "nonexistent-id").unwrap();
        assert!(result.is_none());
    }

    /// Test 7: List transactions filtered by account_id.
    #[test]
    fn test_list_transactions_by_account() {
        let (_dir, conn) = test_conn();
        let cash_id = make_account(&conn, "Cash", "asset");
        let bank_id = make_account(&conn, "Bank", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        // Transaction 1 involves Cash
        let _tx1 = create_transaction(
            &conn,
            "2024-01-10",
            "Cash expense",
            None,
            &[
                PostingInput {
                    account_id: cash_id.clone(),
                    amount: -100,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 100,
                },
            ],
        )
        .unwrap();

        // Transaction 2 involves Bank
        let _tx2 = create_transaction(
            &conn,
            "2024-01-20",
            "Bank expense",
            None,
            &[
                PostingInput {
                    account_id: bank_id.clone(),
                    amount: -200,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 200,
                },
            ],
        )
        .unwrap();

        // Filter by Cash account
        let cash_txs = list_transactions(&conn, Some(&cash_id), None, None).unwrap();
        assert_eq!(cash_txs.len(), 1);
        assert_eq!(cash_txs[0].transaction.description, "Cash expense");

        // Filter by Bank account
        let bank_txs = list_transactions(&conn, Some(&bank_id), None, None).unwrap();
        assert_eq!(bank_txs.len(), 1);
        assert_eq!(bank_txs[0].transaction.description, "Bank expense");
    }

    /// Test 8: List transactions filtered by date range.
    #[test]
    fn test_list_transactions_by_date_range() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let _tx1 = create_transaction(
            &conn,
            "2024-01-05",
            "Early transaction",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -100,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 100,
                },
            ],
        )
        .unwrap();

        let _tx2 = create_transaction(
            &conn,
            "2024-01-15",
            "Middle transaction",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -200,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 200,
                },
            ],
        )
        .unwrap();

        let _tx3 = create_transaction(
            &conn,
            "2024-01-25",
            "Late transaction",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -300,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 300,
                },
            ],
        )
        .unwrap();

        // Filter by date range (2024-01-10 to 2024-01-20)
        let filtered =
            list_transactions(&conn, None, Some("2024-01-10"), Some("2024-01-20")).unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].transaction.description, "Middle transaction");

        // Filter by from_date only
        let from_filtered = list_transactions(&conn, None, Some("2024-01-15"), None).unwrap();
        assert_eq!(from_filtered.len(), 2);

        // Filter by to_date only
        let to_filtered = list_transactions(&conn, None, None, Some("2024-01-15")).unwrap();
        assert_eq!(to_filtered.len(), 2);
    }

    /// Test 9: Transaction with category_id.
    #[test]
    fn test_create_transaction_with_category() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        // Create a category
        let cat_id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO categories (id, name, type, created_at, updated_at) VALUES (?1, 'Groceries', 'expense', ?2, ?2)",
            rusqlite::params![&cat_id, &now],
        )
        .unwrap();

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Grocery shopping",
            Some(&cat_id),
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -500,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 500,
                },
            ],
        )
        .unwrap();

        let tx = get_transaction_with_postings(&conn, &tx_id)
            .unwrap()
            .expect("Transaction should exist");

        assert_eq!(tx.transaction.category_id, Some(cat_id));
    }

    /// Test 10: Zero-sum transaction (all postings sum to 0).
    #[test]
    fn test_create_transaction_zero_sum() {
        let (_dir, conn) = test_conn();
        let asset1_id = make_account(&conn, "Cash", "asset");
        let asset2_id = make_account(&conn, "Bank", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        // Three postings that sum to 0: -500 + 300 + 200 = 0
        let postings = [
            PostingInput {
                account_id: asset1_id.clone(),
                amount: -500,
            },
            PostingInput {
                account_id: asset2_id.clone(),
                amount: 300,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 200,
            },
        ];

        let tx_id =
            create_transaction(&conn, "2024-01-15", "Split payment", None, &postings).unwrap();

        let tx = get_transaction_with_postings(&conn, &tx_id)
            .unwrap()
            .expect("Transaction should exist");

        assert_eq!(tx.postings.len(), 3);
        let sum: i64 = tx.postings.iter().map(|p| p.amount).sum();
        assert_eq!(sum, 0);
    }

    /// Test: Transaction with non-existent account_id should fail with ValidationError.
    #[test]
    fn test_create_transaction_invalid_account() {
        let (_dir, conn) = test_conn();

        // Create one valid account
        let asset_id = make_account(&conn, "Cash", "asset");

        // Use a non-existent account_id
        let fake_account_id = Uuid::new_v4().to_string();

        let postings = [
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: fake_account_id.clone(), // Invalid!
                amount: 1000,
            },
        ];

        let result = create_transaction(
            &conn,
            "2024-01-15",
            "Invalid account transaction",
            None,
            &postings,
        );

        assert!(
            result.is_err(),
            "Transaction with invalid account_id should be rejected"
        );

        match result.unwrap_err() {
            AppError::NotFound(msg) => {
                assert!(
                    msg.contains("accountNotFound"),
                    "Error should reference accountNotFound"
                );
            }
            other => panic!("Expected NotFound, got: {:?}", other),
        }
    }

    #[test]
    fn test_create_transaction_duplicate_account() {
        let (_dir, conn) = test_conn();

        let asset_id = make_account(&conn, "Cash", "asset");

        let postings = [
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: asset_id.clone(),
                amount: 1000,
            },
        ];

        let result = create_transaction(
            &conn,
            "2024-01-15",
            "Duplicate account transaction",
            None,
            &postings,
        );

        assert!(
            result.is_err(),
            "Transaction with duplicate account_id should be rejected"
        );

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert!(
                    msg.contains("duplicateAccount"),
                    "Error should reference duplicateAccount"
                );
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_list_transactions_many_with_postings() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        // Create 50 transactions
        for i in 0..50 {
            let _tx_id = create_transaction(
                &conn,
                &format!("2024-01-{:02}", (i % 28) + 1),
                &format!("Transaction {}", i),
                None,
                &[
                    PostingInput {
                        account_id: asset_id.clone(),
                        amount: -100 * (i + 1) as i64,
                    },
                    PostingInput {
                        account_id: expense_id.clone(),
                        amount: 100 * (i + 1) as i64,
                    },
                ],
            )
            .unwrap();
        }

        let transactions = list_transactions(&conn, None, None, None).unwrap();

        // Verify all 50 transactions loaded
        assert_eq!(transactions.len(), 50);

        // Verify each transaction has 2 postings
        for tx in &transactions {
            assert_eq!(tx.postings.len(), 2);
            // Verify postings are correctly associated
            assert!(tx
                .postings
                .iter()
                .all(|p| p.transaction_id == tx.transaction.id));
        }
    }

    // ---------------------------------------------------------------------------
    // QuickAdd Tests
    // ---------------------------------------------------------------------------

    /// Test 11: QuickAdd expense with category should succeed
    #[test]
    fn test_quick_add_expense_with_category() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let input = QuickAddInput {
            mode: "expense".to_string(),
            amount: "35.00".to_string(),
            category_id: None,
            source_account_id: Some(asset_id.clone()),
            destination_account_id: Some(expense_id.clone()),
            description: Some("Lunch".to_string()),
            date: None,
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_ok());

        let tx = result.unwrap();
        assert_eq!(tx.postings.len(), 2);

        let sum: i64 = tx.postings.iter().map(|p| p.amount).sum();
        assert_eq!(sum, 0);
    }

    /// Test 12: QuickAdd transfer to same account should fail
    #[test]
    fn test_quick_add_transfer_same_account_fails() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");

        let input = QuickAddInput {
            mode: "transfer".to_string(),
            amount: "100.00".to_string(),
            source_account_id: Some(asset_id.clone()),
            destination_account_id: Some(asset_id.clone()),
            category_id: None,
            description: Some("Self transfer".to_string()),
            date: None,
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_err());
    }

    /// Test 13: QuickAdd negative amount should fail
    #[test]
    fn test_quick_add_negative_amount_fails() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let input = QuickAddInput {
            mode: "expense".to_string(),
            amount: "-35.00".to_string(),
            source_account_id: Some(asset_id.clone()),
            destination_account_id: Some(expense_id.clone()),
            category_id: None,
            description: Some("Lunch".to_string()),
            date: None,
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_err());
    }

    /// Test: QuickAdd expense without source account should fail (RED test for bug fix)
    /// This tests the validation fix - expense MUST have source_account_id
    #[test]
    fn test_quick_add_expense_without_source_fails() {
        let (_dir, conn) = test_conn();
        let expense_id = make_account(&conn, "Food", "expense");
        let category_id = make_category(&conn, "Groceries", "expense");

        // Expense with destination account and category, but NO source account
        // This should FAIL because source_account_id is required for expenses
        let input = QuickAddInput {
            mode: "expense".to_string(),
            amount: "35.00".to_string(),
            source_account_id: None, // MISSING - this is the bug being tested
            destination_account_id: Some(expense_id.clone()),
            category_id: Some(category_id.clone()),
            description: Some("Lunch".to_string()),
            date: None,
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(
            result.is_err(),
            "Expense without source account should be rejected"
        );

        // Verify error message is correct i18n key
        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(
                    msg, "errors.transaction.expenseRequiresSource",
                    "Should return expenseRequiresSource error"
                );
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    /// Test: QuickAdd expense without destination or category should fail (RED test)
    /// This tests the validation fix - expense MUST have either destination_account_id OR category_id
    #[test]
    fn test_quick_add_expense_without_destination_or_category_fails() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");

        // Expense with source account, but NO destination or category
        // This should FAIL because expense needs a destination
        let input = QuickAddInput {
            mode: "expense".to_string(),
            amount: "35.00".to_string(),
            source_account_id: Some(asset_id.clone()), // Source provided
            destination_account_id: None,              // MISSING destination
            category_id: None,                         // MISSING category
            description: Some("Lunch".to_string()),
            date: None,
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(
            result.is_err(),
            "Expense without destination or category should be rejected"
        );

        // Verify error message is correct i18n key
        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(
                    msg, "errors.transaction.expenseRequiresDestinationOrCategory",
                    "Should return expenseRequiresDestinationOrCategory error"
                );
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    // ---------------------------------------------------------------------------
    // Transaction Detail Tests
    // ---------------------------------------------------------------------------

    /// Test 14: Get transaction detail with all posting information
    #[test]
    fn test_get_transaction_detail() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Grocery shopping",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -3500,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 3500,
                },
            ],
        )
        .unwrap();

        let detail = get_transaction_detail(&conn, &tx_id).unwrap();

        assert_eq!(detail.id, tx_id);
        assert_eq!(detail.description, "Grocery shopping");
        assert_eq!(detail.postings.len(), 2);
        assert!(detail.is_balanced);
        assert_eq!(detail.debit_total, 3500);
        assert_eq!(detail.credit_total, -3500);

        // Postings sorted by amount ASC: -3500 first, +3500 second
        assert!(!detail.postings[0].is_debit);
        assert!(detail.postings[1].is_debit);
    }

    /// Test 15: Get transaction detail for non-existent transaction
    #[test]
    fn test_get_transaction_detail_not_found() {
        let (_dir, conn) = test_conn();

        let result = get_transaction_detail(&conn, "nonexistent-id");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::NotFound(msg) => {
                assert_eq!(msg, "errors.transaction.notFound");
            }
            other => panic!("Expected NotFound error, got: {:?}", other),
        }
    }

    // ---------------------------------------------------------------------------
    // Update Transaction Tests
    // ---------------------------------------------------------------------------

    /// Test 16: Update transaction description
    #[test]
    fn test_update_transaction_description() {
        let (_dir, mut conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Old description",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let input = UpdateTransactionInput {
            description: Some("New description".to_string()),
            ..Default::default()
        };

        let result = update_transaction(&mut conn, &tx_id, &input).unwrap();
        assert_eq!(result.description, "New description");
        assert_eq!(result.date, "2024-01-15");
        assert_eq!(result.postings.len(), 2);
    }

    /// Test 17: Update transaction with unbalanced postings fails
    #[test]
    fn test_update_transaction_unbalanced_fails() {
        let (_dir, mut conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Test",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let input = UpdateTransactionInput {
            postings: Some(vec![
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: 5000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: -3000,
                },
            ]),
            ..Default::default()
        };

        let result = update_transaction(&mut conn, &tx_id, &input);
        assert!(result.is_err());
    }

    /// Test 18: Update transaction with empty input fails
    #[test]
    fn test_update_transaction_empty_input_fails() {
        let (_dir, mut conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Test",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let input = UpdateTransactionInput::default();

        let result = update_transaction(&mut conn, &tx_id, &input);
        assert!(result.is_err());
    }

    /// Test 19: Update transaction with __clear__ marker clears category_id to NULL
    #[test]
    fn test_update_transaction_clear_category() {
        let (_dir, mut conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");
        let category_id = make_category(&conn, "Groceries", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Test with category",
            Some(&category_id),
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let tx_before = get_transaction_with_postings(&conn, &tx_id)
            .unwrap()
            .expect("Transaction should exist");
        assert_eq!(
            tx_before.transaction.category_id,
            Some(category_id.clone()),
            "Category should be set initially"
        );

        let input = UpdateTransactionInput {
            category_id: Some("__clear__".to_string()),
            ..Default::default()
        };

        let result = update_transaction(&mut conn, &tx_id, &input).unwrap();

        assert_eq!(
            result.category_id, None,
            "Category should be cleared with __clear__ marker"
        );

        let category_in_db: Option<String> = conn
            .query_row(
                "SELECT category_id FROM transactions WHERE id = ?1",
                [&tx_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            category_in_db, None,
            "Database should have NULL category_id after clearing"
        );
    }

    // ---------------------------------------------------------------------------
    // Delete Transaction Tests
    // ---------------------------------------------------------------------------

    /// Test 19: Delete transaction succeeds
    #[test]
    fn test_delete_transaction() {
        let (_dir, mut conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "To delete",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        delete_transaction(&mut conn, &tx_id).unwrap();

        let exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
                [&tx_id],
                |row| row.get(0),
            )
            .unwrap();
        assert!(!exists);

        let posting_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
                [&tx_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(posting_count, 0);
    }

    /// Test 20: Delete non-existent transaction fails
    #[test]
    fn test_delete_transaction_not_found() {
        let (_dir, mut conn) = test_conn();

        let result = delete_transaction(&mut conn, "nonexistent-id");
        assert!(result.is_err());
    }

    /// Test 21: Get delete preview
    #[test]
    fn test_get_delete_preview() {
        let (_dir, conn) = test_conn();
        let asset_id = make_account(&conn, "Cash", "asset");
        let expense_id = make_account(&conn, "Food", "expense");

        let tx_id = create_transaction(
            &conn,
            "2024-01-15",
            "Preview test",
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -1000,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 1000,
                },
            ],
        )
        .unwrap();

        let preview = get_delete_preview(&conn, &tx_id).unwrap();

        assert!(preview.can_delete);
        assert_eq!(preview.posting_count, 2);
        assert_eq!(preview.description, "Preview test");
    }
}
