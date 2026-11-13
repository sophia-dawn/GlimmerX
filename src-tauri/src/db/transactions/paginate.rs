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

    let items_sql = format!(
        "SELECT
            t.id, t.date, t.description, t.category_id, t.created_at, t.updated_at,
            c.name as category_name,
            c.icon as category_icon,
            COALESCE(SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END), 0) as display_amount,
            GROUP_CONCAT(
                a.name || ':' || CASE
                    WHEN p.amount > 0 THEN '+' || CAST(ABS(p.amount) AS TEXT)
                    ELSE '-' || CAST(ABS(p.amount) AS TEXT)
                END,
                '|'
            ) as postings_data
         FROM transactions t
         LEFT JOIN categories c ON c.id = t.category_id
         JOIN postings p ON p.transaction_id = t.id
         JOIN accounts a ON a.id = p.account_id
         WHERE {} AND t.deleted_at IS NULL
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
                category_id: row.get(3)?,
                category_name: row.get(6)?,
                category_icon: row.get(7)?,
                postings_summary,
                display_amount: row.get(8)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
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
        conditions.push("t.category_id = ?".to_string());
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
