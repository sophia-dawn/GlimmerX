//! Data integrity audit: unbalanced transactions, uncategorized entries and
//! account usage statistics.

use rusqlite::Connection;

use crate::utils::time::now_rfc3339;

use super::types::{
    AccountUsageStat, AuditReportDto, BalanceCheckResult, CategoryCheckResult,
    UnbalancedTransaction, UncategorizedTransaction,
};

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
