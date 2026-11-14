//! Tests for the ledger capability layer (design doc 25, P1).
//!
//! Fixtures go through the real APIs — `init_schema` for the database,
//! `create_account_with_path` for accounts — never hand-written DDL.

use rusqlite::Connection;
use tempfile::TempDir;

use super::*;
use crate::db::{accounts, categories};

fn test_conn() -> (TempDir, Connection) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    crate::db::init_schema(&conn).unwrap();
    (dir, conn)
}

fn asset(conn: &Connection, path: &str) -> String {
    accounts::create_account_with_path(conn, path, None).unwrap()
}

// ---------------------------------------------------------------------------
// resolve.rs — the four name rules, plus the id path
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_exact_match_case_insensitive() {
    let (_dir, conn) = test_conn();
    let id = asset(&conn, "Assets/Cash");
    let accounts = accounts::list_accounts(&conn).unwrap();

    let (resolved, warning) = resolve_account(&accounts, &AccountRef::Name("cash".into())).unwrap();
    assert_eq!(resolved, id);
    assert!(warning.is_none());
}

#[test]
fn test_resolve_unique_contains_match_warns() {
    let (_dir, conn) = test_conn();
    let wechat = asset(&conn, "Assets/WeChat Wallet");
    let accounts = accounts::list_accounts(&conn).unwrap();

    let (resolved, warning) =
        resolve_account(&accounts, &AccountRef::Name("wechat".into())).unwrap();
    assert_eq!(resolved, wechat);
    let warning = warning.expect("fuzzy match must warn");
    assert!(warning.contains("按名称匹配到"));
    assert!(warning.contains("WeChat Wallet"));
}

#[test]
fn test_resolve_ambiguous_contains_errors() {
    let (_dir, conn) = test_conn();
    asset(&conn, "Assets/WeChat Wallet");
    asset(&conn, "Assets/WeChat Pay");
    let accounts = accounts::list_accounts(&conn).unwrap();

    let err = resolve_account(&accounts, &AccountRef::Name("wechat".into())).unwrap_err();
    assert!(err.to_string().contains("匹配到多个账户"));
}

#[test]
fn test_resolve_no_match_errors() {
    let (_dir, conn) = test_conn();
    asset(&conn, "Assets/Cash");
    let accounts = accounts::list_accounts(&conn).unwrap();

    let err = resolve_account(&accounts, &AccountRef::Name("支付宝".into())).unwrap_err();
    assert!(err.to_string().contains("找不到该名称的账户"));
}

#[test]
fn test_resolve_ignores_inactive_accounts() {
    let (_dir, conn) = test_conn();
    let id = asset(&conn, "Assets/Old");
    accounts::update_account(
        &conn,
        &accounts::UpdateAccountParams {
            id: id.as_str(),
            name: None,
            description: None,
            account_number: None,
            iban: None,
            is_active: Some(false),
            include_net_worth: None,
        },
    )
    .unwrap();
    let accounts = accounts::list_accounts(&conn).unwrap();

    // Exact name of an inactive account is not a hit.
    assert!(resolve_account(&accounts, &AccountRef::Name("Old".into())).is_err());
    // Id of an inactive account is an error too.
    assert!(resolve_account(&accounts, &AccountRef::Id(id)).is_err());
}

#[test]
fn test_resolve_by_id() {
    let (_dir, conn) = test_conn();
    let id = asset(&conn, "Assets/Cash");
    let accounts = accounts::list_accounts(&conn).unwrap();

    let (resolved, warning) = resolve_account(&accounts, &AccountRef::Id(id.clone())).unwrap();
    assert_eq!(resolved, id);
    assert!(warning.is_none());

    let err = resolve_account(&accounts, &AccountRef::Id("no-such-id".into())).unwrap_err();
    assert!(err.to_string().contains("账户不存在"));
}

#[test]
fn test_resolve_empty_name_errors() {
    let (_dir, conn) = test_conn();
    let accounts = accounts::list_accounts(&conn).unwrap();
    assert!(resolve_account(&accounts, &AccountRef::Name("   ".into())).is_err());
}

// ---------------------------------------------------------------------------
// read.rs
// ---------------------------------------------------------------------------

#[test]
fn test_list_accounts_includes_balances() {
    let (_dir, conn) = test_conn();
    asset(&conn, "Assets/Cash");

    let views = list_accounts(&conn).unwrap();
    let cash = views.iter().find(|a| a.name == "Cash").unwrap();
    assert_eq!(cash.account_type, "asset");
    assert!(cash.is_active);
    assert_eq!(cash.balance_cents, 0);
}

