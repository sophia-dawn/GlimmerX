//! Month-over-month comparison report.

use rusqlite::{params, Connection};

use crate::constants::account_type;

use super::common::{get_expense_sum, get_income_sum, parse_month_bounds};
use super::types::{CategoryComparisonItem, MonthComparisonReportDto};

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
        INNER JOIN transactions t ON t.category_id = c.id
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
