use chrono::{Datelike, NaiveDate};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::constants::account_type;
use crate::db::categories::CategoryType;
use crate::db::AppError;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub month_income: i64,
    pub month_expense: i64,
    pub month_start: String,
    pub month_end: String,
    pub year_income: i64,
    pub year_expense: i64,
    pub year_start: String,
    pub year_end: String,
    pub total_assets: i64,
    pub total_liabilities: i64,
    pub net_worth: i64,
    pub calculated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthlyChartData {
    pub year: i32,
    pub month: i32,
    pub days: Vec<DailyIncomeExpense>,
    pub month_total_income: i64,
    pub month_total_expense: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyIncomeExpense {
    pub date: String,
    pub income: i64,
    pub expense: i64,
    pub has_transactions: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryBreakdownData {
    pub year: i32,
    pub month: i32,
    pub category_type: CategoryType,
    pub categories: Vec<CategoryAmount>,
    pub total_amount: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryAmount {
    pub category_id: String,
    pub category_name: String,
    pub icon: Option<String>,
    pub amount: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopExpensesData {
    pub year: i32,
    pub month: i32,
    pub expenses: Vec<TopExpenseItem>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopExpenseItem {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub amount: i64,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
}

// ---------------------------------------------------------------------------
// Query functions
// ---------------------------------------------------------------------------

/// Get dashboard summary data for a date range.
pub fn get_dashboard_summary(
    conn: &Connection,
    month_start: &str,
    month_end: &str,
    year_start: &str,
    year_end: &str,
) -> Result<DashboardSummary, AppError> {
    let calculated_at = crate::utils::time::now_rfc3339();

    // Calculate month income (from income accounts with negative postings, negate for display)
    // Income accounts receive credit entries (negative amounts) per standard double-entry convention
    let month_income: i64 = conn.query_row(
        &format!(
            "SELECT COALESCE(-SUM(p.amount), 0)
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             JOIN transactions t ON t.id = p.transaction_id
             WHERE a.type = '{}' AND p.amount < 0
             AND t.date >= ?1 AND t.date <= ?2",
            account_type::INCOME
        ),
        rusqlite::params![month_start, month_end],
        |row| row.get(0),
    )?;

    // Calculate month expense (from expense accounts with positive postings)
    // Expense accounts receive debit entries (positive amounts) per standard double-entry convention
    let month_expense: i64 = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(p.amount), 0)
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             JOIN transactions t ON t.id = p.transaction_id
             WHERE a.type = '{}' AND p.amount > 0
             AND t.date >= ?1 AND t.date <= ?2",
            account_type::EXPENSE
        ),
        rusqlite::params![month_start, month_end],
        |row| row.get(0),
    )?;

    // Calculate year income
    let year_income: i64 = conn.query_row(
        &format!(
            "SELECT COALESCE(-SUM(p.amount), 0)
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             JOIN transactions t ON t.id = p.transaction_id
             WHERE a.type = '{}' AND p.amount < 0
             AND t.date >= ?1 AND t.date <= ?2",
            account_type::INCOME
        ),
        rusqlite::params![year_start, year_end],
        |row| row.get(0),
    )?;

    // Calculate year expense
    let year_expense: i64 = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(p.amount), 0)
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             JOIN transactions t ON t.id = p.transaction_id
             WHERE a.type = '{}' AND p.amount > 0
             AND t.date >= ?1 AND t.date <= ?2",
            account_type::EXPENSE
        ),
        rusqlite::params![year_start, year_end],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(&format!(
        "SELECT a.id, a.type, COALESCE(SUM(p.amount), 0) as balance
             FROM accounts a
             LEFT JOIN postings p ON p.account_id = a.id
             WHERE a.type IN ('{}', '{}') AND a.is_active = 1
             GROUP BY a.id, a.type",
        account_type::ASSET,
        account_type::LIABILITY
    ))?;

    let account_balances: Vec<(String, String, i64)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut total_assets: i64 = 0;
    let mut total_liabilities: i64 = 0;

    for (_account_id, account_type_str, balance) in account_balances {
        if account_type_str == account_type::ASSET {
            total_assets += balance;
        } else if account_type_str == account_type::LIABILITY {
            total_liabilities += balance;
        }
    }

    let net_worth = total_assets + total_liabilities;

    Ok(DashboardSummary {
        month_income,
        month_expense,
        month_start: month_start.to_string(),
        month_end: month_end.to_string(),
        year_income,
        year_expense,
        year_start: year_start.to_string(),
        year_end: year_end.to_string(),
        total_assets,
        total_liabilities,
        net_worth,
        calculated_at,
    })
}

