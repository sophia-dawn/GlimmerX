//! Tests for this area of the transaction module.

use super::*;

/// Pagination returns exactly page_size items and reports has_next based
/// on fetching one extra row (no full COUNT scan).
#[test]
fn test_list_transactions_paginated_has_next() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Create 25 transactions
    for i in 0..25 {
        let _tx_id = create_transaction(
            &conn,
            &format!("2024-01-{:02}", (i % 28) + 1),
            &format!("Transaction {}", i),
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -100,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 100,
                },
            ],
        )
        .unwrap();
    }

    // Page 1 (20 items): has_next = true, no COUNT metadata
    let page1 = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            page: Some(1),
            page_size: Some(20),
            ..TransactionFilter::default()
        },
    )
    .unwrap();
    assert_eq!(page1.items.len(), 20);
    assert!(page1.pagination.has_next);
    assert!(page1.pagination.total_count.is_none());

    // Page 2 (remaining 5): has_next = false
    let page2 = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            page: Some(2),
            page_size: Some(20),
            ..TransactionFilter::default()
        },
    )
    .unwrap();
    assert_eq!(page2.items.len(), 5);
    assert!(!page2.pagination.has_next);
    assert!(page2.pagination.has_prev);
}

// ---------------------------------------------------------------------------
// QuickAdd Tests
// ---------------------------------------------------------------------------

#[test]
fn test_list_transactions_paginated_sort_by_amount() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");
    create_transaction(
        &conn,
        "2024-01-15",
        "A",
        None,
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -100,
            },
            PostingInput {
                account_id: food.clone(),
                amount: 100,
            },
        ],
    )
    .unwrap();

    let response = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            from_date: None,
            to_date: None,
            min_amount: None,
            max_amount: None,
            account_id: None,
            category_id: None,
            description_query: None,
            page: Some(1),
            page_size: Some(10),
            sort_by: Some("amount".to_string()),
            sort_order: Some("asc".to_string()),
        },
    )
    .unwrap();
    assert_eq!(response.items.len(), 1);
    assert_eq!(response.items[0].display_amount, 100);
}

#[test]
fn test_format_date_display_invalid() {
    assert_eq!(format_date_display("2024"), "2024");
    assert_eq!(format_date_display("2024-01-15"), "2024年1月15日");
}

#[test]
fn test_parse_postings_summary_malformed() {
    let parsed = parse_postings_summary("no-colon").unwrap();
    assert!(parsed.is_empty());
    let parsed = parse_postings_summary("Cash:-100|Food:100").unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].0, "Cash");
}
