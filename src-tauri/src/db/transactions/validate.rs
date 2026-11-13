//! Validation helpers for postings and quick-add input.

use rusqlite::Connection;

use crate::db::AppError;
use crate::utils::money::parse_amount_to_cents;

use super::types::QuickAddInput;

pub(in crate::db::transactions) fn validate_accounts_active(
    conn: &Connection,
    account_ids: &[String],
) -> Result<(), AppError> {
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

pub(in crate::db::transactions) fn validate_no_equity_accounts(
    conn: &Connection,
    account_ids: &[String],
) -> Result<(), AppError> {
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

/// Format cents to display string (e.g., "+¥35.00" or "-¥35.00").
#[allow(dead_code)]
pub(in crate::db::transactions) fn format_amount_display(amount: i64) -> String {
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
pub(in crate::db::transactions) fn validate_quick_add_input(
    input: &QuickAddInput,
) -> Result<i64, AppError> {
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