pub fn get_monthly_chart(
    conn: &Connection,
    year: i32,
    month: i32,
) -> Result<MonthlyChartData, AppError> {
    let month_start = NaiveDate::from_ymd_opt(year, month as u32, 1).ok_or_else(|| {
        AppError::ValidationError(format!("Invalid year/month: {}-{}", year, month))
    })?;

    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    } else {
        NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    };

    let days_in_month = month_end.day();
    let start_str = month_start.format("%Y-%m-%d").to_string();
    let end_str = month_end.format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(
        &format!(
            "SELECT 
                t.date,
                SUM(CASE WHEN a.type = '{}' AND p.amount < 0 THEN -p.amount ELSE 0 END) as income,
                SUM(CASE WHEN a.type = '{}' AND p.amount > 0 THEN p.amount ELSE 0 END) as expense,
                COUNT(DISTINCT CASE WHEN (a.type = '{}' OR a.type = '{}') AND p.amount != 0 THEN t.id END) as tx_count
            FROM transactions t
            JOIN postings p ON p.transaction_id = t.id
            JOIN accounts a ON a.id = p.account_id
            WHERE t.date >= ?1 AND t.date <= ?2
            GROUP BY t.date
            ORDER BY t.date",
            account_type::INCOME,
            account_type::EXPENSE,
            account_type::INCOME,
            account_type::EXPENSE
        )
    )?;

    let daily_data: Vec<(String, i64, i64, i64)> = stmt
        .query_map(rusqlite::params![&start_str, &end_str], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let data_map: std::collections::HashMap<String, (i64, i64, bool)> = daily_data
        .into_iter()
        .map(|(date, income, expense, tx_count)| (date, (income, expense, tx_count > 0)))
        .collect();

    let mut days: Vec<DailyIncomeExpense> = Vec::new();
    let mut month_total_income: i64 = 0;
    let mut month_total_expense: i64 = 0;

    for day in 1..=days_in_month {
        let date = NaiveDate::from_ymd_opt(year, month as u32, day)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?;
        let date_str = date.format("%Y-%m-%d").to_string();

        let (income, expense, has_transactions) =
            data_map.get(&date_str).copied().unwrap_or((0, 0, false));

        month_total_income += income;
        month_total_expense += expense;

        days.push(DailyIncomeExpense {
            date: date_str,
            income,
            expense,
            has_transactions,
        });
    }

    Ok(MonthlyChartData {
        year,
        month,
        days,
        month_total_income,
        month_total_expense,
    })
}

