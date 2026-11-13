//! Paginated listing with filtering and date grouping.

use std::collections::HashMap;

use itertools::Itertools;
use rusqlite::Connection;

use crate::db::AppError;

use super::types::{
    PaginationInfo, TransactionDateGroup, TransactionFilter, TransactionListItem,
    TransactionListResponse,
};

/// Get paginated transaction list with filtering and date grouping.
///
/// Uses single JOIN query to avoid N+1 problem.
/// All amounts are calculated on backend per AGENTS.md constraint.
pub fn list_transactions_paginated(
    conn: &Connection,
    filter: &TransactionFilter,
) -> Result<TransactionListResponse, AppError> {
    let page = filter.page.unwrap_or(1).max(1);
    #[allow(clippy::manual_clamp)]
    let page_size = filter.page_size.unwrap_or(20).min(100).max(1);
    let offset = (page - 1) * page_size;

    let (where_clause, having_clause, params) = build_filter_where_clause(filter);

    let has_having = !having_clause.is_empty();

    let sort_by = filter.sort_by.as_deref().unwrap_or("date");
    let sort_order = filter.sort_order.as_deref().unwrap_or("desc");
    let sort_column = match sort_by {
        "amount" => "display_amount",
        "description" => "t.description",
        _ => "t.date",
    };
    let sort_dir = if sort_order == "asc" { "ASC" } else { "DESC" };
    let secondary_sort = if sort_column == "t.date" {
        ", t.created_at DESC"
    } else {
        ""
    };

    let having_sql = if has_having {
        format!("HAVING {}", having_clause)
    } else {
        "".to_string()
    };

    // Categories are derived from the postings: `categories.account_id` is the
    // ledger account a posting points at, so a split transaction yields several
    // names here and one cached `transactions.category_id` could never do that.
    //
    // The name list is a correlated subquery rather than a joined aggregate: as
    // a joined `GROUP BY transaction_id` it was materialised for the whole
    // ledger on every page (60k postings before the first row was emitted),
    // while here it runs once per returned row against
    // `idx_postings_transaction`. `DISTINCT` matters: two postings in the same
    // category used to render as "餐饮、餐饮".
    // `display_amount` is the debit total, so it is always >= 0 and can never
    // tell a payment from a receipt. `flow_amount` sums only the balance-sheet
    // side: an expense leaves it negative, income positive, a transfer between
    // two of the user's own accounts zero.
    let cash_flow_types = format!(
        "a.type IN ('{}', '{}')",
        crate::constants::account_type::ASSET,
        crate::constants::account_type::LIABILITY
    );

    let items_sql = format!(
        "SELECT
            t.id, t.date, t.description, t.created_at, t.updated_at,
            COALESCE((
                SELECT GROUP_CONCAT(name, '|') FROM (
                    SELECT DISTINCT c.name AS name
                      FROM postings p2
                      JOIN accounts a2 ON a2.id = p2.account_id
                      JOIN categories c ON c.account_id = a2.id
                     WHERE p2.transaction_id = t.id
                     ORDER BY c.name
                )
            ), '') as category_names,
            (SELECT MIN(c.icon)
               FROM postings p2
               JOIN accounts a2 ON a2.id = p2.account_id
               JOIN categories c ON c.account_id = a2.id
              WHERE p2.transaction_id = t.id) as category_icon,
            COALESCE(SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END), 0) as display_amount,
            COALESCE(SUM(CASE WHEN {cash_flow_types} THEN p.amount ELSE 0 END), 0) as flow_amount,
            GROUP_CONCAT(
                a.name || ':' || CASE
                    WHEN p.amount > 0 THEN '+' || CAST(ABS(p.amount) AS TEXT)
                    ELSE '-' || CAST(ABS(p.amount) AS TEXT)
                END,
                '|'
            ) as postings_data
         FROM transactions t
         JOIN postings p ON p.transaction_id = t.id
         JOIN accounts a ON a.id = p.account_id
         WHERE {}
         GROUP BY t.id
         {}
         ORDER BY {} {}{}
         LIMIT ? OFFSET ?",
        if where_clause.is_empty() {
            "1=1"
        } else {
            &where_clause
        },
        having_sql,
        sort_column,
        sort_dir,
        secondary_sort
    );

    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = params;
    // Fetch one extra row to determine whether another page exists without
    // running an expensive COUNT over the full filtered dataset.
    let fetch_limit = page_size + 1;
    sql_params.push(Box::new(fetch_limit as i32));
    sql_params.push(Box::new(offset as i32));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();

    let mut items: Vec<TransactionListItem> = conn
        .prepare(&items_sql)?
        .query_map(rusqlite::params_from_iter(param_refs), |row| {
            let postings_data: Option<String> = row.get::<_, Option<String>>(9)?;

            let postings_summary = postings_data
                .and_then(|data| parse_postings_summary(&data))
                .unwrap_or_default();

            Ok(TransactionListItem {
                id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                category_names: parse_category_names(row.get::<_, String>(5)?.as_str()),
                category_icon: row.get(6)?,
                postings_summary,
                display_amount: row.get(7)?,
                flow_amount: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let has_next = items.len() > page_size as usize;
    items.truncate(page_size as usize);

    let date_groups = build_date_groups(&items);

    Ok(TransactionListResponse {
        items: items.clone(),
        pagination: PaginationInfo {
            page,
            page_size,
            total_count: None,
            total_pages: None,
            has_next,
            has_prev: page > 1,
        },
        date_groups,
    })
}

// ---------------------------------------------------------------------------
// Helper functions for paginated list
// ---------------------------------------------------------------------------

fn build_filter_where_clause(
    filter: &TransactionFilter,
) -> (String, String, Vec<Box<dyn rusqlite::types::ToSql>>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut having_conditions: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(ref from) = filter.from_date {
        conditions.push("t.date >= ?".to_string());
        params.push(Box::new(from.clone()));
    }

    if let Some(ref to) = filter.to_date {
        conditions.push("t.date <= ?".to_string());
        params.push(Box::new(to.clone()));
    }

    if let Some(ref acct_id) = filter.account_id {
        conditions.push("p.account_id = ?".to_string());
        params.push(Box::new(acct_id.clone()));
    }

    if let Some(ref cat_id) = filter.category_id {
        // A split transaction can contain the category on any of its postings,
        // so this cannot be a plain column comparison.
        conditions.push(
            "EXISTS (SELECT 1 FROM postings pc
                       JOIN accounts ac ON ac.id = pc.account_id
                       JOIN categories cc ON cc.account_id = ac.id
                      WHERE pc.transaction_id = t.id AND cc.id = ?)"
                .to_string(),
        );
        params.push(Box::new(cat_id.clone()));
    }

    if let Some(ref query) = filter.description_query {
        let escaped = escape_like_pattern(query);
        conditions.push("t.description LIKE ? ESCAPE '\\'".to_string());
        params.push(Box::new(format!("%{}%", escaped)));
    }

    if let Some(min) = filter.min_amount {
        having_conditions
            .push("SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END) >= ?".to_string());
        params.push(Box::new(min));
    }

    if let Some(max) = filter.max_amount {
        having_conditions
            .push("SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END) <= ?".to_string());
        params.push(Box::new(max));
    }

    let where_clause = conditions.join(" AND ");
    let having_clause = having_conditions.join(" AND ");
    (where_clause, having_clause, params)
}

fn build_date_groups(items: &[TransactionListItem]) -> Vec<TransactionDateGroup> {
    let mut groups: HashMap<String, TransactionDateGroup> = HashMap::new();

    for item in items {
        if !groups.contains_key(&item.date) {
            groups.insert(
                item.date.clone(),
                TransactionDateGroup {
                    date: item.date.clone(),
                    date_display: format_date_display(&item.date),
                    items: Vec::new(),
                    day_total: 0,
                },
            );
        }

        let group = groups.get_mut(&item.date).unwrap();
        group.items.push(item.clone());
        group.day_total += item.display_amount;
    }

    groups
        .into_values()
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect()
}

pub(in crate::db::transactions) fn format_date_display(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let year: i32 = parts[0].parse().unwrap_or(0);
        let month: u32 = parts[1].parse().unwrap_or(0);
        let day: u32 = parts[2].parse().unwrap_or(0);
        format!("{}年{}月{}日", year, month, day)
    } else {
        date_str.to_string()
    }
}

/// Split the `category_names` aggregate back into individual names.
///
/// De-duplicated: the SQL already asks for `DISTINCT`, but a duplicate here
/// would read as "餐饮、餐饮" and inflate the "and N more" count on the card, so
/// the parser refuses to produce one.
pub(in crate::db::transactions) fn parse_category_names(data: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    data.split('|')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .filter(|name| seen.insert(name.to_string()))
        .map(str::to_string)
        .collect()
}

pub(in crate::db::transactions) fn parse_postings_summary(
    data: &str,
) -> Option<Vec<(String, i64)>> {
    let postings: Vec<(String, i64)> = data
        .split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() == 2 {
                let account = parts[0].trim();
                let amount_str = parts[1].trim();
                let amount: i64 = amount_str.parse().ok()?;
                Some((account.to_string(), amount))
            } else {
                None
            }
        })
        .collect();

    // Sort: negative amounts first (outflow), then positive (inflow)
    // Within each group, sort by absolute value descending
    let sorted = postings
        .into_iter()
        .sorted_by(|a, b| {
            match (a.1 < 0, b.1 < 0) {
                (true, false) => std::cmp::Ordering::Less, // negative first
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.1.abs().cmp(&a.1.abs()), // same sign: by abs desc
            }
        })
        .collect();

    Some(sorted)
}

fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
