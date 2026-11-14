//! Tests for this area of the transaction module.

use super::*;

/// Test 11: QuickAdd expense with category should succeed
#[test]
fn test_quick_add_expense_with_category() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        category_id: None,
        category_name: None,
        source_account_id: Some(asset_id.clone()),
        destination_account_id: Some(expense_id.clone()),
        description: Some("Lunch".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(result.is_ok());

    let tx = result.unwrap();
    assert_eq!(tx.postings.len(), 2);

    let sum: i64 = tx.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

/// Test 12: QuickAdd transfer to same account should fail
#[test]
fn test_quick_add_transfer_same_account_fails() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");

    let input = QuickAddInput {
        mode: "transfer".to_string(),
        amount: "100.00".to_string(),
        source_account_id: Some(asset_id.clone()),
        destination_account_id: Some(asset_id.clone()),
        category_id: None,
        category_name: None,
        description: Some("Self transfer".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(result.is_err());
}

/// Test 13: QuickAdd negative amount should fail
#[test]
fn test_quick_add_negative_amount_fails() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");
    let expense_id = make_account(&conn, "Food", "expense");

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "-35.00".to_string(),
        source_account_id: Some(asset_id.clone()),
        destination_account_id: Some(expense_id.clone()),
        category_id: None,
        category_name: None,
        description: Some("Lunch".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(result.is_err());
}

/// Test: QuickAdd expense without source account should fail (RED test for bug fix)
/// This tests the validation fix - expense MUST have source_account_id
#[test]
fn test_quick_add_expense_without_source_fails() {
    let (_dir, conn) = test_conn();
    let expense_id = make_account(&conn, "Food", "expense");
    let category_id = make_category(&conn, "Groceries", "expense");

    // Expense with destination account and category, but NO source account
    // This should FAIL because source_account_id is required for expenses
    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        source_account_id: None, // MISSING - this is the bug being tested
        destination_account_id: Some(expense_id.clone()),
        category_id: Some(category_id.clone()),
        category_name: None,
        description: Some("Lunch".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(
        result.is_err(),
        "Expense without source account should be rejected"
    );

    // Verify error message is correct i18n key
    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert_eq!(
                msg, "errors.transaction.expenseRequiresSource",
                "Should return expenseRequiresSource error"
            );
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

/// Test: QuickAdd expense without destination or category should fail (RED test)
/// This tests the validation fix - expense MUST have either destination_account_id OR category_id
#[test]
fn test_quick_add_expense_without_destination_or_category_fails() {
    let (_dir, conn) = test_conn();
    let asset_id = make_account(&conn, "Cash", "asset");

    // Expense with source account, but NO destination or category
    // This should FAIL because expense needs a destination
    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        source_account_id: Some(asset_id.clone()), // Source provided
        destination_account_id: None,              // MISSING destination
        category_id: None,                         // MISSING category
        category_name: None,
        description: Some("Lunch".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(
        result.is_err(),
        "Expense without destination or category should be rejected"
    );

    // Verify error message is correct i18n key
    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert_eq!(
                msg, "errors.transaction.expenseRequiresDestinationOrCategory",
                "Should return expenseRequiresDestinationOrCategory error"
            );
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Transaction Detail Tests
// ---------------------------------------------------------------------------

#[test]
fn test_quick_add_variants() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let bank = make_account(&conn, "Bank", "asset");
    let food = make_account(&conn, "Food", "expense");
    let salary_cat = make_category(&conn, "Salary", "income");

    // Expense with explicit destination
    let tx = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "expense".to_string(),
            amount: "35.00".to_string(),
            source_account_id: Some(cash.clone()),
            destination_account_id: Some(food.clone()),
            category_id: None,
            category_name: None,
            description: Some("Lunch".to_string()),
            date: Some("2024-01-15".to_string()),
        },
    )
    .unwrap();
    assert_eq!(tx.postings.len(), 2);

    // Income with explicit source
    let tx = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "income".to_string(),
            amount: "100.00".to_string(),
            source_account_id: Some(food.clone()),
            destination_account_id: Some(cash.clone()),
            category_id: None,
            category_name: None,
            description: None,
            date: None,
        },
    )
    .unwrap();
    assert_eq!(tx.postings.len(), 2);

    // Income with category -> category account auto-created
    let tx = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "income".to_string(),
            amount: "50.00".to_string(),
            source_account_id: None,
            destination_account_id: Some(cash.clone()),
            category_id: Some(salary_cat.clone()),
            category_name: None,
            description: None,
            date: None,
        },
    )
    .unwrap();
    assert_eq!(tx.postings.len(), 2);

    // Transfer between two balance-sheet accounts
    let tx = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "transfer".to_string(),
            amount: "20.00".to_string(),
            source_account_id: Some(cash.clone()),
            destination_account_id: Some(bank.clone()),
            category_id: None,
            category_name: None,
            description: None,
            date: None,
        },
    )
    .unwrap();
    assert_eq!(tx.postings.len(), 2);

    // Transfer with a P&L endpoint is rejected (design doc 25, P0-2)
    let err = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "transfer".to_string(),
            amount: "20.00".to_string(),
            source_account_id: Some(cash.clone()),
            destination_account_id: Some(food.clone()),
            category_id: None,
            category_name: None,
            description: None,
            date: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("不允许转账的账户类型"));

    // Transfer to self blocked
    let err = quick_add_transaction(
        &conn,
        &QuickAddInput {
            mode: "transfer".to_string(),
            amount: "20.00".to_string(),
            source_account_id: Some(cash.clone()),
            destination_account_id: Some(cash.clone()),
            category_id: None,
            category_name: None,
            description: None,
            date: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("不能转账到同一账户"));
}