/// Get category breakdown for income or expense.
pub fn get_category_breakdown(
    conn: &Connection,
    year: i32,
    month: i32,
    category_type: CategoryType,
) -> Result<CategoryBreakdownData, AppError> {
    // Convert CategoryType to string for SQL
    let category_type_str = match category_type {
        CategoryType::Income => "income",
        CategoryType::Expense => "expense",
    };

    // Calculate month boundaries
    let month_start = NaiveDate::from_ymd_opt(year, month as u32, 1).ok_or_else(|| {
        AppError::ValidationError(format!("Invalid year/month: {}-{}", year, month))
    })?;

    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    } else {
        NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    };

    // Income accounts have negative amounts, expense accounts have positive amounts
    let (amount_condition, amount_expr) = match category_type {
        CategoryType::Income => ("p.amount < 0", "-COALESCE(SUM(p.amount), 0)"),
        CategoryType::Expense => ("p.amount > 0", "COALESCE(SUM(p.amount), 0)"),
    };

    let sql = format!(
        "
        SELECT c.id, c.name, c.icon, {} as amount
         FROM categories c
         LEFT JOIN transactions t ON t.category_id = c.id
         LEFT JOIN postings p ON p.transaction_id = t.id
         LEFT JOIN accounts a ON a.id = p.account_id
         WHERE c.type = ?1
         AND (t.date IS NULL OR (t.date >= ?2 AND t.date <= ?3))
         AND (p.amount IS NULL OR {})
         AND (a.type IS NULL OR a.type = ?1)
         GROUP BY c.id
         ORDER BY amount DESC",
        amount_expr, amount_condition
    );

    // Get category breakdown
    let mut stmt = conn.prepare(&sql)?;
    let start_str = month_start.format("%Y-%m-%d").to_string();
    let end_str = month_end.format("%Y-%m-%d").to_string();

    let categories_raw: Vec<(String, String, Option<String>, i64)> = stmt
        .query_map(
            rusqlite::params![category_type_str, &start_str, &end_str],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?
        .collect::<Result<Vec<_>, _>>()?;

    let total_amount: i64 = categories_raw.iter().map(|(_, _, _, amt)| amt).sum();

    let categories: Vec<CategoryAmount> = categories_raw
        .into_iter()
        .map(|(category_id, category_name, icon, amount)| {
            let percentage = if total_amount > 0 {
                (amount as f64 / total_amount as f64) * 100.0
            } else {
                0.0
            };
            CategoryAmount {
                category_id,
                category_name,
                icon,
                amount,
                percentage,
            }
        })
        .collect();

    Ok(CategoryBreakdownData {
        year,
        month,
        category_type,
        categories,
        total_amount,
    })
}

