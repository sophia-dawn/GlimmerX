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

/// `duplicateAccount` keeps the same category out of a transaction twice, so a
/// repeated name should not reach the parser — but if it ever does, "餐饮、餐饮"
/// and an inflated "and N more" count is a worse failure than a set join.
#[test]
fn test_parse_category_names_drops_repeats() {
    assert_eq!(
        parse_category_names("Food|Food|Transport"),
        vec!["Food", "Transport"]
    );
    assert_eq!(parse_category_names(""), Vec::<String>::new());
    assert_eq!(parse_category_names("Food||"), vec!["Food"]);
}

// ---------------------------------------------------------------------------
// Split transactions
// ---------------------------------------------------------------------------

/// A split transaction reports every category it touches, and filtering by any
/// one of them finds it. Before v3 the list could only show one cached
/// category, so the other halves of a split were invisible.
#[test]
fn test_split_transaction_lists_every_category() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food_cat = make_category(&conn, "Food", "expense");
    let transport_cat = make_category(&conn, "Transport", "expense");
    let food = crate::db::categories::account_id_for_category(&conn, &food_cat).unwrap();
    let transport = crate::db::categories::account_id_for_category(&conn, &transport_cat).unwrap();

    create_transaction(
        &conn,
        "2024-01-15",
        "Lunch and taxi",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -300,
            },
            PostingInput {
                account_id: food,
                amount: 100,
            },
            PostingInput {
                account_id: transport,
                amount: 200,
            },
        ],
    )
    .unwrap();

    let response = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            page: Some(1),
            page_size: Some(10),
            ..TransactionFilter::default()
        },
    )
    .unwrap();
    let item = &response.items[0];
    assert_eq!(item.category_names, vec!["Food", "Transport"]);
    assert_eq!(
        item.display_amount, 300,
        "the whole outflow is one transaction"
    );
    assert_eq!(item.flow_amount, -300, "money left the balance sheet");

    // Filtering by either category finds the transaction; the displayed amount
    // stays the transaction total, because the filter asks "does this
    // transaction touch Food", not "how much did Food cost".
    for (category_id, name) in [(&food_cat, "Food"), (&transport_cat, "Transport")] {
        let filtered = list_transactions_paginated(
            &conn,
            &TransactionFilter {
                category_id: Some(category_id.clone()),
                page: Some(1),
                page_size: Some(10),
                ..TransactionFilter::default()
            },
        )
        .unwrap();
        assert_eq!(filtered.items.len(), 1, "{name} must match the split");
        assert_eq!(filtered.items[0].display_amount, 300);
    }

    // A category the transaction does not touch must not match.
    let other_cat = make_category(&conn, "Books", "expense");
    let none = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            category_id: Some(other_cat),
            page: Some(1),
            page_size: Some(10),
            ..TransactionFilter::default()
        },
    )
    .unwrap();
    assert!(none.items.is_empty());
}

/// `display_amount` is the debit total, so it is always positive and an
/// expense looked exactly like an income. `flow_amount` carries the sign the
/// card colours by: balance-sheet movement only.
#[test]
fn test_flow_amount_distinguishes_expense_income_and_transfer() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let wallet = make_account(&conn, "Wallet", "asset");
    let food = make_account(&conn, "Food", "expense");
    let salary = make_account(&conn, "Salary", "income");

    let expense = create_transaction(
        &conn,
        "2024-01-10",
        "Lunch",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -100,
            },
            PostingInput {
                account_id: food,
                amount: 100,
            },
        ],
    )
    .unwrap();

    let income = create_transaction(
        &conn,
        "2024-01-11",
        "Payday",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: 5000,
            },
            PostingInput {
                account_id: salary,
                amount: -5000,
            },
        ],
    )
    .unwrap();

    let transfer = create_transaction(
        &conn,
        "2024-01-12",
        "Top up wallet",
        &[
            PostingInput {
                account_id: cash,
                amount: -300,
            },
            PostingInput {
                account_id: wallet,
                amount: 300,
            },
        ],
    )
    .unwrap();

    let response = list_transactions_paginated(
        &conn,
        &TransactionFilter {
            page: Some(1),
            page_size: Some(10),
            sort_by: Some("date".to_string()),
            sort_order: Some("asc".to_string()),
            ..TransactionFilter::default()
        },
    )
    .unwrap();

    let by_id = |id: &str| {
        response
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("missing transaction {id}"))
    };

    assert_eq!(by_id(&expense).display_amount, 100);
    assert_eq!(
        by_id(&expense).flow_amount,
        -100,
        "expense leaves the wallet"
    );

    assert_eq!(by_id(&income).display_amount, 5000);
    assert_eq!(by_id(&income).flow_amount, 5000, "income arrives");

    // Moving money between two of the user's own accounts changes no net worth.
    assert_eq!(by_id(&transfer).display_amount, 300);
    assert_eq!(by_id(&transfer).flow_amount, 0, "transfer is neutral");
}