#[test]
fn test_list_categories_returns_views() {
    let (_dir, conn) = test_conn();
    let id = asset(&conn, "Assets/Cash");

    let outcome = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 3500,
            date: Some("2026-11-14".into()),
            description: "Lunch".into(),
            source_account: Some(AccountRef::Id(id)),
            destination_account: None,
            category: Some(CategoryRef::Name("测试餐饮".into())),
        },
    )
    .unwrap();

    let cats = list_categories(&conn).unwrap();
    let cat = cats.iter().find(|c| c.name == "测试餐饮").unwrap();
    assert_eq!(cat.category_type, categories::CategoryType::Expense);

    // The transaction itself is visible through the read path.
    let items = query_transactions(&conn, &LedgerQuery::default()).unwrap();
    assert!(items.iter().any(|t| t.id == outcome.transaction_id));
}

#[test]
fn test_account_balances_reflect_writes() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");
    let food = asset(&conn, "Assets/Extra");

    record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Transfer,
            amount_cents: 5000,
            date: None,
            description: "transfer".into(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: Some(AccountRef::Id(food.clone())),
            category: None,
        },
    )
    .unwrap();

    let balances = account_balances(&conn).unwrap();
    let extra = balances.iter().find(|b| b.account_id == food).unwrap();
    assert_eq!(extra.balance_cents, 5000);
    let cash_balance = balances.iter().find(|b| b.account_name == "Cash").unwrap();
    assert_eq!(cash_balance.balance_cents, -5000);
}

#[test]
fn test_query_transactions_filters_by_description_and_account() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");
    let other = asset(&conn, "Assets/Other");

    record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 3500,
            date: Some("2026-11-14".into()),
            description: "coffee".into(),
            source_account: Some(AccountRef::Id(cash.clone())),
            destination_account: None,
            category: Some(CategoryRef::Name("咖啡".into())),
        },
    )
    .unwrap();
    record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 1200,
            date: Some("2026-11-14".into()),
            description: "tea".into(),
            source_account: Some(AccountRef::Id(other)),
            destination_account: None,
            category: Some(CategoryRef::Name("茶饮".into())),
        },
    )
    .unwrap();

    let by_description = query_transactions(
        &conn,
        &LedgerQuery {
            description_query: Some("coffee".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(by_description.len(), 1);
    assert_eq!(by_description[0].description, "coffee");

    let by_account = query_transactions(
        &conn,
        &LedgerQuery {
            account_id: Some(cash),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(by_account.len(), 1);
    assert_eq!(by_account[0].description, "coffee");
}

#[test]
fn test_query_transactions_rejects_bad_date() {
    let (_dir, conn) = test_conn();
    let err = query_transactions(
        &conn,
        &LedgerQuery {
            from_date: Some("2026/11/14".into()),
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("日期格式无效"));
}

#[test]
fn test_period_summary_reuses_report_totals() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");

    record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 3500,
            date: Some("2026-11-14".into()),
            description: "Lunch".into(),
            source_account: Some(AccountRef::Id(cash.clone())),
            destination_account: None,
            category: Some(CategoryRef::Name("餐饮".into())),
        },
    )
    .unwrap();
    record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Income,
            amount_cents: 10000,
            date: Some("2026-11-15".into()),
            description: "Salary".into(),
            source_account: None,
            destination_account: Some(AccountRef::Id(cash)),
            category: Some(CategoryRef::Name("测试工资".into())),
        },
    )
    .unwrap();

    let summary = period_summary(&conn, "2026-11-01", "2026-11-30").unwrap();
    assert_eq!(summary.income_cents, 10000);
    assert_eq!(summary.expense_cents, 3500);
    assert_eq!(summary.net_cents, 6500);

    // A range outside the records is all zeros.
    let empty = period_summary(&conn, "2026-01-01", "2026-01-31").unwrap();
    assert_eq!(empty.income_cents, 0);
    assert_eq!(empty.expense_cents, 0);
}

#[test]
fn test_period_summary_rejects_bad_date() {
    let (_dir, conn) = test_conn();
    assert!(period_summary(&conn, "2026-11", "2026-11-30").is_err());
}

// ---------------------------------------------------------------------------
// write.rs — record_transaction
// ---------------------------------------------------------------------------

