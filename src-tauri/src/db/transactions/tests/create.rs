//! Tests for this area of the transaction module.

use super::*;

/// Test 1: Balanced transaction creation should succeed.
/// Two postings with opposite amounts (e.g., +1000 and -1000) sum to 0.
#[test]
fn test_create_transaction_balanced() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let postings = [
        PostingInput {
            account_id: asset_id.clone(),
            amount: -1000, // money leaving asset
        },
        PostingInput {
            account_id: expense_id.clone(),
            amount: 1000, // money entering expense
        },
    ];

    let tx_id =
        create_transaction(&conn, "2024-01-15", "Grocery shopping", None, &postings).unwrap();

    assert!(!tx_id.is_empty());

    // Verify transaction was created correctly
    let tx_with_postings = get_transaction_with_postings(&conn, &tx_id)
        .unwrap()
        .expect("Transaction should exist");

    assert_eq!(tx_with_postings.transaction.date, "2024-01-15");
    assert_eq!(tx_with_postings.transaction.description, "Grocery shopping");
    assert_eq!(tx_with_postings.postings.len(), 2);

    // Verify postings are balanced (sum = 0)
    let sum: i64 = tx_with_postings.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

/// Test 2: Unbalanced transaction should be blocked.
/// Postings that don't sum to 0 violate double-entry rule.
#[test]
fn test_create_transaction_unbalanced_blocked() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let postings = [
        PostingInput {
            account_id: asset_id.clone(),
            amount: -1000,
        },
        PostingInput {
            account_id: expense_id.clone(),
            amount: 500, // Does NOT balance! Sum = -500
        },
    ];

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Incomplete transaction",
        None,
        &postings,
    );

    assert!(result.is_err(), "Unbalanced transaction should be rejected");

    // Verify error message is i18n key
    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert_eq!(msg, "errors.transactionUnbalanced");
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

/// Test 3: Transaction with fewer than 2 postings should be blocked.
#[test]
fn test_create_transaction_single_posting_blocked() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");

    let postings = [PostingInput {
        account_id: asset_id.clone(),
        amount: 1000,
    }];

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Invalid single posting",
        None,
        &postings,
    );

    assert!(
        result.is_err(),
        "Single posting transaction should be rejected"
    );

    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert_eq!(msg, "errors.transactionMinPostings");
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

/// Test 9: Transaction with category_id.
#[test]
fn test_create_transaction_with_category() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Create a category
    let cat_id = Uuid::new_v4().to_string();
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
            "INSERT INTO categories (id, name, type, created_at, updated_at) VALUES (?1, 'Groceries', 'expense', ?2, ?2)",
            rusqlite::params![&cat_id, &now],
        )
        .unwrap();

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Grocery shopping",
        Some(&cat_id),
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -500,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 500,
            },
        ],
    )
    .unwrap();

    let tx = get_transaction_with_postings(&conn, &tx_id)
        .unwrap()
        .expect("Transaction should exist");

    assert_eq!(tx.transaction.category_id, Some(cat_id));
}

/// Test 10: Zero-sum transaction (all postings sum to 0).
#[test]
fn test_create_transaction_zero_sum() {
    let (_dir, conn) = test_conn();
    let asset1_id = make_account(&conn, "Cash", "asset");
    let asset2_id = make_account(&conn, "Bank", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Three postings that sum to 0: -500 + 300 + 200 = 0
    let postings = [
        PostingInput {
            account_id: asset1_id.clone(),
            amount: -500,
        },
        PostingInput {
            account_id: asset2_id.clone(),
            amount: 300,
        },
        PostingInput {
            account_id: expense_id.clone(),
            amount: 200,
        },
    ];

    let tx_id = create_transaction(&conn, "2024-01-15", "Split payment", None, &postings).unwrap();

    let tx = get_transaction_with_postings(&conn, &tx_id)
        .unwrap()
        .expect("Transaction should exist");

    assert_eq!(tx.postings.len(), 3);
    let sum: i64 = tx.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

/// Test: Transaction with non-existent account_id should fail with ValidationError.
#[test]
fn test_create_transaction_invalid_account() {
    let (_dir, conn) = test_conn();

    // Create one valid account
    let asset_id = make_account(&conn, "Cash", "asset");

    // Use a non-existent account_id
    let fake_account_id = Uuid::new_v4().to_string();

    let postings = [
        PostingInput {
            account_id: asset_id.clone(),
            amount: -1000,
        },
        PostingInput {
            account_id: fake_account_id.clone(), // Invalid!
            amount: 1000,
        },
    ];

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Invalid account transaction",
        None,
        &postings,
    );

    assert!(
        result.is_err(),
        "Transaction with invalid account_id should be rejected"
    );

    match result.unwrap_err() {
        AppError::NotFound(msg) => {
            assert!(
                crate::errors::render(&msg).contains("账户不存在"),
                "Error should reference accountNotFound"
            );
        }
        other => panic!("Expected NotFound, got: {:?}", other),
    }
}

#[test]
fn test_create_transaction_duplicate_account() {
    let (_dir, conn) = test_conn();

    let asset_id = make_account(&conn, "Cash", "asset");

    let postings = [
        PostingInput {
            account_id: asset_id.clone(),
            amount: -1000,
        },
        PostingInput {
            account_id: asset_id.clone(),
            amount: 1000,
        },
    ];

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Duplicate account transaction",
        None,
        &postings,
    );

    assert!(
        result.is_err(),
        "Transaction with duplicate account_id should be rejected"
    );

    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert!(
                crate::errors::render(&msg).contains("同一账户不能在同一交易中出现多次"),
                "Error should reference duplicateAccount"
            );
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

#[test]
fn test_create_transaction_missing_postings_table() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");
    conn.execute_batch("DROP TABLE postings;").unwrap();

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Broken",
        None,
        &[
            PostingInput {
                account_id: cash,
                amount: -100,
            },
            PostingInput {
                account_id: food,
                amount: 100,
            },
        ],
    );
    assert!(result.is_err());
}
