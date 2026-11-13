//! Date-range resolution and the shared income/expense aggregates.

use rusqlite::{params, Connection};

use crate::constants::account_type;
use crate::utils::time::*;

use super::types::{DateRangePreset, PeriodGranularity, ReportFilter};

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

/// Calculates total income for the given period.
// Income accounts receive credit entries (negative amounts) per standard double-entry convention
pub(in crate::db::reports) fn get_income_sum(
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
pub(in crate::db::reports) fn get_expense_sum(
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
pub(in crate::db::reports) fn get_prev_period_bounds(preset: &DateRangePreset) -> (String, String) {
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

pub(in crate::db::reports) fn parse_month_bounds(month: &str) -> Result<(String, String), String> {
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
