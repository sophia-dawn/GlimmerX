use chrono::Datelike;
use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::db::categories::{find_category, CategoryType};
use crate::db::AppError;

// ---------------------------------------------------------------------------
// Type definitions
// ---------------------------------------------------------------------------

/// Budget period enum for budget cycle type.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BudgetPeriod {
    #[default]
    Monthly,
    Weekly,
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Internal DB row representation for a budget.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetRecord {
    pub id: String,
    pub category_id: String,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Budget status with spending information for display.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetStatusRecord {
    pub id: String,
    pub category_id: String,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub amount: i64,
    pub period: BudgetPeriod,
    pub rollover: bool,
    /// Total spent in the current period (sum of expense postings)
    pub spent: i64,
    /// Remaining budget (amount - spent + rollover_amount)
    pub remaining: i64,
    /// True if spent > (amount + rollover_amount)
    pub over_budget: bool,
    /// Previous period's unused budget carried over (if rollover enabled)
    pub rollover_amount: i64,
    /// Budget available for spending (amount + rollover_amount)
    pub available: i64,
}

// ---------------------------------------------------------------------------
// Constants and helpers
// ---------------------------------------------------------------------------

/// Common columns for SELECT queries on budgets table.
const BUDGET_COLUMNS: &str = "id, category_id, amount, period, rollover, created_at, updated_at";

/// Read a BudgetRecord from a row.
fn row_to_budget(row: &rusqlite::Row<'_>) -> rusqlite::Result<BudgetRecord> {
    let period_str: String = row.get(3)?;
    let period = match period_str.as_str() {
        "monthly" => BudgetPeriod::Monthly,
        "weekly" => BudgetPeriod::Weekly,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(BudgetRecord {
        id: row.get(0)?,
        category_id: row.get(1)?,
        amount: row.get(2)?,
        period,
        rollover: row.get::<_, i32>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// List all budgets, sorted by creation date descending.
pub fn list_budgets(conn: &Connection) -> Result<Vec<BudgetRecord>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {} FROM budgets ORDER BY created_at DESC",
        BUDGET_COLUMNS
    ))?;
    let budgets = stmt
        .query_map([], row_to_budget)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(budgets)
}

/// Find a single budget by ID.
pub fn find_budget(conn: &Connection, id: &str) -> Result<Option<BudgetRecord>, AppError> {
    let row = conn
        .query_row(
            &format!("SELECT {} FROM budgets WHERE id = ?1", BUDGET_COLUMNS),
            [id],
            row_to_budget,
        )
        .optional()?;
    Ok(row)
}

/// Find a budget by category ID (for uniqueness check).
pub fn find_by_category(
    conn: &Connection,
    category_id: &str,
) -> Result<Option<BudgetRecord>, AppError> {
    let row = conn
        .query_row(
            &format!(
                "SELECT {} FROM budgets WHERE category_id = ?1",
                BUDGET_COLUMNS
            ),
            [category_id],
            row_to_budget,
        )
        .optional()?;
    Ok(row)
}

// ---------------------------------------------------------------------------
// Budget status calculation
// ---------------------------------------------------------------------------

/// Inclusive date window a budget's spending is measured over.
#[derive(Debug, Clone)]
struct Window {
    from: String,
    to: String,
}

fn invalid_month() -> AppError {
    AppError::ValidationError("errors.budgetInvalidMonth".to_string())
}

fn first_of_month(year: i32, month: i32) -> Result<chrono::NaiveDate, AppError> {
    chrono::NaiveDate::from_ymd_opt(year, month as u32, 1).ok_or_else(invalid_month)
}

fn last_of_month(first: chrono::NaiveDate) -> chrono::NaiveDate {
    let (year, month) = (first.year(), first.month());
    let next = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    next.expect("a first-of-month always has a successor") - chrono::Duration::days(1)
}

/// The window a budget of `period` is measured over for the requested month.
///
/// A weekly budget is a week, not a month: measuring it over 30 days made a
/// ¥500/week budget report 360% overspend in a normal month. The week is the
/// Monday-to-Sunday one containing "today" when the caller is looking at the
/// current month, and the last week of that month otherwise — a month view of a
/// past month has no "current week" to show, and the last one is at least a real
/// week rather than a silently different unit.
fn budget_window(period: &BudgetPeriod, year: i32, month: i32) -> Result<Window, AppError> {
    let first = first_of_month(year, month)?;
    let last = last_of_month(first);
    match period {
        BudgetPeriod::Monthly => Ok(Window {
            from: first.to_string(),
            to: last.to_string(),
        }),
        BudgetPeriod::Weekly => {
            let today = crate::utils::time::today_date();
            let anchor = if today.starts_with(&format!("{year:04}-{month:02}")) {
                chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap_or(last)
            } else {
                last
            };
            let monday =
                anchor - chrono::Duration::days(i64::from(anchor.weekday().num_days_from_monday()));
            Ok(Window {
                from: monday.to_string(),
                to: (monday + chrono::Duration::days(6)).to_string(),
            })
        }
    }
}

fn spent_in_window(
    conn: &Connection,
    category_id: &str,
    from: &str,
    to: &str,
) -> Result<i64, AppError> {
    // `p.amount > 0` on purpose: reports count gross spending too
    // (`reports/common.rs`), so a refund recorded as a negative posting on the
    // category does not make the budget card and the category breakdown show
    // two different numbers for the same month. Net-refund accounting is a
    // product decision, tracked in docs/design/21; until then both sides use the
    // same rule.
    let spent = conn.query_row(
        "SELECT COALESCE(SUM(p.amount), 0)
           FROM postings p
           JOIN accounts a ON a.id = p.account_id
           JOIN categories c ON c.account_id = a.id
           JOIN transactions t ON t.id = p.transaction_id
          WHERE c.id = ?1 AND t.date >= ?2 AND t.date <= ?3 AND p.amount > 0",
        rusqlite::params![category_id, from, to],
        |row| row.get(0),
    )?;
    Ok(spent)
}

/// Start of the period that contains `date` (Monday for weekly budgets).
fn period_start_of(
    date: chrono::NaiveDate,
    period: &BudgetPeriod,
) -> Result<chrono::NaiveDate, AppError> {
    match period {
        BudgetPeriod::Monthly => first_of_month(date.year(), date.month() as i32),
        BudgetPeriod::Weekly => {
            Ok(date - chrono::Duration::days(i64::from(date.weekday().num_days_from_monday())))
        }
    }
}

/// Last day of the period starting at `start`.
fn period_end_of(start: chrono::NaiveDate, period: &BudgetPeriod) -> chrono::NaiveDate {
    match period {
        BudgetPeriod::Monthly => last_of_month(start),
        BudgetPeriod::Weekly => start + chrono::Duration::days(6),
    }
}

fn next_period(
    start: chrono::NaiveDate,
    period: &BudgetPeriod,
) -> Result<chrono::NaiveDate, AppError> {
    match period {
        BudgetPeriod::Monthly => {
            let (year, month) = (start.year(), start.month());
            if month == 12 {
                chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(invalid_month)
            } else {
                chrono::NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or_else(invalid_month)
            }
        }
        BudgetPeriod::Weekly => Ok(start + chrono::Duration::days(7)),
    }
}

/// Unused amount carried into the current period.
///
/// Walks forward from the period the budget was created in, clamping each period
/// at zero: overspending is not debt that eats the next budget. Works for both
/// periods — a weekly budget carries last week's leftover, a monthly one last
/// month's.
///
/// Bounded at [`MAX_ROLLOVER_PERIODS`] periods (roughly five years either way),
/// because an unbounded walk over a decade-old budget costs a query per period
/// on every page load.
fn rollover_carry(
    conn: &Connection,
    budget: &BudgetRecord,
    current_start: &chrono::NaiveDate,
) -> Result<i64, AppError> {
    const MAX_ROLLOVER_PERIODS: usize = 260;

    let created = budget
        .created_at
        .get(..10)
        .and_then(|date| chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").ok());
    let mut cursor = match created {
        Some(date) => period_start_of(date, &budget.period)?,
        None => return Ok(0),
    };

    let mut carry: i64 = 0;
    let mut periods = 0;
    while cursor < *current_start && periods < MAX_ROLLOVER_PERIODS {
        let end = period_end_of(cursor, &budget.period);
        let spent = spent_in_window(
            conn,
            &budget.category_id,
            &cursor.to_string(),
            &end.to_string(),
        )?;
        carry = (carry + budget.amount - spent).max(0);
        cursor = next_period(cursor, &budget.period)?;
        periods += 1;
    }
    Ok(carry)
}

/// Calculate budget status for a single budget.
#[allow(dead_code)]
pub fn calculate_budget_status(
    conn: &Connection,
    budget: &BudgetRecord,
    year: i32,
    month: i32,
) -> Result<BudgetStatusRecord, AppError> {
    let category = find_category(conn, &budget.category_id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    let window = budget_window(&budget.period, year, month)?;
    let spent = spent_in_window(conn, &budget.category_id, &window.from, &window.to)?;
    let current_start =
        chrono::NaiveDate::parse_from_str(&window.from, "%Y-%m-%d").map_err(|_| invalid_month())?;
    let rollover_amount = if budget.rollover {
        rollover_carry(conn, budget, &current_start)?
    } else {
        0
    };

    let available = budget.amount + rollover_amount;
    let remaining = available - spent;

    Ok(BudgetStatusRecord {
        id: budget.id.clone(),
        category_id: budget.category_id.clone(),
        category_name: category.name,
        category_icon: category.icon,
        amount: budget.amount,
        period: budget.period.clone(),
        rollover: budget.rollover,
        spent,
        remaining,
        over_budget: spent > available,
        rollover_amount,
        available,
    })
}

/// List all budget statuses with spending information for a specific year/month.
///
/// One aggregated query: each budget's window depends on its own period, so the
/// two windows are passed in and selected per row instead of running a query per
/// budget. Rollover is applied afterwards because it needs history, not a single
/// aggregate.
pub fn list_budget_statuses(
    conn: &Connection,
    year: i32,
    month: i32,
) -> Result<Vec<BudgetStatusRecord>, AppError> {
    let month_window = budget_window(&BudgetPeriod::Monthly, year, month)?;
    let week_window = budget_window(&BudgetPeriod::Weekly, year, month)?;

    struct Row {
        id: String,
        category_id: String,
        category_name: String,
        category_icon: Option<String>,
        amount: i64,
        period: BudgetPeriod,
        rollover: bool,
        created_at: String,
        spent: i64,
    }

    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                b.id, b.category_id, c.name, c.icon, b.amount, b.period, b.rollover,
                b.created_at,
                COALESCE(SUM(
                    CASE WHEN t.date >= (CASE WHEN b.period = 'weekly' THEN ?3 ELSE ?1 END)
                          AND t.date <= (CASE WHEN b.period = 'weekly' THEN ?4 ELSE ?2 END)
                          AND p.amount > 0
                         THEN p.amount ELSE 0 END
                ), 0) as spent
            FROM budgets b
            JOIN categories c ON c.id = b.category_id
            LEFT JOIN postings p ON p.account_id = c.account_id
            LEFT JOIN transactions t ON t.id = p.transaction_id
            GROUP BY b.id
            ORDER BY b.created_at DESC
            "#,
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let rows: Vec<Row> = stmt
        .query_map(
            rusqlite::params![
                &month_window.from,
                &month_window.to,
                &week_window.from,
                &week_window.to
            ],
            |row| {
                let period_str: String = row.get(5)?;
                let period = match period_str.as_str() {
                    "monthly" => BudgetPeriod::Monthly,
                    "weekly" => BudgetPeriod::Weekly,
                    _ => return Err(rusqlite::Error::InvalidQuery),
                };
                Ok(Row {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    category_name: row.get(2)?,
                    category_icon: row.get(3)?,
                    amount: row.get(4)?,
                    period,
                    rollover: row.get::<_, i32>(6)? != 0,
                    created_at: row.get(7)?,
                    spent: row.get(8)?,
                })
            },
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    drop(stmt);

    let mut statuses = Vec::with_capacity(rows.len());
    for row in rows {
        let period_window = budget_window(&row.period, year, month)?;
        let rollover_amount = if row.rollover {
            let budget = BudgetRecord {
                id: row.id.clone(),
                category_id: row.category_id.clone(),
                amount: row.amount,
                period: row.period.clone(),
                rollover: row.rollover,
                created_at: row.created_at.clone(),
                updated_at: row.created_at.clone(),
            };
            let current_start = chrono::NaiveDate::parse_from_str(&period_window.from, "%Y-%m-%d")
                .map_err(|_| invalid_month())?;
            rollover_carry(conn, &budget, &current_start)?
        } else {
            0
        };
        let available = row.amount + rollover_amount;
        statuses.push(BudgetStatusRecord {
            id: row.id,
            category_id: row.category_id,
            category_name: row.category_name,
            category_icon: row.category_icon,
            amount: row.amount,
            period: row.period,
            rollover: row.rollover,
            spent: row.spent,
            remaining: available - row.spent,
            over_budget: row.spent > available,
            rollover_amount,
            available,
        });
    }

    Ok(statuses)
}

// ---------------------------------------------------------------------------
// Create operation
// ---------------------------------------------------------------------------

/// Create a new budget for an expense category.
/// Validates:
/// - Category must exist and be expense type
/// - No duplicate budget for same category
/// - Amount must be positive
pub fn create_budget(
    conn: &Connection,
    category_id: &str,
    amount: i64,
    period: &BudgetPeriod,
    rollover: bool,
) -> Result<String, AppError> {
    // Validate category exists and is expense type
    let category = find_category(conn, category_id)?
        .ok_or_else(|| AppError::ValidationError("errors.categoryNotFound".to_string()))?;

    if category.category_type != CategoryType::Expense {
        return Err(AppError::ValidationError(
            "errors.budgetOnlyForExpense".to_string(),
        ));
    }

    // Check for duplicate budget
    if let Some(_existing) = find_by_category(conn, category_id)? {
        return Err(AppError::ValidationError(
            "errors.budgetAlreadyExists".to_string(),
        ));
    }

    // Validate amount is positive
    if amount <= 0 {
        return Err(AppError::ValidationError(
            "errors.budgetAmountPositive".to_string(),
        ));
    }

    let id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    let period_str = match period {
        BudgetPeriod::Monthly => "monthly",
        BudgetPeriod::Weekly => "weekly",
    };
    let rollover_int = if rollover { 1 } else { 0 };

    conn.execute(
        &format!(
            "INSERT INTO budgets ({}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
            BUDGET_COLUMNS
        ),
        rusqlite::params![&id, category_id, amount, period_str, rollover_int, &now],
    )?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// Update operation
// ---------------------------------------------------------------------------

/// Parameters for updating a budget.
pub struct UpdateBudgetParams<'a> {
    pub id: &'a str,
    pub amount: Option<i64>,
    pub period: Option<BudgetPeriod>,
    pub rollover: Option<bool>,
}

/// Update budget fields.
pub fn update_budget(conn: &Connection, params: &UpdateBudgetParams<'_>) -> Result<(), AppError> {
    // Verify budget exists
    let _budget = find_budget(conn, params.id)?
        .ok_or_else(|| AppError::ValidationError("errors.budgetNotFound".to_string()))?;

    // Validate amount if provided
    if let Some(amount) = params.amount {
        if amount <= 0 {
            return Err(AppError::ValidationError(
                "errors.budgetAmountPositive".to_string(),
            ));
        }
    }

    let now = crate::utils::time::now_rfc3339();
    let mut sets: Vec<String> = Vec::new();
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(val) = params.amount {
        sets.push("amount = ?".to_string());
        sql_params.push(Box::new(val));
    }

    if let Some(ref val) = params.period {
        sets.push("period = ?".to_string());
        let period_str = match val {
            BudgetPeriod::Monthly => "monthly",
            BudgetPeriod::Weekly => "weekly",
        };
        sql_params.push(Box::new(period_str.to_string()));
    }

    if let Some(val) = params.rollover {
        sets.push("rollover = ?".to_string());
        let rollover_int = if val { 1 } else { 0 };
        sql_params.push(Box::new(rollover_int));
    }

    if sets.is_empty() {
        return Ok(());
    }

    sets.push("updated_at = ?".to_string());
    sql_params.push(Box::new(now));
    sql_params.push(Box::new(params.id.to_string()));

    let sql = format!("UPDATE budgets SET {} WHERE id = ?", sets.join(", "));
    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();
    stmt.execute(rusqlite::params_from_iter(param_refs))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Delete operation
// ---------------------------------------------------------------------------

/// Delete a budget by ID.
pub fn delete_budget(conn: &Connection, id: &str) -> Result<(), AppError> {
    // Verify budget exists
    let _budget = find_budget(conn, id)?
        .ok_or_else(|| AppError::ValidationError("errors.budgetNotFound".to_string()))?;

    conn.execute("DELETE FROM budgets WHERE id = ?1", [id])?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::categories::{create_category, CategoryType};
    use tempfile::TempDir;

    fn test_conn() -> (TempDir, Connection) {
        // The real schema, not a hand-written copy.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        crate::db::init_schema(&conn).unwrap();
        (dir, conn)
    }

    fn make_category(conn: &Connection, name: &str, category_type: &CategoryType) -> String {
        create_category(conn, name, category_type, None).unwrap()
    }

    /// Reuse an existing account when `(type, name)` is already taken: since
    /// v2 every category owns exactly one P&L account of the same name, so
    /// creating a second "餐饮" would (rightly) violate the unique index.
    fn make_account(conn: &Connection, name: &str, account_type: &str) -> String {
        if let Ok(id) = conn.query_row(
            "SELECT id FROM accounts WHERE type = ?1 AND name = ?2",
            rusqlite::params![account_type, name],
            |row| row.get::<_, String>(0),
        ) {
            return id;
        }
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

    /// The category is carried by the postings themselves: one of them must sit
    /// on the category's mirror account for the budget to see the spend.
    #[allow(dead_code)]
    fn make_categorized_transaction(
        conn: &Connection,
        date: &str,
        description: &str,
        postings: &[(String, i64)],
    ) -> String {
        let tx_id = Uuid::new_v4().to_string();
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)",
            rusqlite::params![&tx_id, date, description, &now],
        )
        .unwrap();

        for (seq, (account_id, amount)) in postings.iter().enumerate() {
            let posting_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![&posting_id, &tx_id, account_id, *amount, seq as i32, &now],
            )
            .unwrap();
        }

        tx_id
    }

    #[test]
    fn test_create_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        assert!(!budget_id.is_empty());

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.category_id, cat_id);
        assert_eq!(budget.amount, 10000);
        assert_eq!(budget.period, BudgetPeriod::Monthly);
        assert!(!budget.rollover);
    }

    #[test]
    fn test_create_budget_income_category_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "工资", &CategoryType::Income);

        let result = create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetOnlyForExpense");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_create_duplicate_budget_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);

        create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let result = create_budget(&conn, &cat_id, 5000, &BudgetPeriod::Weekly, true);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetAlreadyExists");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_create_budget_zero_amount_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);

        let result = create_budget(&conn, &cat_id, 0, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetAmountPositive");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }

        let result = create_budget(&conn, &cat_id, -100, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(15000),
                period: Some(BudgetPeriod::Weekly),
                rollover: Some(true),
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 15000);
        assert_eq!(budget.period, BudgetPeriod::Weekly);
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_zero_amount_blocked() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let result = update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(0),
                period: None,
                rollover: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_budget() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        delete_budget(&conn, &budget_id).unwrap();
        assert!(find_budget(&conn, &budget_id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_budget() {
        let (_dir, conn) = test_conn();

        let result = delete_budget(&conn, "nonexistent-id");
        assert!(result.is_err());

        match result.unwrap_err() {
            AppError::ValidationError(msg) => {
                assert_eq!(msg, "errors.budgetNotFound");
            }
            other => panic!("Expected ValidationError, got: {:?}", other),
        }
    }

    #[test]
    fn test_list_budgets() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let budgets = list_budgets(&conn).unwrap();
        assert_eq!(budgets.len(), 2);
    }

    #[test]
    fn test_find_by_category() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let found = find_by_category(&conn, &cat_id).unwrap().unwrap();
        assert_eq!(found.id, budget_id);

        // Category without budget should return None
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);
        assert!(find_by_category(&conn, &cat2_id).unwrap().is_none());
    }

    #[test]
    fn test_budget_status_no_spending() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let _expense_acct = make_account(&conn, "餐饮", "expense");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
        assert_eq!(status.remaining, 10000);
        assert!(!status.over_budget);
    }

    #[test]
    fn test_budget_status_with_spending() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        make_categorized_transaction(
            &conn,
            "2024-04-15",
            "午餐",
            &[(expense_acct.clone(), 5000), (asset_acct.clone(), -5000)],
        );

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 5000);
        assert_eq!(status.remaining, 5000);
        assert!(!status.over_budget);
    }

    #[test]
    fn test_budget_status_ignores_other_dates() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        make_categorized_transaction(
            &conn,
            "2024-01-15",
            "历史消费",
            &[(expense_acct.clone(), 5000), (asset_acct.clone(), -5000)],
        );

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
    }

    /// The ledger is hard-delete: once the row is gone, so is its budget impact.
    /// (Before v2 this test relied on `deleted_at`, which no longer exists.)
    #[test]
    fn test_budget_status_after_hard_delete() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let expense_acct = make_account(&conn, "餐饮", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let tx_id = make_categorized_transaction(
            &conn,
            "2024-04-15",
            "午饭",
            &[(expense_acct.clone(), 5000), (asset_acct.clone(), -5000)],
        );
        assert_eq!(
            calculate_budget_status(&conn, &budget, 2024, 4)
                .unwrap()
                .spent,
            5000
        );

        conn.execute("DELETE FROM transactions WHERE id = ?1", [&tx_id])
            .unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.spent, 0);
    }

    #[test]
    fn test_list_budget_statuses() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "交通", &CategoryType::Expense);
        make_account(&conn, "餐饮", "expense");
        make_account(&conn, "交通", "expense");

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let statuses = list_budget_statuses(&conn, 2024, 4).unwrap();
        assert_eq!(statuses.len(), 2);

        let food_status = statuses.iter().find(|s| s.category_name == "餐饮").unwrap();
        assert!(food_status.category_icon.is_none());

        let transport_status = statuses.iter().find(|s| s.category_name == "交通").unwrap();
        assert!(transport_status.rollover);
    }

    #[test]
    fn test_create_budget_with_weekly_period() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "交通", &CategoryType::Expense);
        let budget_id = create_budget(&conn, &cat_id, 5000, &BudgetPeriod::Weekly, true).unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.period, BudgetPeriod::Weekly);
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_partial() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // Update only amount
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: Some(15000),
                period: None,
                rollover: None,
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 15000);
        assert_eq!(budget.period, BudgetPeriod::Monthly); // unchanged
        assert!(!budget.rollover); // unchanged

        // Update only rollover
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: None,
                period: None,
                rollover: Some(true),
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert!(budget.rollover);
    }

    #[test]
    fn test_update_budget_no_changes() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // No changes should succeed without error
        update_budget(
            &conn,
            &UpdateBudgetParams {
                id: &budget_id,
                amount: None,
                period: None,
                rollover: None,
            },
        )
        .unwrap();

        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();
        assert_eq!(budget.amount, 10000);
    }

    #[test]
    fn test_update_nonexistent_budget() {
        let (_dir, conn) = test_conn();

        let result = update_budget(
            &conn,
            &UpdateBudgetParams {
                id: "nonexistent-id",
                amount: Some(10000),
                period: None,
                rollover: None,
            },
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_category_icon_included() {
        let (_dir, conn) = test_conn();
        let cat_id = crate::db::categories::create_category(
            &conn,
            "餐饮",
            &CategoryType::Expense,
            Some("🍔"),
        )
        .unwrap();
        make_account(&conn, "餐饮", "expense");

        let budget_id =
            create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let budget = find_budget(&conn, &budget_id).unwrap().unwrap();

        let status = calculate_budget_status(&conn, &budget, 2024, 4).unwrap();
        assert_eq!(status.category_icon, Some("🍔".to_string()));
    }

    #[test]
    fn test_budget_status_different_category_same_account_name() {
        let (_dir, conn) = test_conn();
        let cat1_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let cat2_id = make_category(&conn, "外卖", &CategoryType::Expense);

        let food_acct = make_account(&conn, "餐饮", "expense");
        let delivery_acct = make_account(&conn, "外卖", "expense");
        let asset_acct = make_account(&conn, "Cash", "asset");

        create_budget(&conn, &cat1_id, 10000, &BudgetPeriod::Monthly, false).unwrap();
        create_budget(&conn, &cat2_id, 5000, &BudgetPeriod::Monthly, false).unwrap();

        make_categorized_transaction(
            &conn,
            "2024-04-15",
            "午餐",
            &[(food_acct.clone(), 3000), (asset_acct.clone(), -3000)],
        );

        make_categorized_transaction(
            &conn,
            "2024-04-15",
            "外卖",
            &[(delivery_acct.clone(), 2000), (asset_acct.clone(), -2000)],
        );

        let statuses = list_budget_statuses(&conn, 2024, 4).unwrap();

        let food_status = statuses.iter().find(|s| s.category_name == "餐饮").unwrap();
        assert_eq!(food_status.spent, 3000);

        let delivery_status = statuses.iter().find(|s| s.category_name == "外卖").unwrap();
        assert_eq!(delivery_status.spent, 2000);
    }

    #[test]
    fn test_list_budget_statuses_december() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        let statuses = list_budget_statuses(&conn, 2024, 12).unwrap();
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].spent, 0);
    }

    /// A weekly budget is measured over a week. Measuring it over the whole
    /// month reported ~4x the real overspend for anyone who used one.
    #[test]
    fn test_weekly_budget_counts_only_the_current_week() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let food_acct = make_account(&conn, "餐饮", "expense");
        let cash = make_account(&conn, "Cash", "asset");

        create_budget(&conn, &cat_id, 50000, &BudgetPeriod::Weekly, false).unwrap();

        let today =
            chrono::NaiveDate::parse_from_str(&crate::utils::time::today_date(), "%Y-%m-%d")
                .expect("today parses");
        let (year, month) = (today.year(), today.month() as i32);
        let window = budget_window(&BudgetPeriod::Weekly, year, month).unwrap();
        let start = chrono::NaiveDate::parse_from_str(&window.from, "%Y-%m-%d").unwrap();
        let end = chrono::NaiveDate::parse_from_str(&window.to, "%Y-%m-%d").unwrap();

        assert_eq!((end - start).num_days(), 6, "a week is seven days");
        assert_eq!(
            start.weekday(),
            chrono::Weekday::Mon,
            "weeks run Monday to Sunday"
        );

        // Inside the week.
        make_categorized_transaction(
            &conn,
            &start.to_string(),
            "本周午餐",
            &[(food_acct.clone(), 12000), (cash.clone(), -12000)],
        );
        // Outside it, but inside the same month: must not count.
        make_categorized_transaction(
            &conn,
            &(start - chrono::Duration::days(20)).to_string(),
            "三周前",
            &[(food_acct.clone(), 30000), (cash.clone(), -30000)],
        );

        let statuses = list_budget_statuses(&conn, year, month).unwrap();
        assert_eq!(
            statuses[0].spent, 12000,
            "only the current week's spending belongs to a weekly budget"
        );
        assert_eq!(statuses[0].available, 50000);
        assert!(!statuses[0].over_budget);
    }

    /// Unused budget from the previous period carries over when the switch is
    /// on — the field used to be hardcoded to zero while still being exposed.
    #[test]
    fn test_monthly_rollover_carries_last_months_leftover() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let food_acct = make_account(&conn, "餐饮", "expense");
        let cash = make_account(&conn, "Cash", "asset");

        let budget_id = create_budget(&conn, &cat_id, 50000, &BudgetPeriod::Monthly, true).unwrap();
        // Pretend the budget has existed since October.
        conn.execute(
            "UPDATE budgets SET created_at = '2026-10-05T00:00:00Z' WHERE id = ?1",
            [&budget_id],
        )
        .unwrap();

        // October: 300.00 of 500.00 spent -> 200.00 carries into November.
        make_categorized_transaction(
            &conn,
            "2026-10-15",
            "十月午餐",
            &[(food_acct.clone(), 30000), (cash.clone(), -30000)],
        );

        let statuses = list_budget_statuses(&conn, 2026, 11).unwrap();
        assert_eq!(statuses[0].spent, 0);
        assert_eq!(statuses[0].rollover_amount, 20000);
        assert_eq!(statuses[0].available, 70000);
        assert_eq!(statuses[0].remaining, 70000);

        // November spending beyond 500.00 is still fine thanks to the carry.
        make_categorized_transaction(
            &conn,
            "2026-11-03",
            "十一月午餐",
            &[(food_acct.clone(), 60000), (cash.clone(), -60000)],
        );
        let statuses = list_budget_statuses(&conn, 2026, 11).unwrap();
        assert_eq!(statuses[0].spent, 60000);
        assert_eq!(statuses[0].available, 70000);
        assert!(!statuses[0].over_budget);
        assert_eq!(statuses[0].remaining, 10000);
    }

    /// Overspending one period must not make the next period's budget smaller.
    #[test]
    fn test_rollover_never_goes_negative() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        let food_acct = make_account(&conn, "餐饮", "expense");
        let cash = make_account(&conn, "Cash", "asset");

        let budget_id = create_budget(&conn, &cat_id, 50000, &BudgetPeriod::Monthly, true).unwrap();
        conn.execute(
            "UPDATE budgets SET created_at = '2026-10-05T00:00:00Z' WHERE id = ?1",
            [&budget_id],
        )
        .unwrap();
        make_categorized_transaction(
            &conn,
            "2026-10-15",
            "十月超支",
            &[(food_acct, 90000), (cash, -90000)],
        );

        let statuses = list_budget_statuses(&conn, 2026, 11).unwrap();
        assert_eq!(statuses[0].rollover_amount, 0);
        assert_eq!(statuses[0].available, 50000);
    }

    #[test]
    fn test_budget_status_rejects_invalid_month() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        create_budget(&conn, &cat_id, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // Used to panic in `from_ymd_opt(...).unwrap()` while holding both
        // database mutexes.
        assert!(list_budget_statuses(&conn, 2024, 13).is_err());
        assert!(list_budget_statuses(&conn, 2024, 0).is_err());
    }

    #[test]
    fn test_list_budgets_invalid_period_row() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        conn.execute(
            "INSERT INTO budgets (id, category_id, amount, period, rollover, created_at, updated_at)
             VALUES ('budget-bad', ?1, 1000, 'bad-period', 0, '2024-01-01T00:00:00+08:00', '2024-01-01T00:00:00+08:00')",
            [&cat_id],
        )
        .unwrap();

        assert!(list_budgets(&conn).is_err());
    }

    #[test]
    fn test_list_budget_statuses_invalid_period_row() {
        let (_dir, conn) = test_conn();
        let cat_id = make_category(&conn, "餐饮", &CategoryType::Expense);
        conn.execute_batch("PRAGMA ignore_check_constraints = ON;")
            .unwrap();
        conn.execute(
            "INSERT INTO budgets (id, category_id, amount, period, rollover, created_at, updated_at)
             VALUES ('budget-bad-2', ?1, 1000, 'bad-period', 0, '2024-01-01T00:00:00+08:00', '2024-01-01T00:00:00+08:00')",
            [&cat_id],
        )
        .unwrap();

        assert!(list_budget_statuses(&conn, 2024, 1).is_err());
    }

    #[test]
    fn test_create_budget_missing_table() {
        let dir = TempDir::new().unwrap();
        let conn = Connection::open(dir.path().join("bare.db")).unwrap();
        let result = create_budget(&conn, "cat-1", 1000, &BudgetPeriod::Monthly, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_split_transaction_counts_toward_each_budget() {
        let (_dir, conn) = test_conn();
        let food_cat = make_category(&conn, "餐饮", &CategoryType::Expense);
        let transport_cat = make_category(&conn, "交通", &CategoryType::Expense);
        let food_acct = crate::db::categories::account_id_for_category(&conn, &food_cat).unwrap();
        let transport_acct =
            crate::db::categories::account_id_for_category(&conn, &transport_cat).unwrap();
        let asset_acct = make_account(&conn, "Cash", "asset");

        let food_budget =
            create_budget(&conn, &food_cat, 10000, &BudgetPeriod::Monthly, false).unwrap();
        let transport_budget =
            create_budget(&conn, &transport_cat, 10000, &BudgetPeriod::Monthly, false).unwrap();

        // One split payment: 3000 for food, 2000 for transport.
        make_categorized_transaction(
            &conn,
            "2024-04-15",
            "午餐+打车",
            &[
                (food_acct, 3000),
                (transport_acct, 2000),
                (asset_acct, -5000),
            ],
        );

        let food_status = calculate_budget_status(
            &conn,
            &find_budget(&conn, &food_budget).unwrap().unwrap(),
            2024,
            4,
        )
        .unwrap();
        assert_eq!(
            food_status.spent, 3000,
            "only the food half hits the budget"
        );

        let transport_status = calculate_budget_status(
            &conn,
            &find_budget(&conn, &transport_budget).unwrap().unwrap(),
            2024,
            4,
        )
        .unwrap();
        assert_eq!(transport_status.spent, 2000);
    }
}
