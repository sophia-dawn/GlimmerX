use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::constants::account_type;
use crate::utils::time::*;

// ---------------------------------------------------------------------------
// Report filter types
// ---------------------------------------------------------------------------

/// Filter parameters for generating reports.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFilter {
    pub date_range_preset: DateRangePreset,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period_granularity: PeriodGranularity,
    pub account_ids: Option<Vec<String>>,
    pub category_ids: Option<Vec<String>>,
}

/// Preset date ranges for report filtering.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum DateRangePreset {
    CurrentMonth,
    LastMonth,
    CurrentYear,
    LastYear,
    Last3Months,
    Last6Months,
    Last12Months,
    Custom,
}

/// Granularity for aggregating report data by time period.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PeriodGranularity {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl std::fmt::Display for PeriodGranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeriodGranularity::Daily => write!(f, "daily"),
            PeriodGranularity::Weekly => write!(f, "weekly"),
            PeriodGranularity::Monthly => write!(f, "monthly"),
            PeriodGranularity::Yearly => write!(f, "yearly"),
        }
    }
}

// ---------------------------------------------------------------------------
// Report DTOs
// ---------------------------------------------------------------------------

/// Standard overview report with period comparison.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardReportDto {
    pub period_income: i64,
    pub period_expense: i64,
    pub prev_income: i64,
    pub prev_expense: i64,
    pub income_change_pct: f64,
    pub expense_change_pct: f64,
    pub net_worth_trend: Vec<NetWorthPoint>,
    pub account_changes: Vec<AccountChange>,
}

/// Single point in the net worth trend over time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthPoint {
    pub date: String,
    pub net_worth: i64,
}

/// Account balance change over the report period.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountChange {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub start_balance: i64,
    pub end_balance: i64,
    pub change: i64,
}

/// Category spending breakdown report.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownReportDto {
    pub total_amount: i64,
    pub categories: Vec<CategoryBreakdownItem>,
}

/// Single category breakdown with spending details.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownItem {
    pub category_id: String,
    pub category_name: String,
    pub amount: i64,
    pub percentage: f64,
    pub transaction_count: i64,
}

/// Balance sheet report showing assets, liabilities, and net worth.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheetReportDto {
    pub snapshot_date: String,
    pub assets: Vec<BalanceSheetItem>,
    pub liabilities: Vec<BalanceSheetItem>,
    pub total_assets: i64,
    pub total_liabilities: i64,
    pub net_worth: i64,
}

/// Single account entry in the balance sheet.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheetItem {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub balance: i64,
}

/// Trend report showing income/expense patterns over time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendReportDto {
    pub granularity: String,
    pub data_points: Vec<TrendPoint>,
    pub total_income: i64,
    pub total_expense: i64,
    pub total_net: i64,
}

/// Single data point in a trend report.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub period: String,
    pub period_start: String,
    pub period_end: String,
    pub income: i64,
    pub expense: i64,
    pub net: i64,
}

/// Month comparison report comparing two months side-by-side.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthComparisonReportDto {
    pub month1: String,
    pub month2: String,
    pub month1_income: i64,
    pub month1_expense: i64,
    pub month2_income: i64,
    pub month2_expense: i64,
    pub income_diff: i64,
    pub expense_diff: i64,
    pub income_change_pct: f64,
    pub expense_change_pct: f64,
    pub category_comparison: Vec<CategoryComparisonItem>,
}

