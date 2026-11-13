//! Quick-add flow: category/account resolution and posting derivation.

use rusqlite::Connection;

use crate::db::AppError;

use super::create::create_transaction;
use super::query::get_transaction_with_postings;
use super::types::{PostingInput, QuickAddInput, TransactionWithPostings};
use super::validate::validate_quick_add_input;

#[allow(dead_code)]
pub fn quick_add_transaction(
    conn: &Connection,
    input: &QuickAddInput,
) -> Result<TransactionWithPostings, AppError> {
    quick_add_with_category(conn, input, None)
}

/// Write a quick-add transaction atomically.
///
/// Category resolution/creation (AI path), the derived account creation, the
/// transaction row and all postings share one transaction. This is the single
/// transaction boundary for every quick-add caller; command layers must not
/// open their own, otherwise `unchecked_transaction()` would nest.
pub fn quick_add_with_category(
    conn: &Connection,
    input: &QuickAddInput,
    category: Option<(&str, &crate::db::categories::CategoryType)>,
) -> Result<TransactionWithPostings, AppError> {
    let tx = conn.unchecked_transaction()?;

    let mut resolved = input.clone();
    if let Some((name, category_type)) = category {
        resolved.category_id = Some(crate::db::categories::resolve_or_create(
            &tx,
            name,
            category_type,
        )?);
    }

    let amount_cents = validate_quick_add_input(&resolved)?;
    let postings = derive_postings(&tx, &resolved, amount_cents)?;

    let date = resolved
        .date
        .clone()
        .unwrap_or_else(crate::utils::time::today_date);
    let description = resolved.description.clone().unwrap_or_default();

    let tx_id = create_transaction(&tx, &date, &description, &postings)?;

    let result = get_transaction_with_postings(&tx, &tx_id)?
        .ok_or_else(|| AppError::DatabaseError("Created transaction not found".to_string()))?;
    tx.commit()?;
    Ok(result)
}

fn ensure_category_account(conn: &Connection, category_id: &str) -> Result<String, AppError> {
    // The category's mirror account is recorded on the row itself. Looking it
    // up by name is what used to create a second account after every rename.
    crate::db::categories::account_id_for_category(conn, category_id)
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
