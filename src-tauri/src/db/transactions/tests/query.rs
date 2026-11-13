//! Tests for this area of the transaction module.

use super::*;

/// Test 4: List transactions should return all non-deleted transactions.
#[test]
fn test_list_transactions() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Create two transactions
    let _tx1 = create_transaction(
        &conn,
        "2024-01-10",
        "Transaction 1",
        None,
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

    let _tx2 = create_transaction(
        &conn,
        "2024-01-20",
        "Transaction 2",
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

    let transactions = list_transactions(&conn, None, None, None).unwrap();
    assert_eq!(transactions.len(), 2);

    // Verify ordering (by date descending, then id descending)
    assert_eq!(transactions[0].transaction.date, "2024-01-20");
    assert_eq!(transactions[1].transaction.date, "2024-01-10");
}

/// Test 5: Get transaction by ID should return transaction with postings.
#[test]
fn test_get_transaction_by_id() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Test transaction",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -2000,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 2000,
            },
        ],
    )
    .unwrap();

    let result = get_transaction_with_postings(&conn, &tx_id).unwrap();
    assert!(result.is_some());

    let tx = result.unwrap();
    assert_eq!(tx.transaction.id, tx_id);
    assert_eq!(tx.transaction.description, "Test transaction");
    assert_eq!(tx.postings.len(), 2);
}

/// Test 6: Get non-existent transaction should return None.
#[test]
fn test_get_transaction_nonexistent() {
    let (_dir, conn) = test_conn();

    let result = get_transaction_with_postings(&conn, "nonexistent-id").unwrap();
    assert!(result.is_none());
}

/// Test 7: List transactions filtered by account_id.
#[test]
fn test_list_transactions_by_account() {
    let (_dir, conn) = test_conn();
    let cash_id = make_account(&conn, "Cash", "asset");
    let bank_id = make_account(&conn, "Bank", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Transaction 1 involves Cash
    let _tx1 = create_transaction(
        &conn,
        "2024-01-10",
        "Cash expense",
        None,
        &[
            PostingInput {
                account_id: cash_id.clone(),
                amount: -100,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 100,
            },
        ],
    )
    .unwrap();

    // Transaction 2 involves Bank
    let _tx2 = create_transaction(
        &conn,
        "2024-01-20",
        "Bank expense",
        None,
        &[
            PostingInput {
                account_id: bank_id.clone(),
                amount: -200,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 200,
            },
        ],
    )
    .unwrap();

    // Filter by Cash account
    let cash_txs = list_transactions(&conn, Some(&cash_id), None, None).unwrap();
    assert_eq!(cash_txs.len(), 1);
    assert_eq!(cash_txs[0].transaction.description, "Cash expense");

    // Filter by Bank account
    let bank_txs = list_transactions(&conn, Some(&bank_id), None, None).unwrap();
    assert_eq!(bank_txs.len(), 1);
    assert_eq!(bank_txs[0].transaction.description, "Bank expense");
}

/// Test 8: List transactions filtered by date range.
#[test]
fn test_list_transactions_by_date_range() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let _tx1 = create_transaction(
        &conn,
        "2024-01-05",
        "Early transaction",
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

    let _tx2 = create_transaction(
        &conn,
        "2024-01-15",
        "Middle transaction",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -200,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 200,
            },
        ],
    )
    .unwrap();

    let _tx3 = create_transaction(
        &conn,
        "2024-01-25",
        "Late transaction",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -300,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 300,
            },
        ],
    )
    .unwrap();

    // Filter by date range (2024-01-10 to 2024-01-20)
    let filtered = list_transactions(&conn, None, Some("2024-01-10"), Some("2024-01-20")).unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].transaction.description, "Middle transaction");

    // Filter by from_date only
    let from_filtered = list_transactions(&conn, None, Some("2024-01-15"), None).unwrap();
    assert_eq!(from_filtered.len(), 2);

    // Filter by to_date only
    let to_filtered = list_transactions(&conn, None, None, Some("2024-01-15")).unwrap();
    assert_eq!(to_filtered.len(), 2);
}

#[test]
fn test_list_transactions_many_with_postings() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    // Create 50 transactions
    for i in 0..50 {
        let _tx_id = create_transaction(
            &conn,
            &format!("2024-01-{:02}", (i % 28) + 1),
            &format!("Transaction {}", i),
            None,
            &[
                PostingInput {
                    account_id: asset_id.clone(),
                    amount: -100 * (i + 1) as i64,
                },
                PostingInput {
                    account_id: expense_id.clone(),
                    amount: 100 * (i + 1) as i64,
                },
            ],
        )
        .unwrap();
    }

    let transactions = list_transactions(&conn, None, None, None).unwrap();

    // Verify all 50 transactions loaded
    assert_eq!(transactions.len(), 50);

    // Verify each transaction has 2 postings
    for tx in &transactions {
        assert_eq!(tx.postings.len(), 2);
        // Verify postings are correctly associated
        assert!(tx
            .postings
            .iter()
            .all(|p| p.transaction_id == tx.transaction.id));
    }
}

/// Test 14: Get transaction detail with all posting information
#[test]
fn test_get_transaction_detail() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let tx_id = create_transaction(
        &conn,
        "2024-01-15",
        "Grocery shopping",
        None,
        &[
            PostingInput {
                account_id: asset_id.clone(),
                amount: -3500,
            },
            PostingInput {
                account_id: expense_id.clone(),
                amount: 3500,
            },
        ],
    )
    .unwrap();

    let detail = get_transaction_detail(&conn, &tx_id).unwrap();

    assert_eq!(detail.id, tx_id);
    assert_eq!(detail.description, "Grocery shopping");
    assert_eq!(detail.postings.len(), 2);
    assert!(detail.is_balanced);
    assert_eq!(detail.debit_total, 3500);
    assert_eq!(detail.credit_total, -3500);

    // Postings sorted by amount ASC: -3500 first, +3500 second
    assert!(!detail.postings[0].is_debit);
    assert!(detail.postings[1].is_debit);
}

/// Test 15: Get transaction detail for non-existent transaction
#[test]
fn test_get_transaction_detail_not_found() {
    let (_dir, conn) = test_conn();

    let result = get_transaction_detail(&conn, "nonexistent-id");
    assert!(result.is_err());

    match result.unwrap_err() {
        AppError::NotFound(msg) => {
            assert_eq!(msg, "errors.transaction.notFound");
        }
        other => panic!("Expected NotFound error, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Update Transaction Tests
// ---------------------------------------------------------------------------

#[test]
fn test_get_transaction_detail_query_error() {
    let dir = TempDir::new().unwrap();
    let conn = Connection::open(dir.path().join("bare.db")).unwrap();
    let result = get_transaction_detail(&conn, "tx-1");
    assert!(result.is_err());
}
