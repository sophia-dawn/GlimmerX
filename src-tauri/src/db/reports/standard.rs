//! Standard overview report: period totals, comparison and account changes.

use rusqlite::{params, Connection};

use crate::constants::account_type;

use super::common::{get_expense_sum, get_income_sum, get_prev_period_bounds, resolve_date_range};
use super::types::{AccountChange, NetWorthPoint, ReportFilter, StandardReportDto};

pub fn get_standard_report(
    conn: &Connection,
    filter: &ReportFilter,
) -> Result<StandardReportDto, String> {
    let (start_date, end_date) = resolve_date_range(filter);
    let account_ids_json = filter
        .account_ids
        .as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string()));

    let period_income = get_income_sum(conn, &start_date, &end_date, account_ids_json.as_deref())?;
    let period_expense =
        get_expense_sum(conn, &start_date, &end_date, account_ids_json.as_deref())?;

    let prev_bounds = get_prev_period_bounds(&filter.date_range_preset);
    let prev_income = get_income_sum(
        conn,
        &prev_bounds.0,
        &prev_bounds.1,
        account_ids_json.as_deref(),
    )?;
    let prev_expense = get_expense_sum(
        conn,
        &prev_bounds.0,
        &prev_bounds.1,
        account_ids_json.as_deref(),
    )?;

    let income_diff = period_income - prev_income;
    let income_change_pct = if prev_income > 0 {
        (income_diff as f64 / prev_income as f64) * 100.0
    } else if income_diff > 0 {
        100.0 // New income appeared when previous period had none
    } else {
        0.0
    };

    let expense_diff = period_expense - prev_expense;
    let expense_change_pct = if prev_expense > 0 {
        (expense_diff as f64 / prev_expense as f64) * 100.0
    } else if expense_diff > 0 {
        100.0 // New expense appeared when previous period had none
    } else {
        0.0
    };

    let net_worth_trend = get_net_worth_trend(conn, &start_date, &end_date)?;
    let account_changes =
        get_account_changes(conn, &start_date, &end_date, account_ids_json.as_deref())?;

    Ok(StandardReportDto {
        period_income,
        period_expense,
        prev_income,
        prev_expense,
        income_change_pct,
        expense_change_pct,
        net_worth_trend,
        account_changes,
    })
}

/// Retrieves net worth trend points for the given period.
// Net worth = Assets + Liabilities (only Balance Sheet accounts)
fn get_net_worth_trend(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<Vec<NetWorthPoint>, String> {
    let sql = format!(
        "
        SELECT
          t.date,
          (
            SELECT COALESCE(SUM(p2.amount), 0)
            FROM postings p2
            JOIN accounts a2 ON a2.id = p2.account_id
            JOIN transactions t2 ON t2.id = p2.transaction_id
            WHERE a2.type IN ('{}', '{}')
              AND t2.date <= t.date
          ) as net_worth
        FROM transactions t
        WHERE t.date >= ?1 AND t.date <= ?2
        GROUP BY t.date
        ORDER BY t.date
    ",
        account_type::ASSET,
        account_type::LIABILITY
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let points = stmt
        .query_map(params![start, end], |row| {
            Ok(NetWorthPoint {
                date: row.get(0)?,
                net_worth: row.get(1)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(points)
}

/// Retrieves account balance changes for the given period.
// Only Asset and Liability accounts have persistent balances (Balance Sheet accounts)
fn get_account_changes(
    conn: &Connection,
    start: &str,
    end: &str,
    account_ids: Option<&str>,
) -> Result<Vec<AccountChange>, String> {
    let sql = format!(
        "
        SELECT
          a.id,
          a.name,
          a.type,
          (
            SELECT COALESCE(SUM(p2.amount), 0)
            FROM postings p2
            JOIN transactions t2 ON t2.id = p2.transaction_id
            WHERE p2.account_id = a.id
              AND t2.date < ?1
          ) as start_balance,
          (
            SELECT COALESCE(SUM(p3.amount), 0)
            FROM postings p3
            JOIN transactions t3 ON t3.id = p3.transaction_id
            WHERE p3.account_id = a.id
              AND t3.date <= ?2
          ) as end_balance
        FROM accounts a
        WHERE a.type IN ('{}', '{}')
          AND a.is_active = 1
          AND (?3 IS NULL OR a.id IN (SELECT value FROM json_each(?3)))
        ORDER BY a.type, a.name
    ",
        account_type::ASSET,
        account_type::LIABILITY
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let changes = stmt
        .query_map(params![start, end, account_ids], |row| {
            let start_balance: i64 = row.get(3)?;
            let end_balance: i64 = row.get(4)?;
            Ok(AccountChange {
                account_id: row.get(0)?,
                account_name: row.get(1)?,
                account_type: row.get(2)?,
                start_balance,
                end_balance,
                change: end_balance - start_balance,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(changes)
}
