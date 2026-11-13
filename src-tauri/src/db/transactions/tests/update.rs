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

/// Test 19: replacing the postings carries the category with them.
#[test]
fn test_update_postings_moves_the_category() {
    let (_dir, mut conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let food_cat = make_category(&conn, "Food", "expense");
    let travel_cat = make_category(&conn, "Travel", "expense");
    let food_acc = crate::db::categories::account_id_for_category(&conn, &food_cat).unwrap();
    let travel_acc = crate::db::categories::account_id_for_category(&conn, &travel_cat).unwrap();

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test with category",
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: food_acc,
                amount: 1000,
            },
        ],
    )
    .unwrap();

    let input = UpdateTransactionInput {
        postings: Some(vec![
            PostingInput {
                account_id: asset_id.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: travel_acc,
                amount: 1000,
            },
        ]),
        ..Default::default()
    };

    let result = update_transaction(&mut conn, &tx_id, &input).unwrap();

    let categorised: Vec<_> = result
        .postings
        .iter()
        .filter(|p| p.category_id.is_some())
        .collect();
    assert_eq!(categorised.len(), 1, "only the P&L side has a category");
    assert_eq!(
        categorised[0].category_id.as_deref(),
        Some(travel_cat.as_str())
    );
    assert_eq!(categorised[0].category_name.as_deref(), Some("Travel"));
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
            postings: None,
        },
    )
    .unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test",
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

    let detail = update_transaction(
        &mut conn,
        &tx_id,
        &UpdateTransactionInput {
            date: Some("2024-01-16".to_string()),
            description: Some("Updated".to_string()),
            postings: None,
        },
    )
    .unwrap();
    assert_eq!(detail.description, "Updated");
    assert_eq!(detail.date, "2024-01-16");
}
