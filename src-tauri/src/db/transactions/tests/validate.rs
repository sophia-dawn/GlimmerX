//! Tests for this area of the transaction module.

use super::*;

#[test]
fn test_validate_accounts_active_empty() {
    let (_dir, conn) = test_conn();
    assert!(validate_accounts_active(&conn, &[]).is_ok());
    assert!(validate_no_equity_accounts(&conn, &[]).is_ok());
}

#[test]
fn test_validate_accounts_active_inactive() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");
    conn.execute("UPDATE accounts SET is_active = 0 WHERE id = ?1", [&cash])
        .unwrap();

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Inactive",
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
    match result.unwrap_err() {
        AppError::ValidationError(msg) => assert_eq!(msg, "errors.account.inactive"),
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

#[test]
fn test_validate_no_equity_accounts() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let equity = make_account(&conn, "Owner", "equity");

    let result = create_transaction(
        &conn,
        "2024-01-15",
        "Equity",
        &[
            PostingInput {
                account_id: cash,
                amount: -100,
            },
            PostingInput {
                account_id: equity,
                amount: 100,
            },
        ],
    );
    match result.unwrap_err() {
        AppError::ValidationError(msg) => {
            assert_eq!(msg, "errors.transaction.equityAccountRestricted")
        }
        other => panic!("Expected ValidationError, got: {:?}", other),
    }

    // Missing account in the equity check
    let err = validate_no_equity_accounts(&conn, &["missing-account".to_string()]).unwrap_err();
    assert!(matches!(err, AppError::NotFound(_)));
}

#[test]
fn test_format_amount_display_branches() {
    assert_eq!(format_amount_display(3500), "+¥35.00");
    assert_eq!(format_amount_display(3550), "+¥35.50");
    assert_eq!(format_amount_display(-3500), "-¥35.00");
    assert_eq!(format_amount_display(-3550), "-¥35.50");
}

#[test]
fn test_validate_quick_add_input_errors() {
    let (_dir, conn) = test_conn();
    let cash = make_account(&conn, "Cash", "asset");
    let food = make_account(&conn, "Food", "expense");

    let base = |mode: &str| QuickAddInput {
        mode: mode.to_string(),
        amount: "10.00".to_string(),
        source_account_id: Some(cash.clone()),
        destination_account_id: Some(food.clone()),
        category_id: None,
        category_name: None,
        description: None,
        date: None,
    };

    // Invalid mode
    let mut input = base("bogus");
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("交易类型无效"));

    // Non-positive amount
    input = base("expense");
    input.amount = "0.00".to_string();
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("金额必须为正数"));

    // Expense without source
    input = base("expense");
    input.source_account_id = None;
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("支出需要一个来源账户。"));

    // Expense without destination or category
    input = base("expense");
    input.destination_account_id = None;
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("支出交易需要指定目标账户或分类"));

    // Income without destination or category
    input = base("income");
    input.destination_account_id = None;
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("收入需要一个目标账户。"));

    // Transfer missing one account
    input = base("transfer");
    input.destination_account_id = None;
    let err = quick_add_transaction(&conn, &input).unwrap_err();
    assert!(err.to_string().contains("转账需要来源和目标账户。"));
}

#[test]
fn test_parse_amount_to_cents() {
    assert_eq!(parse_amount_to_cents("12.50").unwrap(), 1250);
    assert_eq!(parse_amount_to_cents("100").unwrap(), 10000);
    assert_eq!(parse_amount_to_cents("-3.99").unwrap(), -399);
    assert_eq!(parse_amount_to_cents("0.005").unwrap(), 1);
    assert!(parse_amount_to_cents("abc").is_err());
    assert!(parse_amount_to_cents("").is_err());
}
