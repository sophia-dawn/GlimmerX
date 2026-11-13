//! Trend, category breakdown, balance sheet and year summary reports.

use std::collections::HashMap;

use rusqlite::{params, Connection};

use crate::constants::account_type;
use crate::utils::time::year_bounds;

use super::common::{get_expense_sum, get_income_sum, get_period_format, resolve_date_range};
use super::types::{
    BalanceSheetItem, BalanceSheetReportDto, CategoryAmount, CategoryBreakdownItem,
    CategoryBreakdownReportDto, MonthlySummary, ReportFilter, TrendPoint, TrendReportDto,
    YearSummaryReportDto,
};

/// Retrieves trend report showing income/expense patterns over time.
pub fn get_trend_report(
    conn: &Connection,
    filter: &ReportFilter,
) -> Result<TrendReportDto, String> {
    let (start_date, end_date) = resolve_date_range(filter);
    let category_ids_json = filter
        .category_ids
        .as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string()));

    let period_format = get_period_format(&filter.period_granularity);

    let sql = format!(
        "
        SELECT
            strftime('{}', t.date) as period,
            MIN(t.date) as period_start,
            MAX(t.date) as period_end,
            COALESCE(SUM(CASE WHEN a.type = '{}' AND p.amount < 0 THEN -p.amount ELSE 0 END), 0) as income,
            COALESCE(SUM(CASE WHEN a.type = '{}' AND p.amount > 0 THEN p.amount ELSE 0 END), 0) as expense
        FROM postings p
        JOIN accounts a ON a.id = p.account_id
        JOIN transactions t ON t.id = p.transaction_id
        WHERE t.date >= ?1 AND t.date <= ?2
          AND (?3 IS NULL OR t.category_id IN (SELECT value FROM json_each(?3)))
        GROUP BY strftime('{}', t.date)
        ORDER BY period
        ",
        period_format, account_type::INCOME, account_type::EXPENSE, period_format
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let points = stmt
        .query_map(
            params![start_date, end_date, category_ids_json.as_deref()],
            |row| {
                let income: i64 = row.get(3)?;
                let expense: i64 = row.get(4)?;
                Ok(TrendPoint {
                    period: row.get(0)?,
                    period_start: row.get(1)?,
                    period_end: row.get(2)?,
                    income,
                    expense,
                    net: income - expense,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let total_income: i64 = points.iter().map(|p| p.income).sum();
    let total_expense: i64 = points.iter().map(|p| p.expense).sum();
    let total_net: i64 = points.iter().map(|p| p.net).sum();

    Ok(TrendReportDto {
        granularity: filter.period_granularity.to_string(),
        data_points: points,
        total_income,
        total_expense,
        total_net,
    })
}

/// Retrieves category breakdown report showing spending by category.
pub fn get_category_breakdown_report(
    conn: &Connection,
    filter: &ReportFilter,
    income_or_expense: &str,
) -> Result<CategoryBreakdownReportDto, String> {
    let (start_date, end_date) = resolve_date_range(filter);
    let category_ids_json = filter
        .category_ids
        .as_ref()
        .map(|ids| serde_json::to_string(ids).unwrap_or_else(|_| "[]".to_string()));

    // Income accounts have negative amounts (credit entries per double-entry convention)
    // Expense accounts have positive amounts (debit entries)
    let (amount_condition, amount_expr) = match income_or_expense {
        "income" => ("p.amount < 0", "-SUM(p.amount)"),
        _ => ("p.amount > 0", "SUM(p.amount)"),
    };

    let sql = format!(
        "
        SELECT
            c.id as category_id,
            c.name as category_name,
            {} as amount,
            COUNT(DISTINCT t.id) as transaction_count
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id
        JOIN accounts a ON a.id = p.account_id AND a.type = ?1
        JOIN categories c ON c.id = t.category_id AND c.type = ?1
        WHERE t.date >= ?2 AND t.date <= ?3
          AND {}
          AND (?4 IS NULL OR c.id IN (SELECT value FROM json_each(?4)))
        GROUP BY c.id
        ORDER BY amount DESC
        ",
        amount_expr, amount_condition
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(
            params![
                income_or_expense,
                start_date,
                end_date,
                category_ids_json.as_deref()
            ],
            |row| {
                Ok(CategoryBreakdownItem {
                    category_id: row.get(0)?,
                    category_name: row.get(1)?,
                    amount: row.get(2)?,
                    percentage: 0.0,
                    transaction_count: row.get(3)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let total_amount: i64 = items.iter().map(|i| i.amount).sum();
    let categories: Vec<CategoryBreakdownItem> = items
        .into_iter()
        .map(|mut item| {
            item.percentage = if total_amount > 0 {
                (item.amount as f64 / total_amount as f64) * 100.0
            } else {
                0.0
            };
            item
        })
        .collect();

    Ok(CategoryBreakdownReportDto {
        total_amount,
        categories,
    })
}

/// Retrieves balance sheet report at a specific snapshot date.
pub fn get_balance_sheet_report(
    conn: &Connection,
    snapshot_date: &str,
) -> Result<BalanceSheetReportDto, String> {
    let sql = format!(
        "
        SELECT
            a.id as account_id,
            a.name as account_name,
            a.type as account_type,
            (
                SELECT COALESCE(SUM(p2.amount), 0)
                FROM postings p2
                JOIN transactions t2 ON t2.id = p2.transaction_id
                WHERE p2.account_id = a.id
                  AND t2.date <= ?1
            ) as balance
        FROM accounts a
        WHERE a.type IN ('{}', '{}')
          AND a.is_active = 1
        ORDER BY a.type, a.name
    ",
        account_type::ASSET,
        account_type::LIABILITY
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![snapshot_date], |row| {
            Ok(BalanceSheetItem {
                account_id: row.get(0)?,
                account_name: row.get(1)?,
                account_type: row.get(2)?,
                balance: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let assets: Vec<BalanceSheetItem> = items
        .iter()
        .filter(|i| i.account_type == account_type::ASSET)
        .cloned()
        .collect();
    let liabilities: Vec<BalanceSheetItem> = items
        .iter()
        .filter(|i| i.account_type == account_type::LIABILITY)
        .cloned()
        .collect();

    let total_assets: i64 = assets.iter().map(|i| i.balance).sum();
    let total_liabilities: i64 = liabilities.iter().map(|i| i.balance).sum();
    let net_worth = total_assets + total_liabilities;

    Ok(BalanceSheetReportDto {
        snapshot_date: snapshot_date.to_string(),
        assets,
        liabilities,
        total_assets,
        total_liabilities,
        net_worth,
    })
}

/// Retrieves year summary report with monthly breakdown and top categories.
pub fn get_year_summary_report(
    conn: &Connection,
    year: i32,
) -> Result<YearSummaryReportDto, String> {
    let (start_date, end_date) = year_bounds(year);

    let total_income = get_income_sum(conn, &start_date, &end_date, None)?;
    let total_expense = get_expense_sum(conn, &start_date, &end_date, None)?;

    let monthly_sql = format!(
        "
        SELECT
            CAST(strftime('%m', t.date) AS INTEGER) as month,
            COALESCE(SUM(CASE WHEN a.type = '{}' AND p.amount < 0 THEN -p.amount ELSE 0 END), 0) as income,
            COALESCE(SUM(CASE WHEN a.type = '{}' AND p.amount > 0 THEN p.amount ELSE 0 END), 0) as expense
        FROM postings p
        JOIN accounts a ON a.id = p.account_id
        JOIN transactions t ON t.id = p.transaction_id
        WHERE strftime('%Y', t.date) = ?1
        GROUP BY strftime('%m', t.date)
        ORDER BY month
    ",
        account_type::INCOME, account_type::EXPENSE
    );

    let monthly_map: HashMap<u32, (i64, i64)> = conn
        .prepare(&monthly_sql)
        .map_err(|e| e.to_string())?
        .query_map(params![year.to_string()], |row| {
            Ok((
                row.get::<_, u32>(0)?,
                (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?),
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let monthly_breakdown: Vec<MonthlySummary> = (1..=12)
        .map(|month| {
            let (income, expense) = monthly_map.get(&month).copied().unwrap_or((0, 0));
            MonthlySummary {
                month,
                income,
                expense,
                net: income - expense,
            }
        })
        .collect();

    let top_income_sql = format!(
        "
        SELECT c.id, c.name, -SUM(p.amount) as amount
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id
        JOIN accounts a ON a.id = p.account_id AND a.type = '{}'
        JOIN categories c ON c.id = t.category_id
        WHERE t.date >= ?1 AND t.date <= ?2 AND p.amount < 0
        GROUP BY c.id
        ORDER BY amount DESC
        LIMIT 5
    ",
        account_type::INCOME
    );
    let top_income_categories =
        get_top_categories(conn, &top_income_sql, &start_date, &end_date, total_income)?;

    let top_expense_sql = format!(
        "
        SELECT c.id, c.name, SUM(p.amount) as amount
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id
        JOIN accounts a ON a.id = p.account_id AND a.type = '{}'
        JOIN categories c ON c.id = t.category_id
        WHERE t.date >= ?1 AND t.date <= ?2 AND p.amount > 0
        GROUP BY c.id
        ORDER BY amount DESC
        LIMIT 5
    ",
        account_type::EXPENSE
    );
    let top_expense_categories = get_top_categories(
        conn,
        &top_expense_sql,
        &start_date,
        &end_date,
        total_expense,
    )?;

    Ok(YearSummaryReportDto {
        year,
        total_income,
        total_expense,
        net: total_income - total_expense,
        monthly_breakdown,
        top_income_categories,
        top_expense_categories,
    })
}

fn get_top_categories(
    conn: &Connection,
    sql: &str,
    start: &str,
    end: &str,
    total: i64,
) -> Result<Vec<CategoryAmount>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![start, end], |row| {
            let amount: i64 = row.get(2)?;
            Ok(CategoryAmount {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                amount,
                percentage: if total > 0 {
                    (amount as f64 / total as f64) * 100.0
                } else {
                    0.0
                },
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}