// ---------------------------------------------------------------------------
// Design doc 25, P0 regression tests
// ---------------------------------------------------------------------------

/// P0-2: a transfer whose endpoint is an expense account must be rejected.
/// The old quick-add transfer branch had no endpoint validation, so money
/// could be "transferred" out of a P&L category account.
#[test]
fn test_quick_add_transfer_expense_endpoint_rejected() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");

    let input = QuickAddInput {
        mode: "transfer".to_string(),
        amount: "20.00".to_string(),
        source_account_id: Some(cash.clone()),
        destination_account_id: Some(food.clone()),
        category_id: None,
        category_name: None,
        description: None,
        date: None,
    };

    let err = quick_add_transaction(&conn, &input).unwrap_err();
    match err {
        AppError::ValidationError(msg) => {
            assert_eq!(msg, "errors.account.transferAccountInvalid")
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
    // Nothing was written.
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM transactions"), 0);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM postings"), 0);
}

/// P0-2: the other direction (expense account as source) is rejected too.
#[test]
fn test_quick_add_transfer_expense_endpoint_as_source_rejected() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");

    let input = QuickAddInput {
        mode: "transfer".to_string(),
        amount: "20.00".to_string(),
        source_account_id: Some(food.clone()),
        destination_account_id: Some(cash.clone()),
        category_id: None,
        category_name: None,
        description: None,
        date: None,
    };

    assert!(quick_add_transaction(&conn, &input).is_err());
}

/// P0-3: `category_name` resolves (or creates) the category and its mirror
/// account, both inside the same transaction as the posting writes.
#[test]
fn test_quick_add_category_name_creates_category_and_mirror_account() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        source_account_id: Some(cash.clone()),
        destination_account_id: None,
        category_id: None,
        category_name: Some("餐饮美食".to_string()),
        description: Some("Lunch".to_string()),
        date: Some("2026-11-14".to_string()),
    };

    let tx = quick_add_transaction(&conn, &input).unwrap();
    assert_eq!(tx.postings.len(), 2);

    // The category was created and typed by the mode (expense).
    let cat = crate::db::categories::find_by_type_and_name(
        &conn,
        &crate::db::categories::CategoryType::Expense,
        "餐饮美食",
    )
    .unwrap()
    .expect("category should be auto-created");

    // The mirror P&L account exists and one posting landed on it.
    let mirror_id = crate::db::categories::account_id_for_category(&conn, &cat.id).unwrap();
    assert!(tx.postings.iter().any(|p| p.account_id == mirror_id));

    // Balanced double entry.
    let sum: i64 = tx.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

/// P0-3: `category_name` on a transfer is rejected — transfers have no P&L
/// category, and silently ignoring the field would hide the caller's mistake.
#[test]
fn test_quick_add_category_name_on_transfer_rejected() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let bank = make_account(&conn, "Bank", "asset");

    let input = QuickAddInput {
        mode: "transfer".to_string(),
        amount: "20.00".to_string(),
        source_account_id: Some(cash.clone()),
        destination_account_id: Some(bank.clone()),
        category_id: None,
        category_name: Some("餐饮美食".to_string()),
        description: None,
        date: None,
    };

    let err = quick_add_transaction(&conn, &input).unwrap_err();
    match err {
        AppError::ValidationError(msg) => {
            assert_eq!(msg, "errors.transaction.transferHasNoCategory")
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
    // The category must not have been created on the rejected path.
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM categories"), 0);
}

/// P0-3: the explicit `category` argument still wins over `category_name`
#[test]
fn test_quick_add_category_id_takes_priority() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    // An existing category the caller resolved (and type-checked) itself.
    let cat_id = crate::db::categories::create_category(
        &conn,
        "显式分类",
        &crate::db::categories::CategoryType::Expense,
        None,
    )
    .unwrap();

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        source_account_id: Some(cash.clone()),
        destination_account_id: None,
        category_id: Some(cat_id),
        category_name: Some("should-be-ignored".to_string()),
        description: None,
        date: None,
    };

    quick_add_transaction(&conn, &input).unwrap();

    // Only the pre-existing category remains; the name was not resolved.
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM categories"), 1);
    assert!(crate::db::categories::find_by_type_and_name(
        &conn,
        &crate::db::categories::CategoryType::Expense,
        "显式分类"
    )
    .unwrap()
    .is_some());
}

/// A failed posting insert must not leave an unbalanced orphan transaction.
#[test]
fn test_quick_add_rolls_back_on_posting_failure() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");
    inject_second_posting_failure(&conn);

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        category_id: None,
        category_name: None,
        source_account_id: Some(cash),
        destination_account_id: Some(food),
        description: Some("Lunch".to_string()),
        date: None,
    };

    assert!(quick_add_transaction(&conn, &input).is_err());
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM transactions"), 0);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM postings"), 0);
}

/// AI path: category auto-creation and derived account creation must
/// also roll back when a posting insert fails.
#[test]
fn test_ai_quick_add_rolls_back_category_and_account() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    inject_second_posting_failure(&conn);

    let input = QuickAddInput {
        mode: "expense".to_string(),
        amount: "35.00".to_string(),
        category_id: None,
        category_name: Some("新分类".to_string()),
        source_account_id: Some(cash),
        destination_account_id: None,
        description: Some("Lunch".to_string()),
        date: None,
    };

    let result = quick_add_transaction(&conn, &input);
    assert!(result.is_err());
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM transactions"), 0);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM postings"), 0);
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM categories"), 0);
    // Only the pre-existing asset account remains; the derived expense
    // account created inside the transaction must be gone too.
    assert_eq!(count_rows(&conn, "SELECT count(*) FROM accounts"), 1);
}