#[test]
fn test_record_expense_with_new_category_warns_and_balances() {
    let (_dir, conn) = test_conn();
    asset(&conn, "Assets/WeChat Wallet");

    let outcome = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 1850,
            date: Some("2026-11-14".into()),
            description: "中午吃饭".into(),
            // Fuzzy account name -> warning, never a silent default.
            source_account: Some(AccountRef::Name("wechat".into())),
            destination_account: None,
            category: Some(CategoryRef::Name("餐饮美食".into())),
        },
    )
    .unwrap();

    assert_eq!(outcome.date, "2026-11-14");
    assert_eq!(outcome.description, "中午吃饭");
    assert_eq!(outcome.postings.len(), 2);

    let sum: i64 = outcome.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);

    // Both warnings surfaced: the fuzzy account match and the new category.
    assert_eq!(outcome.warnings.len(), 2);
    assert!(outcome.warnings.iter().any(|w| w.contains("按名称匹配到")));
    assert!(outcome.warnings.iter().any(|w| w.contains("餐饮美食")));

    // Posting views carry account identity.
    let credit = outcome
        .postings
        .iter()
        .find(|p| p.amount < 0)
        .expect("credit posting");
    assert_eq!(credit.account_name, "WeChat Wallet");
    assert_eq!(credit.account_type, "asset");
}

#[test]
fn test_record_income_by_account_id_with_existing_category() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");

    // An existing income category, resolved by id below.
    let cat_id = crate::db::categories::create_category(
        &conn,
        "工资",
        &categories::CategoryType::Income,
        None,
    )
    .unwrap();

    let outcome = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Income,
            amount_cents: 10000,
            date: Some("2026-11-14".into()),
            description: "Salary".into(),
            source_account: None,
            destination_account: Some(AccountRef::Id(cash)),
            category: Some(CategoryRef::Id(cat_id)),
        },
    )
    .unwrap();

    assert!(outcome.warnings.is_empty());
    let sum: i64 = outcome.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

#[test]
fn test_record_transfer_by_names() {
    let (_dir, conn) = test_conn();
    asset(&conn, "Assets/Alipay");
    asset(&conn, "Assets/WeChat");

    let outcome = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Transfer,
            amount_cents: 50000,
            date: Some("2026-11-14".into()),
            description: "搬家".into(),
            source_account: Some(AccountRef::Name("Alipay".into())),
            destination_account: Some(AccountRef::Name("WeChat".into())),
            category: None,
        },
    )
    .unwrap();

    assert!(outcome.warnings.is_empty());
    assert_eq!(outcome.postings.len(), 2);
    let sum: i64 = outcome.postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);
}

#[test]
fn test_record_transfer_rejects_expense_endpoint() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");
    let cat_id = crate::db::categories::create_category(
        &conn,
        "餐饮美食",
        &categories::CategoryType::Expense,
        None,
    )
    .unwrap();

    // The category's mirror account is an expense account; using it as a
    // transfer endpoint must be rejected (P0-2 invariant, surfaced here).
    let mirror = categories::account_id_for_category(&conn, &cat_id).unwrap();

    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Transfer,
            amount_cents: 5000,
            date: None,
            description: "bad".into(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: Some(AccountRef::Id(mirror)),
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("不允许转账的账户类型"));
}

#[test]
fn test_record_transfer_rejects_category() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");
    let bank = asset(&conn, "Assets/Bank");

    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Transfer,
            amount_cents: 5000,
            date: None,
            description: "bad".into(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: Some(AccountRef::Id(bank)),
            category: Some(CategoryRef::Name("餐饮美食".into())),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("转账不使用分类"));
}

#[test]
fn test_record_rejects_invalid_requests() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");

    // Non-positive amount.
    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 0,
            date: None,
            description: String::new(),
            source_account: Some(AccountRef::Id(cash.clone())),
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("金额必须为正数"));

    // Missing source for expense.
    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 100,
            date: None,
            description: String::new(),
            source_account: None,
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("支出需要一个来源账户"));

    // Bad date.
    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 100,
            date: Some("14-11-2026".into()),
            description: String::new(),
            source_account: Some(AccountRef::Id(cash.clone())),
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("日期格式无效"));

    // Unknown account name must error, never fall back.
    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 100,
            date: None,
            description: String::new(),
            source_account: Some(AccountRef::Name("不存在".into())),
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("找不到该名称的账户"));

    // Nothing was written by any rejected request.
    let txs = query_transactions(&conn, &LedgerQuery::default()).unwrap();
    assert!(txs.is_empty());
}

