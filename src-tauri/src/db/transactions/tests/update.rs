//! Tests for this area of the transaction module.

use super::*;

/// Test 16: Update transaction description
#[test]
fn test_update_transaction_description() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Old description",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let input = UpdateTransactionInput {
        description: Some("New description".to_string()),
        ..Default::default()
    };

    let result = update_transaction(&mut conn, &tx_id, &input).unwrap();
    assert_eq!(result.description, "New description");
    assert_eq!(result.date, "2024-01-15");
    assert_eq!(result.postings.len(), 2);
}

/// Test 17: Update transaction with unbalanced postings fails
#[test]
fn test_update_transaction_unbalanced_fails() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let input = UpdateTransactionInput {
        postings: Some(vec![
            PostingInput {
                account_id: asset_id.clone(),
                amount: 5000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: -3000,
            },
        ]),
        ..Default::default()
    };

    let result = update_transaction(&mut conn, &tx_id, &input);
    assert!(result.is_err());
}

/// Test 18: Update transaction with empty input fails
#[test]
fn test_update_transaction_empty_input_fails() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let input = UpdateTransactionInput::default();

    let result = update_transaction(&mut conn, &tx_id, &input);
    assert!(result.is_err());
}

/// Test 19: Update transaction with __clear__ marker clears category_id to NULL
#[test]
fn test_update_transaction_clear_category() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");
    let category_id = make_category(&conn, "Groceries", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test with category",
        Some(&category_id),
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let tx_before = get_transaction_with_postings(&conn, &tx_id)
        .unwrap()
        .expect("Transaction should exist");
    assert_eq!(
        tx_before.transaction.category_id,
        Some(category_id.clone()),
        "Category should be set initially"
    );

    let input = UpdateTransactionInput {
        category_id: Some("__clear__".to_string()),
        ..Default::default()
    };

    let result = update_transaction(&mut conn, &tx_id, &input).unwrap();

    assert_eq!(
        result.category_id, None,
        "Category should be cleared with __clear__ marker"
    );

    let category_in_db: Option<String> = conn
        .query_row(
            "SELECT category_id FROM transactions WHERE id = ?1",
            [&tx_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        category_in_db, None,
        "Database should have NULL category_id after clearing"
    );
}

// ---------------------------------------------------------------------------
// Delete Transaction Tests
// ---------------------------------------------------------------------------

/// Test 19: Delete transaction succeeds
#[test]
fn test_delete_transaction() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "To delete",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    delete_transaction(&mut conn, &tx_id).unwrap();

    let exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
            [&tx_id],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!exists);

    let posting_count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
            [&tx_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(posting_count, 0);
}

/// Test 20: Delete non-existent transaction fails
#[test]
fn test_delete_transaction_not_found() {
    let (_dir, mut conn) = test_conn();

    let result = delete_transaction(&mut conn, "nonexistent-id");
    assert!(result.is_err());
}

/// Test 21: Get delete preview
#[test]
fn test_get_delete_preview() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Preview test",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let preview = get_delete_preview(&conn, &tx_id).unwrap();

    assert!(preview.can_delete);
    assert_eq!(preview.posting_count, 2);
    assert_eq!(preview.description, "Preview test");
}

#[test]
fn test_update_transaction_validation_errors() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");
    let mut conn = conn;

    // Missing transaction
    let err = update_transaction(
        &mut conn,
        "missing",
        &UpdateTransactionInput {
            date: Some("2024-01-01".to_string()),
            description: None,
            category_id: None,
            postings: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test",
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

    // Too few postings
    let err = update_transaction(
        &mut conn,
        &tx_id,
        &UpdateTransactionInput {
            date: None,
            description: None,
            category_id: None,
            postings: Some(vec![PostingInput {
                account_id: cash.clone(),
                amount: 100,
            }]),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("交易至少需要两个分录"));

    // Duplicate accounts in postings
    let err = update_transaction(
        &mut conn,
        &tx_id,
        &UpdateTransactionInput {
            date: None,
            description: None,
            category_id: None,
            postings: Some(vec![
                PostingInput {
                    account_id: cash.clone(),
                    amount: -100,
                },
                PostingInput {
                    account_id: cash.clone(),
                    amount: 100,
                },
            ]),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("同一账户不能在同一交易中出现多次"));

    // Update category
    let cat_id = make_category(&conn, "Dining", "expense");
    let detail = update_transaction(
        &mut conn,
        &tx_id,
        &UpdateTransactionInput {
            date: Some("2024-01-16".to_string()),
            description: Some("Updated".to_string()),
            category_id: Some(cat_id),
            postings: None,
        },
    )
    .unwrap();
    assert_eq!(detail.description, "Updated");
    assert_eq!(detail.date, "2024-01-16");
}