/// Single category comparison between two months.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryComparisonItem {
    pub category_id: String,
    pub category_name: String,
    pub month1_amount: i64,
    pub month2_amount: i64,
    pub diff: i64,
    pub change_pct: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearSummaryReportDto {
    pub year: i32,
    pub total_income: i64,
    pub total_expense: i64,
    pub net: i64,
    pub monthly_breakdown: Vec<MonthlySummary>,
    pub top_income_categories: Vec<CategoryAmount>,
    pub top_expense_categories: Vec<CategoryAmount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySummary {
    pub month: u32,
    pub income: i64,
    pub expense: i64,
    pub net: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAmount {
    pub category_id: String,
    pub category_name: String,
    pub amount: i64,
    pub percentage: f64,
}

// ---------------------------------------------------------------------------
// Audit Report DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditReportDto {
    pub balance_check: BalanceCheckResult,
    pub category_check: CategoryCheckResult,
    pub account_usage: Vec<AccountUsageStat>,
    pub generated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceCheckResult {
    pub total_transactions: u32,
    pub balanced_count: u32,
    pub unbalanced_count: u32,
    pub unbalanced_has_more: bool,
    pub unbalanced_transactions: Vec<UnbalancedTransaction>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbalancedTransaction {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub sum: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCheckResult {
    pub uncategorized_transactions: u32,
    pub uncategorized_has_more: bool,
    pub uncategorized_list: Vec<UncategorizedTransaction>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UncategorizedTransaction {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageStat {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub posting_count: u32,
    pub total_debit: i64,
    pub total_credit: i64,
    pub last_transaction_date: Option<String>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Resolves a date range preset to actual start and end dates.
pub fn resolve_date_range(filter: &ReportFilter) -> (String, String) {
    match filter.date_range_preset {
        DateRangePreset::CurrentMonth => current_month_bounds(),
        DateRangePreset::LastMonth => last_month_bounds(),
        DateRangePreset::CurrentYear => year_bounds(current_year()),
        DateRangePreset::LastYear => last_year_bounds(),
        DateRangePreset::Last3Months => last_n_months_bounds(3),
        DateRangePreset::Last6Months => last_n_months_bounds(6),
        DateRangePreset::Last12Months => last_n_months_bounds(12),
        DateRangePreset::Custom => (
            filter.start_date.clone().unwrap_or_else(today_date),
            filter.end_date.clone().unwrap_or_else(today_date),
        ),
    }
}

/// Returns the strftime format string for a given period granularity.
pub fn get_period_format(granularity: &PeriodGranularity) -> &'static str {
    match granularity {
        PeriodGranularity::Daily => "%Y-%m-%d",
        PeriodGranularity::Weekly => "%Y-%W",
        PeriodGranularity::Monthly => "%Y-%m",
        PeriodGranularity::Yearly => "%Y",
    }
}

// ---------------------------------------------------------------------------
// Database query functions
// ---------------------------------------------------------------------------

/// Retrieves the standard report data for the given filter.
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

/// Calculates total income for the given period.
// Income accounts receive credit entries (negative amounts) per standard double-entry convention
fn get_income_sum(
    conn: &Connection,
    start: &str,
    end: &str,
    account_ids: Option<&str>,
) -> Result<i64, String> {
    let sql = format!(
        "
        SELECT COALESCE(-SUM(p.amount), 0)
        FROM postings p
        JOIN accounts a ON a.id = p.account_id AND a.type = '{}'
        JOIN transactions t ON t.id = p.transaction_id
        WHERE t.date >= ?1 AND t.date <= ?2
          AND t.deleted_at IS NULL
          AND p.amount < 0
          AND (?3 IS NULL OR a.id IN (SELECT value FROM json_each(?3)))
    ",
        account_type::INCOME
    );
    conn.query_row(&sql, params![start, end, account_ids], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Calculates total expense for the given period.
fn get_expense_sum(
    conn: &Connection,
    start: &str,
    end: &str,
    account_ids: Option<&str>,
) -> Result<i64, String> {
    let sql = format!(
        "
        SELECT COALESCE(SUM(p.amount), 0)
        FROM postings p
        JOIN accounts a ON a.id = p.account_id AND a.type = '{}'
        JOIN transactions t ON t.id = p.transaction_id
        WHERE t.date >= ?1 AND t.date <= ?2
          AND t.deleted_at IS NULL
          AND p.amount > 0
          AND (?3 IS NULL OR a.id IN (SELECT value FROM json_each(?3)))
    ",
        account_type::EXPENSE
    );
    conn.query_row(&sql, params![start, end, account_ids], |row| row.get(0))
        .map_err(|e| e.to_string())
}

/// Returns the bounds of the previous period for comparison.
fn get_prev_period_bounds(preset: &DateRangePreset) -> (String, String) {
    match preset {
        DateRangePreset::CurrentMonth => last_month_bounds(),
        DateRangePreset::LastMonth => {
            let year = current_year();
            if current_month() == 1 {
                month_bounds(year - 1, 11)
            } else if current_month() == 2 {
                month_bounds(year - 1, 12)
            } else {
                month_bounds(year, current_month() - 2)
            }
        }
        DateRangePreset::CurrentYear => year_bounds(current_year() - 1),
        DateRangePreset::LastYear => year_bounds(current_year() - 2),
        DateRangePreset::Last3Months => last_n_months_bounds(6),
        DateRangePreset::Last6Months => last_n_months_bounds(12),
        DateRangePreset::Last12Months => last_n_months_bounds(24),
        DateRangePreset::Custom => last_month_bounds(),
    }
}

/// Returns the start and end dates for a given month.
fn month_bounds(year: i32, month: u32) -> (String, String) {
    (month_start(year, month), month_end(year, month))
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
              AND t2.deleted_at IS NULL
          ) as net_worth
        FROM transactions t
        WHERE t.date >= ?1 AND t.date <= ?2
          AND t.deleted_at IS NULL
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
              AND t2.deleted_at IS NULL
          ) as start_balance,
          (
            SELECT COALESCE(SUM(p3.amount), 0)
            FROM postings p3
            JOIN transactions t3 ON t3.id = p3.transaction_id
            WHERE p3.account_id = a.id
              AND t3.date <= ?2
              AND t3.deleted_at IS NULL
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

/// Retrieves month comparison report data.
pub fn get_month_comparison(
    conn: &Connection,
    month1: &str,
    month2: &str,
) -> Result<MonthComparisonReportDto, String> {
    let month1_bounds = parse_month_bounds(month1)?;
    let month2_bounds = parse_month_bounds(month2)?;

    let month1_income = get_income_sum(conn, &month1_bounds.0, &month1_bounds.1, None)?;
    let month1_expense = get_expense_sum(conn, &month1_bounds.0, &month1_bounds.1, None)?;
    let month2_income = get_income_sum(conn, &month2_bounds.0, &month2_bounds.1, None)?;
    let month2_expense = get_expense_sum(conn, &month2_bounds.0, &month2_bounds.1, None)?;

    // Calculate change from month1 to month2 (month2 compared to month1)
    let income_diff = month2_income - month1_income;
    let expense_diff = month2_expense - month1_expense;
    let income_change_pct = if month1_income > 0 {
        (income_diff as f64 / month1_income as f64) * 100.0
    } else if income_diff > 0 {
        100.0
    } else {
        0.0
    };
    let expense_change_pct = if month1_expense > 0 {
        (expense_diff as f64 / month1_expense as f64) * 100.0
    } else if expense_diff > 0 {
        100.0
    } else {
        0.0
    };

    let category_comparison = get_category_comparison(conn, &month1_bounds, &month2_bounds)?;

    Ok(MonthComparisonReportDto {
        month1: month1.to_string(),
        month2: month2.to_string(),
        month1_income,
        month1_expense,
        month2_income,
        month2_expense,
        income_diff,
        expense_diff,
        income_change_pct,
        expense_change_pct,
        category_comparison,
    })
}

fn parse_month_bounds(month: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = month.split('-').collect();
    if parts.len() != 2 {
        return Err("Invalid month format, expected YYYY-MM".to_string());
    }
    let year: i32 = parts[0].parse().map_err(|_| "Invalid year".to_string())?;
    let month_num: u32 = parts[1].parse().map_err(|_| "Invalid month".to_string())?;
    if !(1..=12).contains(&month_num) {
        return Err("Invalid month number, must be 1-12".to_string());
    }
    Ok((month_start(year, month_num), month_end(year, month_num)))
}

fn get_category_comparison(
    conn: &Connection,
    bounds1: &(String, String),
    bounds2: &(String, String),
) -> Result<Vec<CategoryComparisonItem>, String> {
    let sql = format!(
        "
        SELECT
          c.id,
          c.name,
          COALESCE(SUM(CASE WHEN t.date >= ?1 AND t.date <= ?2 THEN p.amount ELSE 0 END), 0) as month1,
          COALESCE(SUM(CASE WHEN t.date >= ?3 AND t.date <= ?4 THEN p.amount ELSE 0 END), 0) as month2
        FROM categories c
        INNER JOIN transactions t ON t.category_id = c.id AND t.deleted_at IS NULL
        INNER JOIN postings p ON p.transaction_id = t.id AND p.amount > 0
        INNER JOIN accounts a ON a.id = p.account_id AND a.type = '{}'
        GROUP BY c.id
        HAVING month1 > 0 OR month2 > 0
        ORDER BY (month1 + month2) DESC
        LIMIT 20
    ",
        account_type::EXPENSE
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let items = stmt
        .query_map(params![bounds1.0, bounds1.1, bounds2.0, bounds2.1], |row| {
            let month1_amount: i64 = row.get(2)?;
            let month2_amount: i64 = row.get(3)?;
            let diff = month2_amount - month1_amount;
            let change_pct = if month1_amount > 0 {
                (diff as f64 / month1_amount as f64) * 100.0
            } else {
                -1.0 // Sentinel value for "new category" (no previous month data)
            };
            Ok(CategoryComparisonItem {
                category_id: row.get(0)?,
                category_name: row.get(1)?,
                month1_amount,
                month2_amount,
                diff,
                change_pct,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(items)
}

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
          AND t.deleted_at IS NULL
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
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
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
                  AND t2.deleted_at IS NULL
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
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
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
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
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
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
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

/// Account transactions report DTOs
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionsReportDto {
    pub account_id: String,
    pub account_name: String,
    pub start_date: String,
    pub end_date: String,
    pub transactions: Vec<AccountTransactionItem>,
    pub total_inflow: i64,
    pub total_outflow: i64,
    pub net_change: i64,
    pub total_count: u32,
    pub total_pages: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionItem {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub category_name: Option<String>,
    pub amount: i64,
    pub is_inflow: bool,
    pub counter_account_name: Option<String>,
}

/// Retrieves account transactions report with pagination.
pub fn get_account_transactions_report(
    conn: &Connection,
    account_id: &str,
    filter: &ReportFilter,
    page: u32,
    page_size: u32,
) -> Result<AccountTransactionsReportDto, String> {
    let (start_date, end_date) = resolve_date_range(filter);
    let offset = (page.saturating_sub(1)) * page_size;

    let account_name: String = conn
        .query_row(
            "SELECT name FROM accounts WHERE id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Count total matching transactions
    let count_sql = "
        SELECT COUNT(DISTINCT t.id)
        FROM transactions t
        JOIN postings p ON p.transaction_id = t.id
        WHERE p.account_id = ?1
          AND t.date >= ?2 AND t.date <= ?3
          AND t.deleted_at IS NULL
    ";
    let total_count: u32 = conn
        .query_row(
            count_sql,
            params![account_id, start_date, end_date],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let total_pages = match total_count.checked_div(page_size) {
        Some(div_result) => div_result + u32::from(!total_count.is_multiple_of(page_size)),
        None => 1,
    };

    let sql = "
        SELECT
            t.id as transaction_id,
            t.date,
            t.description,
            c.name as category_name,
            p.amount,
            (
                SELECT a2.name
                FROM postings p2
                JOIN accounts a2 ON a2.id = p2.account_id AND a2.id != ?1
                WHERE p2.transaction_id = t.id
                LIMIT 1
            ) as counter_account_name
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
        LEFT JOIN categories c ON c.id = t.category_id
        WHERE p.account_id = ?1
          AND t.date >= ?2 AND t.date <= ?3
        ORDER BY t.date DESC, t.created_at DESC
        LIMIT ?4 OFFSET ?5
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let transactions = stmt
        .query_map(
            params![
                account_id,
                start_date,
                end_date,
                page_size as i64,
                offset as i64
            ],
            |row| {
                let amount: i64 = row.get(4)?;
                let is_inflow = amount > 0;
                Ok(AccountTransactionItem {
                    transaction_id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    category_name: row.get(3)?,
                    amount: amount.abs(),
                    is_inflow,
                    counter_account_name: row.get(5)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let totals_sql = "
        SELECT
            COALESCE(SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END), 0) as inflow,
            COALESCE(SUM(CASE WHEN p.amount < 0 THEN ABS(p.amount) ELSE 0 END), 0) as outflow
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
        WHERE p.account_id = ?1
          AND t.date >= ?2 AND t.date <= ?3
    ";
    let (total_inflow, total_outflow): (i64, i64) = conn
        .query_row(
            totals_sql,
            params![account_id, start_date, end_date],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    Ok(AccountTransactionsReportDto {
        account_id: account_id.to_string(),
        account_name,
        start_date,
        end_date,
        transactions,
        total_inflow,
        total_outflow,
        net_change: total_inflow - total_outflow,
        total_count,
        total_pages,
    })
}

/// Account balance trend report DTOs
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceTrendReportDto {
    pub account_id: String,
    pub account_name: String,
    pub granularity: String,
    pub data_points: Vec<BalancePoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePoint {
    pub period: String,
    pub period_start: String,
    pub period_end: String,
    pub balance: i64,
}

/// Retrieves account balance trend report.
///
/// Uses SQL window functions for efficient cumulative balance tracking:
/// - Computes initial balance before the date range
/// - Calculates running balance for each posting within the range
/// - Groups by period and extracts the balance at each period's end
pub fn get_account_balance_trend_report(
    conn: &Connection,
    account_id: &str,
    filter: &ReportFilter,
) -> Result<AccountBalanceTrendReportDto, String> {
    let (start_date, end_date) = resolve_date_range(filter);
    let period_format = get_period_format(&filter.period_granularity);

    let account_name: String = conn
        .query_row(
            "SELECT name FROM accounts WHERE id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Single query using window functions:
    // 1. initial CTE: balance before start_date
    // 2. running CTE: cumulative balance for each posting in range
    // 3. period_info CTE: min/max dates per period
    // 4. Final: join to get balance at each period's end
    let sql = format!(
        "WITH initial AS (
            SELECT COALESCE(SUM(p.amount), 0) as initial_balance
            FROM postings p
            JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
            WHERE p.account_id = ?1 AND t.date < ?2
        ),
        running AS (
            SELECT 
                p.rowid as rn_id,
                strftime('{pf}', t.date) as period,
                t.date,
                (SELECT initial_balance FROM initial) + 
                    SUM(p.amount) OVER (ORDER BY t.date ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) as balance
            FROM postings p
            JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL
            WHERE p.account_id = ?1 AND t.date >= ?2 AND t.date <= ?3
            ORDER BY t.date
        ),
        period_info AS (
            SELECT period, MIN(date) as period_start, MAX(date) as period_end, MAX(rn_id) as last_rn_id
            FROM running
            GROUP BY period
        )
        SELECT 
            pi.period,
            pi.period_start,
            pi.period_end,
            r.balance
        FROM period_info pi
        JOIN running r ON r.rn_id = pi.last_rn_id
        ORDER BY pi.period_start",
        pf = period_format
    );

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let data_points: Vec<BalancePoint> = stmt
        .query_map(params![account_id, start_date, end_date], |row| {
            Ok(BalancePoint {
                period: row.get::<_, String>(0)?,
                period_start: row.get::<_, String>(1)?,
                period_end: row.get::<_, String>(2)?,
                balance: row.get::<_, i64>(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(AccountBalanceTrendReportDto {
        account_id: account_id.to_string(),
        account_name,
        granularity: filter.period_granularity.to_string(),
        data_points,
    })
}

pub fn get_audit_report(conn: &Connection) -> Result<AuditReportDto, String> {
    let balance_check = get_balance_check_result(conn)?;
    let category_check = get_category_check_result(conn)?;
    let account_usage = get_account_usage_stats(conn)?;

    Ok(AuditReportDto {
        balance_check,
        category_check,
        account_usage,
        generated_at: now_rfc3339(),
    })
}

fn get_balance_check_result(conn: &Connection) -> Result<BalanceCheckResult, String> {
    const LIMIT: u32 = 100;

    let total_transactions: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL AND description != 'Opening Balance'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count transactions: {}", e))?;

    let unbalanced_count: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM (
                SELECT t.id
                FROM transactions t
                JOIN postings p ON p.transaction_id = t.id
                WHERE t.deleted_at IS NULL
                  AND t.description != 'Opening Balance'
                GROUP BY t.id
                HAVING SUM(p.amount) != 0
            )",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count unbalanced transactions: {}", e))?;

    let unbalanced_has_more = unbalanced_count > LIMIT;

    let unbalanced_transactions: Vec<UnbalancedTransaction> = conn
        .prepare(
            "SELECT t.id, t.date, t.description, SUM(p.amount) as sum
             FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             WHERE t.deleted_at IS NULL
               AND t.description != 'Opening Balance'
             GROUP BY t.id
             HAVING SUM(p.amount) != 0
             ORDER BY t.date DESC
             LIMIT 100",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok(UnbalancedTransaction {
                    transaction_id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                    sum: row.get(3)?,
                })
            })
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
        })
        .map_err(|e| format!("Failed to query unbalanced transactions: {}", e))?;

    let balanced_count = total_transactions - unbalanced_count;

    Ok(BalanceCheckResult {
        total_transactions,
        balanced_count,
        unbalanced_count,
        unbalanced_has_more,
        unbalanced_transactions,
    })
}

fn get_category_check_result(conn: &Connection) -> Result<CategoryCheckResult, String> {
    const LIMIT: u32 = 100;

    let uncategorized_transactions: u32 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE category_id IS NULL AND deleted_at IS NULL AND description != 'Opening Balance'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to count uncategorized transactions: {}", e))?;

    let uncategorized_has_more = uncategorized_transactions > LIMIT;

    let uncategorized_list: Vec<UncategorizedTransaction> = conn
        .prepare(
            "SELECT t.id, t.date, t.description
             FROM transactions t
             WHERE t.category_id IS NULL AND t.deleted_at IS NULL AND t.description != 'Opening Balance'
             ORDER BY t.date DESC
             LIMIT 100",
        )
        .and_then(|mut stmt| {
            stmt.query_map([], |row| {
                Ok(UncategorizedTransaction {
                    transaction_id: row.get(0)?,
                    date: row.get(1)?,
                    description: row.get(2)?,
                })
            })
            .map(|iter| iter.filter_map(|r| r.ok()).collect())
        })
        .map_err(|e| format!("Failed to query uncategorized transactions: {}", e))?;

    Ok(CategoryCheckResult {
        uncategorized_transactions,
        uncategorized_has_more,
        uncategorized_list,
    })
}

fn get_account_usage_stats(conn: &Connection) -> Result<Vec<AccountUsageStat>, String> {
    conn.prepare(
        "SELECT a.id, a.name, a.type,
                    COUNT(p.id) as posting_count,
                    SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END) as total_debit,
                    SUM(CASE WHEN p.amount < 0 THEN -p.amount ELSE 0 END) as total_credit,
                    MAX(t.date) as last_transaction_date
             FROM accounts a
             JOIN postings p ON p.account_id = a.id
             JOIN transactions t ON t.id = p.transaction_id AND t.deleted_at IS NULL AND t.description != 'Opening Balance'
             GROUP BY a.id
             ORDER BY posting_count DESC",
    )
    .and_then(|mut stmt| {
        stmt.query_map([], |row| {
            Ok(AccountUsageStat {
                account_id: row.get(0)?,
                account_name: row.get(1)?,
                account_type: row.get(2)?,
                posting_count: row.get(3)?,
                total_debit: row.get(4)?,
                total_credit: row.get(5)?,
                last_transaction_date: row.get(6)?,
            })
        })
        .map(|iter| iter.filter_map(|r| r.ok()).collect())
    })
    .map_err(|e| format!("Failed to query account usage stats: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts::{create_account_with_path, CreateAccountFullInput};
    use crate::db::categories::{create_category, CategoryType};
    use crate::db::transactions::{create_transaction, PostingInput};
    use rusqlite::Connection;
    use rusqlite::params;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_resolve_date_range_current_month() {
        let filter = ReportFilter {
            date_range_preset: DateRangePreset::CurrentMonth,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let (start, end) = resolve_date_range(&filter);
        assert!(start.len() == 10);
        assert!(end.len() == 10);
    }

    #[test]
    fn test_resolve_date_range_custom() {
        let filter = ReportFilter {
            date_range_preset: DateRangePreset::Custom,
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-01-31".to_string()),
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let (start, end) = resolve_date_range(&filter);
        assert_eq!(start, "2024-01-01");
        assert_eq!(end, "2024-01-31");
    }

    #[test]
    fn test_get_standard_report_empty_db() {
        let conn = setup_test_db();
        let filter = ReportFilter {
            date_range_preset: DateRangePreset::CurrentMonth,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let result = get_standard_report(&conn, &filter);
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.period_income, 0);
        assert_eq!(dto.period_expense, 0);
    }

    #[test]
    fn test_parse_month_bounds() {
        let bounds = parse_month_bounds("2024-01");
        assert!(bounds.is_ok());
        let (start, end) = bounds.unwrap();
        assert_eq!(start, "2024-01-01");
        assert_eq!(end, "2024-01-31");
    }

    #[test]
    fn test_parse_month_bounds_invalid_month() {
        let result = parse_month_bounds("2024-13");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("1-12"));
    }

    #[test]
    fn test_get_month_comparison_empty_db() {
        let conn = setup_test_db();
        let result = get_month_comparison(&conn, "2024-01", "2024-02");
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.month1_income, 0);
        assert_eq!(dto.month1_expense, 0);
        assert_eq!(dto.category_comparison.len(), 0);
    }

    #[test]
    fn test_get_trend_report_empty_db() {
        let conn = setup_test_db();
        let filter = ReportFilter {
            date_range_preset: DateRangePreset::CurrentMonth,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let result = get_trend_report(&conn, &filter);
        assert!(result.is_ok());
        let dto = result.unwrap();
        assert_eq!(dto.granularity, "monthly");
        assert_eq!(dto.data_points.len(), 0);
    }

    #[test]
    fn test_get_category_breakdown_report_empty_db() {
        let conn = setup_test_db();
        let filter = ReportFilter {
            date_range_preset: DateRangePreset::CurrentMonth,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let result = get_category_breakdown_report(&conn, &filter, "expense");
        match result {
            Ok(dto) => {
                assert_eq!(dto.total_amount, 0);
                assert_eq!(dto.categories.len(), 0);
            }
            Err(e) => {
                panic!("get_category_breakdown_report returned error: {}", e);
            }
        }
    }

    struct SeededIds {
        cash: String,
        credit_card: String,
        food_account: String,
        food_category: String,
        transport_category: String,
    }

    fn net_worth_input() -> CreateAccountFullInput {
        CreateAccountFullInput {
            description: None,
            account_number: None,
            iban: None,
            is_active: true,
            include_net_worth: true,
        }
    }

    fn seed_data(conn: &Connection) -> SeededIds {
        let cash = create_account_with_path(conn, "Assets/Cash", "CNY", Some(&net_worth_input()))
            .unwrap();
        let credit_card =
            create_account_with_path(conn, "Liabilities/CreditCard", "CNY", Some(&net_worth_input()))
                .unwrap();
        let salary_account = create_account_with_path(conn, "Income/Salary", "CNY", None).unwrap();
        let food_account = create_account_with_path(conn, "Expenses/Food", "CNY", None).unwrap();
        let transport_account =
            create_account_with_path(conn, "Expenses/Transport", "CNY", None).unwrap();

        let salary_category = create_category(conn, "Salary", &CategoryType::Income, None).unwrap();
        let food_category = create_category(conn, "Food", &CategoryType::Expense, None).unwrap();
        let transport_category =
            create_category(conn, "Transport", &CategoryType::Expense, None).unwrap();

        // 2024-01-05 salary of 100000 cents (1000.00)
        create_transaction(
            conn,
            "2024-01-05",
            "Salary Jan",
            Some(&salary_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: 100000,
                },
                PostingInput {
                    account_id: salary_account.clone(),
                    amount: -100000,
                },
            ],
        )
        .unwrap();
        // 2024-01-10 lunch 3500
        create_transaction(
            conn,
            "2024-01-10",
            "Lunch",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -3500,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 3500,
                },
            ],
        )
        .unwrap();
        // 2024-01-20 metro 1200
        create_transaction(
            conn,
            "2024-01-20",
            "Metro",
            Some(&transport_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -1200,
                },
                PostingInput {
                    account_id: transport_account.clone(),
                    amount: 1200,
                },
            ],
        )
        .unwrap();
        // 2024-01-25 credit card purchase 8000 (liability credit posting)
        create_transaction(
            conn,
            "2024-01-25",
            "Online",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: credit_card.clone(),
                    amount: -8000,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 8000,
                },
            ],
        )
        .unwrap();
        // 2024-02-10 dinner 5000
        create_transaction(
            conn,
            "2024-02-10",
            "Dinner",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -5000,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 5000,
                },
            ],
        )
        .unwrap();
        // Unbalanced transaction (single posting on cash) for audit coverage
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, is_reconciled, deleted_at, created_at, updated_at)
             VALUES ('tx-unbalanced', '2024-01-30', 'Broken', NULL, 0, NULL, '2024-01-30T00:00:00+08:00', '2024-01-30T00:00:00+08:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p-unbalanced', 'tx-unbalanced', ?1, 100, 0, '2024-01-30T00:00:00+08:00')",
            params![cash],
        )
        .unwrap();

        SeededIds {
            cash,
            credit_card,
            food_account,
            food_category,
            transport_category,
        }
    }

    fn custom_filter(start: &str, end: &str) -> ReportFilter {
        ReportFilter {
            date_range_preset: DateRangePreset::Custom,
            start_date: Some(start.to_string()),
            end_date: Some(end.to_string()),
            period_granularity: PeriodGranularity::Daily,
            account_ids: None,
            category_ids: None,
        }
    }

    #[test]
    fn test_display_and_period_format() {
        assert_eq!(PeriodGranularity::Daily.to_string(), "daily");
        assert_eq!(PeriodGranularity::Weekly.to_string(), "weekly");
        assert_eq!(PeriodGranularity::Monthly.to_string(), "monthly");
        assert_eq!(PeriodGranularity::Yearly.to_string(), "yearly");
        assert_eq!(get_period_format(&PeriodGranularity::Daily), "%Y-%m-%d");
        assert_eq!(get_period_format(&PeriodGranularity::Weekly), "%Y-%W");
        assert_eq!(get_period_format(&PeriodGranularity::Monthly), "%Y-%m");
        assert_eq!(get_period_format(&PeriodGranularity::Yearly), "%Y");
    }

    #[test]
    fn test_resolve_date_range_all_presets() {
        let presets = [
            DateRangePreset::CurrentMonth,
            DateRangePreset::LastMonth,
            DateRangePreset::CurrentYear,
            DateRangePreset::LastYear,
            DateRangePreset::Last3Months,
            DateRangePreset::Last6Months,
            DateRangePreset::Last12Months,
            DateRangePreset::Custom,
        ];
        for preset in presets {
            let filter = ReportFilter {
                date_range_preset: preset,
                start_date: None,
                end_date: None,
                period_granularity: PeriodGranularity::Monthly,
                account_ids: None,
                category_ids: None,
            };
            let (start, end) = resolve_date_range(&filter);
            assert!(start.len() == 10 && end.len() == 10);
        }
    }

    #[test]
    fn test_get_standard_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_standard_report(&conn, &filter).unwrap();
        assert_eq!(dto.period_income, 100000);
        assert_eq!(dto.period_expense, 12700);
        assert_eq!(dto.prev_income, 0);
        assert_eq!(dto.prev_expense, 0);
        assert_eq!(dto.income_change_pct, 100.0);
        assert_eq!(dto.expense_change_pct, 100.0);
        assert_eq!(dto.net_worth_trend.len(), 5);
        assert_eq!(dto.net_worth_trend[0].net_worth, 100000);
        assert_eq!(dto.net_worth_trend[4].net_worth, 87400);
        assert_eq!(dto.account_changes.len(), 2);
        let cash_change = dto
            .account_changes
            .iter()
            .find(|c| c.account_id == seeded.cash)
            .unwrap();
        assert_eq!(cash_change.end_balance, 95400);
        let cc_change = dto
            .account_changes
            .iter()
            .find(|c| c.account_id == seeded.credit_card)
            .unwrap();
        assert_eq!(cc_change.end_balance, -8000);
    }

    #[test]
    fn test_get_standard_report_empty_period_change_pct_zero() {
        let conn = setup_test_db();
        seed_data(&conn);

        // No data in this range -> both diffs are zero -> 0.0% change
        let filter = custom_filter("2025-01-01", "2025-01-31");
        let dto = get_standard_report(&conn, &filter).unwrap();
        assert_eq!(dto.income_change_pct, 0.0);
        assert_eq!(dto.expense_change_pct, 0.0);
    }

    #[test]
    fn test_get_standard_report_account_filter() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let mut filter = custom_filter("2024-01-01", "2024-01-31");
        filter.account_ids = Some(vec![seeded.food_account.clone()]);
        let dto = get_standard_report(&conn, &filter).unwrap();
        assert_eq!(dto.period_income, 0);
        assert_eq!(dto.period_expense, 11500);
        assert_eq!(dto.account_changes.len(), 0);
    }

    #[test]
    fn test_get_standard_report_all_presets() {
        let conn = setup_test_db();
        seed_data(&conn);
        for preset in [
            DateRangePreset::CurrentMonth,
            DateRangePreset::LastMonth,
            DateRangePreset::CurrentYear,
            DateRangePreset::LastYear,
            DateRangePreset::Last3Months,
            DateRangePreset::Last6Months,
            DateRangePreset::Last12Months,
            DateRangePreset::Custom,
        ] {
            let filter = ReportFilter {
                date_range_preset: preset,
                start_date: Some("2024-01-01".to_string()),
                end_date: Some("2024-01-31".to_string()),
                period_granularity: PeriodGranularity::Monthly,
                account_ids: None,
                category_ids: None,
            };
            assert!(get_standard_report(&conn, &filter).is_ok());
        }
    }

    #[test]
    fn test_get_standard_report_current_month_change_pct() {
        let conn = setup_test_db();
        let (month_start, month_end) = current_month_bounds();
        let (prev_start, prev_end) = last_month_bounds();

        let cash = create_account_with_path(&conn, "Assets/Cash2", "CNY", Some(&net_worth_input()))
            .unwrap();
        let income = create_account_with_path(&conn, "Income/Salary2", "CNY", None).unwrap();
        let expense = create_account_with_path(&conn, "Expenses/Food2", "CNY", None).unwrap();

        // Previous month: income 50000, expense 10000
        create_transaction(
            &conn,
            &prev_start,
            "Prev Salary",
            None,
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: 50000,
                },
                PostingInput {
                    account_id: income.clone(),
                    amount: -50000,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            &prev_end,
            "Prev Food",
            None,
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -10000,
                },
                PostingInput {
                    account_id: expense.clone(),
                    amount: 10000,
                },
            ],
        )
        .unwrap();
        // Current month: income 60000, expense 12000
        create_transaction(
            &conn,
            &month_start,
            "Cur Salary",
            None,
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: 60000,
                },
                PostingInput {
                    account_id: income.clone(),
                    amount: -60000,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            &month_end,
            "Cur Food",
            None,
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -12000,
                },
                PostingInput {
                    account_id: expense.clone(),
                    amount: 12000,
                },
            ],
        )
        .unwrap();

        let filter = ReportFilter {
            date_range_preset: DateRangePreset::CurrentMonth,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let dto = get_standard_report(&conn, &filter).unwrap();
        assert_eq!(dto.period_income, 60000);
        assert_eq!(dto.period_expense, 12000);
        assert_eq!(dto.prev_income, 50000);
        assert_eq!(dto.prev_expense, 10000);
        assert!((dto.income_change_pct - 20.0).abs() < 0.001);
        assert!((dto.expense_change_pct - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_get_month_comparison_with_data() {
        let conn = setup_test_db();
        seed_data(&conn);

        let dto = get_month_comparison(&conn, "2024-01", "2024-02").unwrap();
        assert_eq!(dto.month1_income, 100000);
        assert_eq!(dto.month1_expense, 12700);
        assert_eq!(dto.month2_income, 0);
        assert_eq!(dto.month2_expense, 5000);
        assert_eq!(dto.income_diff, -100000);
        assert_eq!(dto.expense_diff, -7700);
        assert!((dto.income_change_pct - -100.0).abs() < 0.001);
        assert!((dto.expense_change_pct - (-7700.0 / 12700.0 * 100.0)).abs() < 0.001);
        assert_eq!(dto.category_comparison.len(), 2);
        let food = dto
            .category_comparison
            .iter()
            .find(|c| c.category_name == "Food")
            .unwrap();
        assert_eq!(food.month1_amount, 11500);
        assert_eq!(food.month2_amount, 5000);
        let transport = dto
            .category_comparison
            .iter()
            .find(|c| c.category_name == "Transport")
            .unwrap();
        assert_eq!(transport.change_pct, -100.0);
    }

    #[test]
    fn test_get_month_comparison_new_data_branches() {
        let conn = setup_test_db();
        seed_data(&conn);

        // month1 has no data, month2 introduces income/expense -> 100.0%
        let dto = get_month_comparison(&conn, "2024-03", "2024-01").unwrap();
        assert_eq!(dto.income_change_pct, 100.0);
        assert_eq!(dto.expense_change_pct, 100.0);

        // both months empty -> 0.0%
        let dto = get_month_comparison(&conn, "2024-03", "2024-04").unwrap();
        assert_eq!(dto.income_change_pct, 0.0);
        assert_eq!(dto.expense_change_pct, 0.0);
    }

    #[test]
    fn test_get_month_comparison_invalid_month() {
        let conn = setup_test_db();
        assert!(get_month_comparison(&conn, "2024", "2024-02").is_err());
        assert!(get_month_comparison(&conn, "abcd-01", "2024-02").is_err());
    }

    #[test]
    fn test_get_trend_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_trend_report(&conn, &filter).unwrap();
        assert_eq!(dto.granularity, "daily");
        assert_eq!(dto.data_points.len(), 5);
        assert_eq!(dto.total_income, 100000);
        assert_eq!(dto.total_expense, 12700);
        assert_eq!(dto.total_net, 87300);

        // Category filter + monthly granularity
        let mut filtered = custom_filter("2024-01-01", "2024-01-31");
        filtered.period_granularity = PeriodGranularity::Monthly;
        filtered.category_ids = Some(vec![seeded.food_category.clone()]);
        let dto = get_trend_report(&conn, &filtered).unwrap();
        assert_eq!(dto.data_points.len(), 1);
        assert_eq!(dto.data_points[0].period, "2024-01");
        assert_eq!(dto.total_income, 0);
        assert_eq!(dto.total_expense, 11500);

        // Weekly and yearly granularities
        let mut weekly = custom_filter("2024-01-01", "2024-01-31");
        weekly.period_granularity = PeriodGranularity::Weekly;
        assert!(get_trend_report(&conn, &weekly).is_ok());

        let mut yearly = custom_filter("2024-01-01", "2024-01-31");
        yearly.period_granularity = PeriodGranularity::Yearly;
        let dto = get_trend_report(&conn, &yearly).unwrap();
        assert_eq!(dto.data_points.len(), 1);
        assert_eq!(dto.data_points[0].period, "2024");
    }

    #[test]
    fn test_get_category_breakdown_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_category_breakdown_report(&conn, &filter, "expense").unwrap();
        assert_eq!(dto.total_amount, 12700);
        assert_eq!(dto.categories.len(), 2);
        let food = dto
            .categories
            .iter()
            .find(|c| c.category_name == "Food")
            .unwrap();
        assert_eq!(food.amount, 11500);
        assert!((food.percentage - 11500.0 / 12700.0 * 100.0).abs() < 0.001);
        assert_eq!(food.transaction_count, 2);

        let income_dto = get_category_breakdown_report(&conn, &filter, "income").unwrap();
        assert_eq!(income_dto.total_amount, 100000);
        assert_eq!(income_dto.categories.len(), 1);
        assert_eq!(income_dto.categories[0].category_name, "Salary");
        assert_eq!(income_dto.categories[0].percentage, 100.0);

        // Category filter
        let mut filtered = custom_filter("2024-01-01", "2024-01-31");
        filtered.category_ids = Some(vec![seeded.transport_category.clone()]);
        let dto = get_category_breakdown_report(&conn, &filtered, "expense").unwrap();
        assert_eq!(dto.total_amount, 1200);
        assert_eq!(dto.categories.len(), 1);
    }

    #[test]
    fn test_get_balance_sheet_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let dto = get_balance_sheet_report(&conn, "2024-01-31").unwrap();
        assert_eq!(dto.snapshot_date, "2024-01-31");
        assert_eq!(dto.assets.len(), 1);
        assert_eq!(dto.assets[0].account_id, seeded.cash);
        assert_eq!(dto.assets[0].balance, 95400);
        assert_eq!(dto.liabilities.len(), 1);
        assert_eq!(dto.liabilities[0].account_id, seeded.credit_card);
        assert_eq!(dto.liabilities[0].balance, -8000);
        assert_eq!(dto.total_assets, 95400);
        assert_eq!(dto.total_liabilities, -8000);
        assert_eq!(dto.net_worth, 87400);
    }

    #[test]
    fn test_get_year_summary_report_with_data() {
        let conn = setup_test_db();
        seed_data(&conn);

        let dto = get_year_summary_report(&conn, 2024).unwrap();
        assert_eq!(dto.year, 2024);
        assert_eq!(dto.total_income, 100000);
        assert_eq!(dto.total_expense, 17700);
        assert_eq!(dto.net, 82300);
        assert_eq!(dto.monthly_breakdown.len(), 12);
        assert_eq!(dto.monthly_breakdown[0].month, 1);
        assert_eq!(dto.monthly_breakdown[0].income, 100000);
        assert_eq!(dto.monthly_breakdown[0].expense, 12700);
        assert_eq!(dto.monthly_breakdown[1].expense, 5000);
        assert_eq!(dto.top_income_categories.len(), 1);
        assert_eq!(dto.top_income_categories[0].category_name, "Salary");
        assert_eq!(dto.top_income_categories[0].percentage, 100.0);
        assert_eq!(dto.top_expense_categories.len(), 2);
        assert_eq!(dto.top_expense_categories[0].category_name, "Food");
        assert_eq!(dto.top_expense_categories[0].amount, 16500);
        assert_eq!(dto.top_expense_categories[1].category_name, "Transport");
    }

    #[test]
    fn test_get_account_transactions_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 10).unwrap();
        assert_eq!(dto.account_id, seeded.cash);
        assert_eq!(dto.account_name, "Cash");
        assert_eq!(dto.total_count, 4);
        assert_eq!(dto.total_pages, 1);
        assert_eq!(dto.transactions.len(), 4);
        assert_eq!(dto.total_inflow, 100100);
        assert_eq!(dto.total_outflow, 4700);
        assert_eq!(dto.net_change, 95400);

        let salary = dto
            .transactions
            .iter()
            .find(|t| t.description == "Salary Jan")
            .unwrap();
        assert_eq!(salary.amount, 100000);
        assert!(salary.is_inflow);
        assert_eq!(salary.counter_account_name.as_deref(), Some("Salary"));
        assert_eq!(salary.category_name.as_deref(), Some("Salary"));
    }

    #[test]
    fn test_get_account_transactions_report_pagination() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 2).unwrap();
        assert_eq!(dto.total_pages, 2);
        assert_eq!(dto.transactions.len(), 2);

        // page_size 0 -> division by zero handled with fallback page count
        let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 0).unwrap();
        assert_eq!(dto.total_pages, 1);
        assert_eq!(dto.transactions.len(), 0);
    }

    #[test]
    fn test_get_account_balance_trend_report_with_data() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let filter = custom_filter("2024-01-01", "2024-01-31");
        let dto = get_account_balance_trend_report(&conn, &seeded.cash, &filter).unwrap();
        assert_eq!(dto.account_name, "Cash");
        assert_eq!(dto.granularity, "daily");
        assert_eq!(dto.data_points.len(), 4);
        assert_eq!(dto.data_points[0].balance, 100000);
        assert_eq!(dto.data_points[3].balance, 95400);

        let mut monthly = custom_filter("2024-01-01", "2024-01-31");
        monthly.period_granularity = PeriodGranularity::Monthly;
        let dto = get_account_balance_trend_report(&conn, &seeded.cash, &monthly).unwrap();
        assert_eq!(dto.data_points.len(), 1);
        assert_eq!(dto.data_points[0].period, "2024-01");
        assert_eq!(dto.data_points[0].balance, 95400);

        // Unknown account -> error
        assert!(get_account_balance_trend_report(&conn, "missing", &filter).is_err());
    }

    #[test]
    fn test_get_audit_report_with_data() {
        let conn = setup_test_db();
        seed_data(&conn);

        let dto = get_audit_report(&conn).unwrap();
        assert_eq!(dto.balance_check.total_transactions, 6);
        assert_eq!(dto.balance_check.balanced_count, 5);
        assert_eq!(dto.balance_check.unbalanced_count, 1);
        assert!(!dto.balance_check.unbalanced_has_more);
        assert_eq!(dto.balance_check.unbalanced_transactions.len(), 1);
        assert_eq!(
            dto.balance_check.unbalanced_transactions[0].transaction_id,
            "tx-unbalanced"
        );
        assert_eq!(dto.balance_check.unbalanced_transactions[0].sum, 100);

        assert_eq!(dto.category_check.uncategorized_transactions, 1);
        assert_eq!(dto.category_check.uncategorized_list.len(), 1);
        assert!(!dto.category_check.uncategorized_has_more);

        assert_eq!(dto.account_usage.len(), 5);
        let cash_stat = dto
            .account_usage
            .iter()
            .find(|s| s.account_name == "Cash")
            .unwrap();
        assert_eq!(cash_stat.posting_count, 5);
        assert_eq!(cash_stat.total_debit, 100100);
        assert_eq!(cash_stat.total_credit, 9700);
    }

    #[test]
    fn test_get_audit_report_more_than_limit() {
        let conn = setup_test_db();
        let seeded = seed_data(&conn);

        let mut tx_stmt = conn
            .prepare(
                "INSERT INTO transactions (id, date, description, category_id, is_reconciled, deleted_at, created_at, updated_at)
                 VALUES (?1, '2024-03-01', 'Broken', NULL, 0, NULL, '2024-03-01T00:00:00+08:00', '2024-03-01T00:00:00+08:00')",
            )
            .unwrap();
        let mut posting_stmt = conn
            .prepare(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, 100, 0, '2024-03-01T00:00:00+08:00')",
            )
            .unwrap();
        for i in 0..105 {
            let tx_id = format!("tx-unbalanced-{}", i);
            tx_stmt.execute(params![&tx_id]).unwrap();
            posting_stmt
                .execute(params![format!("p-{}", i), &tx_id, &seeded.cash])
                .unwrap();
        }

        let dto = get_audit_report(&conn).unwrap();
        assert!(dto.balance_check.unbalanced_has_more);
        assert_eq!(dto.balance_check.unbalanced_transactions.len(), 100);
        assert_eq!(dto.balance_check.unbalanced_count, 106);
        assert!(dto.category_check.uncategorized_has_more);
        assert_eq!(dto.category_check.uncategorized_list.len(), 100);
        assert_eq!(dto.category_check.uncategorized_transactions, 106);
    }
}