#[test]
fn test_record_category_type_mismatch_rejected() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");
    let cat_id = crate::db::categories::create_category(
        &conn,
        "工资",
        &categories::CategoryType::Income,
        None,
    )
    .unwrap();

    // Income category on an expense request.
    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 100,
            date: None,
            description: String::new(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: None,
            category: Some(CategoryRef::Id(cat_id)),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("分类类型与交易类型不匹配"));
}

// ---------------------------------------------------------------------------
// write.rs — create_account
// ---------------------------------------------------------------------------

#[test]
fn test_create_account_with_opening_balance() {
    let (_dir, conn) = test_conn();

    let outcome = create_account(
        &conn,
        &CreateAccountRequest {
            path: "Assets/支付宝".into(),
            description: Some("日常".into()),
            opening_balance_cents: Some(123450),
            opening_balance_date: Some("2026-11-01".into()),
        },
    )
    .unwrap();

    assert_eq!(outcome.account_path, "Assets/支付宝");
    assert_eq!(outcome.account_type, "asset");
    let opening_tx = outcome.opening_transaction_id.expect("opening entry");
    assert!(!opening_tx.is_empty());
    assert_eq!(outcome.opening_postings.len(), 2);
    let sum: i64 = outcome.opening_postings.iter().map(|p| p.amount).sum();
    assert_eq!(sum, 0);

    // The account exists with the right balance through the read path.
    let views = list_accounts(&conn).unwrap();
    let acc = views.iter().find(|a| a.name == "支付宝").unwrap();
    assert_eq!(acc.balance_cents, 123450);
}

#[test]
fn test_create_account_without_opening_balance() {
    let (_dir, conn) = test_conn();

    let outcome = create_account(
        &conn,
        &CreateAccountRequest {
            path: "Liabilities/信用卡".into(),
            description: None,
            opening_balance_cents: None,
            opening_balance_date: None,
        },
    )
    .unwrap();

    assert_eq!(outcome.account_type, "liability");
    assert!(outcome.opening_transaction_id.is_none());
    assert!(outcome.opening_postings.is_empty());
}

#[test]
fn test_create_account_rejects_duplicate_name() {
    let (_dir, conn) = test_conn();
    create_account(
        &conn,
        &CreateAccountRequest {
            path: "Assets/支付宝".into(),
            description: None,
            opening_balance_cents: None,
            opening_balance_date: None,
        },
    )
    .unwrap();

    let err = create_account(
        &conn,
        &CreateAccountRequest {
            path: "Assets/支付宝".into(),
            description: None,
            opening_balance_cents: None,
            opening_balance_date: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("已存在"));
}

#[test]
fn test_create_account_rejects_bad_date() {
    let (_dir, conn) = test_conn();
    let err = create_account(
        &conn,
        &CreateAccountRequest {
            path: "Assets/Cash".into(),
            description: None,
            opening_balance_cents: Some(100),
            opening_balance_date: Some("2026/11/01".into()),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("日期格式无效"));
}

#[test]
fn test_record_rejects_overflowing_amount() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");

    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: crate::utils::money::MAX_AMOUNT_CENTS + 1,
            date: None,
            description: String::new(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("金额格式无效"));
}

#[test]
fn test_record_income_requires_destination() {
    let (_dir, conn) = test_conn();

    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Income,
            amount_cents: 100,
            date: None,
            description: String::new(),
            source_account: None,
            destination_account: None,
            category: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("收入需要一个目标账户"));
}

#[test]
fn test_record_category_by_unknown_id_rejected() {
    let (_dir, conn) = test_conn();
    let cash = asset(&conn, "Assets/Cash");

    let err = record_transaction(
        &conn,
        &RecordRequest {
            kind: RecordKind::Expense,
            amount_cents: 100,
            date: None,
            description: String::new(),
            source_account: Some(AccountRef::Id(cash)),
            destination_account: None,
            category: Some(CategoryRef::Id("no-such-category".into())),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("分类不存在"));
}

#[test]
fn test_create_account_rejects_overflowing_opening_balance() {
    let (_dir, conn) = test_conn();

    let err = create_account(
        &conn,
        &CreateAccountRequest {
            path: "Assets/Cash".into(),
            description: None,
            opening_balance_cents: Some(crate::utils::money::MAX_AMOUNT_CENTS + 1),
            opening_balance_date: None,
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("金额格式无效"));
    // The account must not be left behind by the rejected request.
    assert!(list_accounts(&conn).unwrap().is_empty());
}