/// Get top expense transactions for a month.
pub fn get_top_expenses(
    conn: &Connection,
    year: i32,
    month: i32,
    limit: i32,
) -> Result<TopExpensesData, AppError> {
    // Calculate month boundaries
    let month_start = NaiveDate::from_ymd_opt(year, month as u32, 1).ok_or_else(|| {
        AppError::ValidationError(format!("Invalid year/month: {}-{}", year, month))
    })?;

    let month_end = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    } else {
        NaiveDate::from_ymd_opt(year, (month + 1) as u32, 1)
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
            .pred_opt()
            .ok_or_else(|| AppError::ValidationError("Invalid date".into()))?
    };

    let start_str = month_start.format("%Y-%m-%d").to_string();
    let end_str = month_end.format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(&format!(
        "SELECT t.id, t.date, t.description, p.amount,
                    c.id as category_id, c.name as category_name, c.icon as category_icon
             FROM transactions t
             JOIN postings p ON p.transaction_id = t.id
             JOIN accounts a ON a.id = p.account_id
             LEFT JOIN categories c ON c.id = t.category_id
             WHERE a.type = '{}' AND p.amount > 0
             AND t.date >= ?1 AND t.date <= ?2
             ORDER BY p.amount DESC
             LIMIT ?3",
        account_type::EXPENSE
    ))?;

    let expenses: Vec<TopExpenseItem> = stmt
        .query_map(rusqlite::params![&start_str, &end_str, limit], |row| {
            Ok(TopExpenseItem {
                transaction_id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                amount: row.get(3)?,
                category_id: row.get(4)?,
                category_name: row.get(5)?,
                category_icon: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TopExpensesData {
        year,
        month,
        expenses,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::accounts::{insert_account, AccountRecord};
    use crate::db::categories::{create_category, CategoryType};
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
                updated_at TEXT NOT NULL
            );

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT REFERENCES categories(id),
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE postings (
                id             TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                account_id     TEXT NOT NULL REFERENCES accounts(id),
                amount         INTEGER NOT NULL,
                created_at     TEXT NOT NULL
            );

            CREATE UNIQUE INDEX idx_accounts_type_name ON accounts(type, name);
            CREATE UNIQUE INDEX idx_categories_type_name ON categories(type, name);
        "#,
        )
        .unwrap();
        (dir, conn)
    }

    fn make_account(name: &str, account_type: &str, include_net_worth: bool) -> AccountRecord {
        AccountRecord {
            id: String::new(),
            name: name.to_string(),
            account_type: account_type.to_string(),
            currency: "CNY".to_string(),
            description: String::new(),
            account_number: None,
            is_system: false,
            iban: None,
            is_active: true,
            include_net_worth,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn test_dashboard_summary_empty() {
        let (_dir, conn) = test_conn();

        // Create some accounts but no transactions
        insert_account(&conn, &make_account("银行", "asset", true)).unwrap();
        insert_account(&conn, &make_account("信用卡", "liability", true)).unwrap();
        insert_account(&conn, &make_account("工资", "income", false)).unwrap();
        insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        let summary = get_dashboard_summary(
            &conn,
            "2024-01-01",
            "2024-01-31",
            "2024-01-01",
            "2024-12-31",
        )
        .unwrap();

        assert_eq!(summary.month_income, 0);
        assert_eq!(summary.month_expense, 0);
        assert_eq!(summary.year_income, 0);
        assert_eq!(summary.year_expense, 0);
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_liabilities, 0);
        assert_eq!(summary.net_worth, 0);
    }

    #[test]
    fn test_monthly_chart_empty() {
        let (_dir, conn) = test_conn();

        insert_account(&conn, &make_account("工资", "income", false)).unwrap();
        insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        let chart = get_monthly_chart(&conn, 2024, 1).unwrap();

        assert_eq!(chart.year, 2024);
        assert_eq!(chart.month, 1);
        assert_eq!(chart.days.len(), 31); // January has 31 days
        assert_eq!(chart.month_total_income, 0);
        assert_eq!(chart.month_total_expense, 0);

        for day in &chart.days {
            assert_eq!(day.income, 0);
            assert_eq!(day.expense, 0);
            assert!(!day.has_transactions);
        }
    }

    #[test]
    fn test_category_breakdown_empty() {
        let (_dir, conn) = test_conn();

        create_category(&conn, "餐饮", &CategoryType::Expense, None).unwrap();
        create_category(&conn, "交通", &CategoryType::Expense, None).unwrap();

        insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        let breakdown = get_category_breakdown(&conn, 2024, 1, CategoryType::Expense).unwrap();

        assert_eq!(breakdown.year, 2024);
        assert_eq!(breakdown.month, 1);
        assert_eq!(breakdown.category_type, CategoryType::Expense);
        assert_eq!(breakdown.total_amount, 0);
        assert_eq!(breakdown.categories.len(), 2);
    }

    #[test]
    fn test_category_breakdown_income_type() {
        let (_dir, conn) = test_conn();

        create_category(&conn, "工资", &CategoryType::Income, None).unwrap();
        insert_account(&conn, &make_account("工资", "income", false)).unwrap();

        let breakdown = get_category_breakdown(&conn, 2024, 1, CategoryType::Income).unwrap();

        assert_eq!(breakdown.category_type, CategoryType::Income);
    }

    #[test]
    fn test_top_expenses_empty() {
        let (_dir, conn) = test_conn();

        insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        let expenses = get_top_expenses(&conn, 2024, 1, 10).unwrap();

        assert_eq!(expenses.year, 2024);
        assert_eq!(expenses.month, 1);
        assert!(expenses.expenses.is_empty());
    }

    #[test]
    fn test_invalid_month() {
        let (_dir, conn) = test_conn();

        let result = get_monthly_chart(&conn, 2024, 13);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_year_month() {
        let (_dir, conn) = test_conn();

        let result = get_dashboard_summary(&conn, "invalid", "date", "2024-01-01", "2024-12-31");
        // Should succeed because SQL won't match any rows with invalid dates
        assert!(result.is_ok());
    }

    #[test]
    fn test_monthly_chart_single_query() {
        let (_dir, conn) = test_conn();

        insert_account(&conn, &make_account("工资", "income", false)).unwrap();
        insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        let chart = get_monthly_chart(&conn, 2024, 1).unwrap();

        assert_eq!(chart.days.len(), 31);
        assert_eq!(chart.year, 2024);
        assert_eq!(chart.month, 1);
    }

    #[test]
    fn test_dashboard_summary_batch_balance() {
        let (_dir, conn) = test_conn();

        insert_account(&conn, &make_account("银行", "asset", true)).unwrap();
        insert_account(&conn, &make_account("信用卡", "liability", true)).unwrap();

        let summary = get_dashboard_summary(
            &conn,
            "2024-01-01",
            "2024-01-31",
            "2024-01-01",
            "2024-12-31",
        )
        .unwrap();

        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_liabilities, 0);
        assert_eq!(summary.net_worth, 0);
    }

    fn insert_transaction_with_postings(
        conn: &Connection,
        tx_id: &str,
        date: &str,
        description: &str,
        postings: &[(&str, i64)],
    ) {
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
            rusqlite::params![tx_id, date, description, now],
        )
        .unwrap();

        for (idx, (account_id, amount)) in postings.iter().enumerate() {
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    format!("{}-p{}", tx_id, idx),
                    tx_id,
                    account_id,
                    amount,
                    now
                ],
            )
            .unwrap();
        }
    }

    #[test]
    fn test_dashboard_summary_with_transactions() {
        let (_dir, conn) = test_conn();

        let asset_id = insert_account(&conn, &make_account("银行", "asset", true)).unwrap();
        let income_id = insert_account(&conn, &make_account("工资", "income", false)).unwrap();
        let expense_id = insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        insert_transaction_with_postings(
            &conn,
            "tx-1",
            "2024-01-15",
            "工资收入",
            // Income account: -10000 (credit), Asset account: +10000 (debit)
            &[(income_id.as_str(), -10000), (asset_id.as_str(), 10000)],
        );

        insert_transaction_with_postings(
            &conn,
            "tx-2",
            "2024-01-20",
            "午餐",
            // Expense account: +5000 (debit), Asset account: -5000 (credit)
            &[(expense_id.as_str(), 5000), (asset_id.as_str(), -5000)],
        );

        let summary = get_dashboard_summary(
            &conn,
            "2024-01-01",
            "2024-01-31",
            "2024-01-01",
            "2024-12-31",
        )
        .unwrap();

        assert_eq!(summary.month_income, 10000);
        assert_eq!(summary.month_expense, 5000);
        assert_eq!(summary.year_income, 10000);
        assert_eq!(summary.year_expense, 5000);
    }

    #[test]
    fn test_month_boundary_last_day() {
        let (_dir, conn) = test_conn();

        let asset_id = insert_account(&conn, &make_account("银行", "asset", true)).unwrap();
        let expense_id = insert_account(&conn, &make_account("餐饮", "expense", false)).unwrap();

        insert_transaction_with_postings(
            &conn,
            "tx-jan31",
            "2024-01-31",
            "月末支出",
            // Expense account: +3000 (debit), Asset account: -3000 (credit)
            &[(expense_id.as_str(), 3000), (asset_id.as_str(), -3000)],
        );

        let chart = get_monthly_chart(&conn, 2024, 1).unwrap();
        assert_eq!(chart.days.len(), 31);
        assert!(chart.days[30].has_transactions);
        assert_eq!(chart.days[30].expense, 3000);
        assert_eq!(chart.month_total_expense, 3000);

        let chart_feb = get_monthly_chart(&conn, 2024, 2).unwrap();
        assert!(!chart_feb.days[28].has_transactions);
        assert_eq!(chart_feb.month_total_expense, 0);
    }

    #[test]
    fn test_net_worth_calculation() {
        let (_dir, conn) = test_conn();

        let asset_id = insert_account(&conn, &make_account("银行", "asset", true)).unwrap();
        let liability_id =
            insert_account(&conn, &make_account("信用卡", "liability", true)).unwrap();
        let income_id = insert_account(&conn, &make_account("工资", "income", false)).unwrap();
        let expense_id = insert_account(&conn, &make_account("消费", "expense", false)).unwrap();

        insert_transaction_with_postings(
            &conn,
            "tx-1",
            "2024-01-10",
            "存入银行",
            // Asset account: +10000 (debit), Income account: -10000 (credit)
            &[(asset_id.as_str(), 10000), (income_id.as_str(), -10000)],
        );

        insert_transaction_with_postings(
            &conn,
            "tx-2",
            "2024-01-15",
            "信用卡消费",
            &[(expense_id.as_str(), 5000), (liability_id.as_str(), -5000)],
        );

        let summary = get_dashboard_summary(
            &conn,
            "2024-01-01",
            "2024-01-31",
            "2024-01-01",
            "2024-12-31",
        )
        .unwrap();

        assert_eq!(summary.total_assets, 10000);
        assert_eq!(summary.total_liabilities, -5000);
        assert_eq!(summary.net_worth, 5000);
    }
}
