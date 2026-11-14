//! Read capabilities (design doc 25, §3.1).
//!
//! Thin views over the existing `db::` layer — no SQL here, no aggregation
//! re-implementation. `period_summary` reuses `db::reports` so the capability
//! layer and the reports page can never disagree about a total.

use rusqlite::Connection;

use crate::db::{accounts, categories, reports, transactions, AppError};

use super::types::{AccountView, BalanceView, CategoryView, LedgerQuery, SummaryView};

/// Validate a `YYYY-MM-DD` business date.
pub(in crate::ledger) fn validate_date(date: &str) -> Result<(), AppError> {
    chrono::NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| AppError::ValidationError("errors.ledger.invalidDate".to_string()))
}

/// All accounts with their current balances (liabilities are negative).
pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountView>, AppError> {
    let records = accounts::list_accounts(conn)?;
    let ids: Vec<String> = records.iter().map(|a| a.id.clone()).collect();
    let balances = accounts::get_account_balances_batch(conn, &ids)?;

    Ok(records
        .into_iter()
        .map(|a| {
            let balance = balances
                .iter()
                .find(|(id, _)| *id == a.id)
                .map(|(_, b)| *b)
                .unwrap_or(0);
            AccountView {
                id: a.id,
                name: a.name,
                account_type: a.account_type,
                is_active: a.is_active,
                include_net_worth: a.include_net_worth,
                balance_cents: balance,
            }
        })
        .collect())
}

/// All categories with their type.
pub fn list_categories(conn: &Connection) -> Result<Vec<CategoryView>, AppError> {
    Ok(categories::list_categories(conn)?
        .into_iter()
        .map(|c| CategoryView {
            id: c.id,
            name: c.name,
            category_type: c.category_type,
            icon: c.icon,
        })
        .collect())
}

/// Account balances only (id, name, cents).
pub fn account_balances(conn: &Connection) -> Result<Vec<BalanceView>, AppError> {
    Ok(list_accounts(conn)?
        .into_iter()
        .map(|a| BalanceView {
            account_id: a.id,
            account_name: a.name,
            balance_cents: a.balance_cents,
        })
        .collect())
}

/// Query transactions by date range, account, category or description.
///
/// Delegates to the paginated list query, which computes every display
/// amount on the backend (AGENTS.md hard rule). `page` defaults to 1,
/// `page_size` to 50 (capped at 100 by the underlying query).
pub fn query_transactions(
    conn: &Connection,
    q: &LedgerQuery,
) -> Result<Vec<transactions::TransactionListItem>, AppError> {
    if let Some(ref from) = q.from_date {
        validate_date(from)?;
    }
    if let Some(ref to) = q.to_date {
        validate_date(to)?;
    }

    let filter = transactions::TransactionFilter {
        from_date: q.from_date.clone(),
        to_date: q.to_date.clone(),
        min_amount: None,
        max_amount: None,
        account_id: q.account_id.clone(),
        category_id: q.category_id.clone(),
        description_query: q.description_query.clone(),
        page: q.page,
        page_size: q.page_size,
        sort_by: None,
        sort_order: None,
    };

    Ok(transactions::list_transactions_paginated(conn, &filter)?.items)
}

/// Income / expense / net totals for a closed date range (inclusive).
///
/// Reuses the standard report so the totals match the reports page exactly.
pub fn period_summary(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> Result<SummaryView, AppError> {
    validate_date(start_date)?;
    validate_date(end_date)?;

    let filter = reports::ReportFilter {
        date_range_preset: reports::DateRangePreset::Custom,
        start_date: Some(start_date.to_string()),
        end_date: Some(end_date.to_string()),
        period_granularity: reports::PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
        compare: reports::CompareMode::None,
    };

    let dto = reports::get_standard_report(conn, &filter).map_err(AppError::DatabaseError)?;

    Ok(SummaryView {
        start_date: start_date.to_string(),
        end_date: end_date.to_string(),
        income_cents: dto.period_income,
        expense_cents: dto.period_expense,
        net_cents: dto.period_income - dto.period_expense,
    })
}
