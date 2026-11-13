//! Per-account reports: transaction ledger and balance trend.

use rusqlite::{params, Connection};

use super::common::{get_period_format, resolve_date_range};
use super::types::{
    AccountBalanceTrendReportDto, AccountTransactionItem, AccountTransactionsReportDto,
    BalancePoint, ReportFilter,
};

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

    // Categories and counter accounts come from the *other* postings of the
    // transaction: a split transaction has several of each, so both are
    // concatenated rather than picked arbitrarily. Reading them off
    // `transactions.category_id` — a single cached category — would report one
    // category for a transaction that legitimately has three.
    let sql = "
        SELECT
            t.id as transaction_id,
            t.date,
            t.description,
            (
                SELECT GROUP_CONCAT(DISTINCT c2.name)
                FROM postings p2
                JOIN accounts a2 ON a2.id = p2.account_id
                JOIN categories c2 ON c2.account_id = a2.id
                WHERE p2.transaction_id = t.id
            ) as category_name,
            p.amount,
            (
                SELECT GROUP_CONCAT(a2.name)
                FROM postings p2
                JOIN accounts a2 ON a2.id = p2.account_id AND a2.id != ?1
                WHERE p2.transaction_id = t.id
            ) as counter_account_name
        FROM postings p
        JOIN transactions t ON t.id = p.transaction_id
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
        JOIN transactions t ON t.id = p.transaction_id
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
            JOIN transactions t ON t.id = p.transaction_id
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
            JOIN transactions t ON t.id = p.transaction_id
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
