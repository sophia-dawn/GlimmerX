//! Tests for account CRUD, metadata, opening balances and transfers.

use rusqlite::{params, Connection};

use crate::db::AppError;

use super::*;
use tempfile::TempDir;

fn test_conn() -> (TempDir, Connection) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("test.db");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE accounts (
                id                  TEXT PRIMARY KEY,
                name                TEXT NOT NULL,
                type                TEXT NOT NULL CHECK (type IN ('asset', 'liability', 'income', 'expense', 'equity')),
                currency            TEXT NOT NULL DEFAULT 'CNY',
                description         TEXT NOT NULL DEFAULT '',
                account_number      TEXT,
                is_system           INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
                iban                TEXT,
                is_active           INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
                include_net_worth   INTEGER NOT NULL DEFAULT 1 CHECK (include_net_worth IN (0, 1)),
                created_at          TEXT NOT NULL,
                updated_at          TEXT NOT NULL
            );

            CREATE TABLE account_meta (
                id          TEXT PRIMARY KEY,
                account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
                key         TEXT NOT NULL,
                value       TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );

            CREATE UNIQUE INDEX idx_accounts_type_name
                ON accounts(type, name);

            CREATE UNIQUE INDEX idx_account_meta_account_key
                ON account_meta(account_id, key);

            CREATE TABLE transactions (
                id          TEXT PRIMARY KEY,
                date        TEXT NOT NULL,
                description TEXT NOT NULL,
                category_id TEXT,
                deleted_at  TEXT,
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE TABLE postings (
                id             TEXT PRIMARY KEY,
                transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
                account_id     TEXT NOT NULL REFERENCES accounts(id),
                amount         INTEGER NOT NULL,
                created_at     TEXT NOT NULL
            );
        "#,
        )
        .unwrap();
    (dir, conn)
}

fn make_account(name: &str, account_type: &str) -> AccountRecord {
    AccountRecord {
        id: String::new(),
        name: name.to_string(),
        account_type: account_type.to_string(),
        description: String::new(),
        account_number: None,
        is_system: false,
        iban: None,
        is_active: true,
        include_net_worth: true,
        created_at: String::new(),
        updated_at: String::new(),
    }
}

#[test]
fn test_insert_account() {
    let (_dir, conn) = test_conn();
    let acct = make_account("招商银行", "asset");
    let id = insert_account(&conn, &acct).unwrap();
    assert!(!id.is_empty());

    let found = find_account(&conn, &id).unwrap().unwrap();
    assert_eq!(found.name, "招商银行");
}

#[test]
fn test_list_accounts_by_type() {
    let (_dir, conn) = test_conn();

    let _id1 = insert_account(&conn, &make_account("招商银行", "asset")).unwrap();
    let _id2 = insert_account(&conn, &make_account("现金", "asset")).unwrap();
    let _id3 = insert_account(&conn, &make_account("花呗", "liability")).unwrap();

    let accounts = list_accounts(&conn).unwrap();
    assert_eq!(accounts.len(), 3);

    // Flat structure - all accounts are roots
}

#[test]
fn test_update_account() {
    let (_dir, conn) = test_conn();

    let acct = make_account("OldName", "asset");
    let id = insert_account(&conn, &acct).unwrap();

    update_account(
        &conn,
        &UpdateAccountParams {
            id: &id,
            name: Some("NewName"),
            description: None,
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    )
    .unwrap();

    let updated = find_account(&conn, &id).unwrap().unwrap();
    assert_eq!(updated.name, "NewName");
}

#[test]
fn test_delete_account() {
    let (_dir, conn) = test_conn();

    let acct = make_account("ToDelete", "expense");
    let id = insert_account(&conn, &acct).unwrap();
    delete_account(&conn, &id).unwrap();

    assert!(find_account(&conn, &id).unwrap().is_none());
}

#[test]
fn test_delete_account_with_transactions_blocked() {
    let (_dir, conn) = test_conn();

    let acct = make_account("WithTx", "asset");
    let acct_id = insert_account(&conn, &acct).unwrap();

    // Create a transaction with a posting to this account
    let tx_id = "test-tx-1";
    conn.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test Tx', NULL, ?2, ?2)",
        rusqlite::params![tx_id, crate::utils::time::now_rfc3339()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 1000, ?4)",
        rusqlite::params![
            "posting-1",
            tx_id,
            acct_id,
            crate::utils::time::now_rfc3339()
        ],
    )
    .unwrap();

    let result = delete_account(&conn, &acct_id);
    assert!(
        result.is_err(),
        "Should block deletion of account with transactions"
    );
}

#[test]
fn test_create_account_with_path_flat() {
    let (_dir, conn) = test_conn();

    let id = create_account_with_path(&conn, "Assets/招商银行", None).unwrap();
    let acct = find_account(&conn, &id).unwrap().unwrap();
    assert_eq!(acct.name, "招商银行");
    assert_eq!(acct.account_type, "asset");

    // Sub-path becomes name
    let id2 = create_account_with_path(&conn, "Liabilities/信用卡/花呗", None).unwrap();
    let acct2 = find_account(&conn, &id2).unwrap().unwrap();
    assert_eq!(acct2.name, "信用卡/花呗");
    assert_eq!(acct2.account_type, "liability");
}

#[test]
fn test_schema_unique_type_name() {
    let (_dir, conn) = test_conn();

    let now = crate::utils::time::now_rfc3339();
    conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('1', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        )
        .unwrap();

    // Duplicate (type, name) should fail
    let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('2', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
    assert!(
        result.is_err(),
        "Duplicate (type, name) should violate unique index"
    );

    // Same name but different type should succeed
    let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('3', 'Test', 'liability', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
    assert!(
        result.is_ok(),
        "Same name with different type should be allowed"
    );
}

#[test]
fn test_accounts_check_type_constraint() {
    let (_dir, conn) = test_conn();

    let now = crate::utils::time::now_rfc3339();
    let result = conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('1', 'Bad', 'invalid_type', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
    assert!(
        result.is_err(),
        "Invalid type should violate CHECK constraint"
    );
}

#[test]
fn test_balance_aggregation_no_sign_flip() {
    let (_dir, conn) = test_conn();

    // Create a liability account
    let acct = make_account("花呗", "liability");
    let id = insert_account(&conn, &acct).unwrap();

    // Add a posting
    let tx_id = "test-tx";
    conn.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test', NULL, ?2, ?2)",
        rusqlite::params![tx_id, crate::utils::time::now_rfc3339()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 5000, ?4)",
        rusqlite::params!["posting-1", tx_id, id, crate::utils::time::now_rfc3339()],
    )
    .unwrap();

    let balance = get_account_balance(&conn, &id).unwrap();
    // Raw balance should be returned as-is for liability (no sign flip)
    assert_eq!(balance, 5000);

    // Negative posting should stay negative
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            "posting-2",
            tx_id,
            id,
            -2000i64,
            crate::utils::time::now_rfc3339()
        ],
    )
    .unwrap();

    let balance = get_account_balance(&conn, &id).unwrap();
    assert_eq!(balance, 3000); // 5000 - 2000 = 3000
}

#[test]
fn test_insert_meta() {
    let (_dir, conn) = test_conn();
    let acct = make_account("TestAccount", "asset");
    let account_id = insert_account(&conn, &acct).unwrap();

    let meta_id = insert_meta(&conn, &account_id, "account_role", "ccAsset").unwrap();
    assert!(!meta_id.is_empty());

    let value = get_meta_value(&conn, &account_id, "account_role")
        .unwrap()
        .unwrap();
    assert_eq!(value, "ccAsset");
}

#[test]
fn test_get_meta_for_account() {
    let (_dir, conn) = test_conn();
    let acct = make_account("TestAccount", "asset");
    let account_id = insert_account(&conn, &acct).unwrap();

    insert_meta(&conn, &account_id, "bar", "value_bar").unwrap();
    insert_meta(&conn, &account_id, "foo", "value_foo").unwrap();

    let metas = get_meta_for_account(&conn, &account_id).unwrap();
    assert_eq!(metas.len(), 2);
    assert_eq!(metas[0].key, "bar");
    assert_eq!(metas[1].key, "foo");
}

#[test]
fn test_meta_unique_constraint() {
    let (_dir, conn) = test_conn();
    let acct = make_account("TestAccount", "asset");
    let account_id = insert_account(&conn, &acct).unwrap();

    insert_meta(&conn, &account_id, "unique_key", "value1").unwrap();

    let result = insert_meta(&conn, &account_id, "unique_key", "value2");
    assert!(
        result.is_err(),
        "Duplicate meta key should violate unique constraint"
    );
}

#[test]
fn test_delete_meta_for_account() {
    let (_dir, conn) = test_conn();
    let acct = make_account("TestAccount", "asset");
    let account_id = insert_account(&conn, &acct).unwrap();

    insert_meta(&conn, &account_id, "key1", "val1").unwrap();
    insert_meta(&conn, &account_id, "key2", "val2").unwrap();

    let deleted = delete_meta_for_account(&conn, &account_id).unwrap();
    assert_eq!(deleted, 2);

    let metas = get_meta_for_account(&conn, &account_id).unwrap();
    assert!(metas.is_empty());
}

#[test]
fn test_set_meta_batch() {
    let (_dir, conn) = test_conn();
    let acct = make_account("TestAccount", "asset");
    let account_id = insert_account(&conn, &acct).unwrap();

    let metas = vec![
        ("key1".to_string(), "val1".to_string()),
        ("key2".to_string(), "val2".to_string()),
        ("key3".to_string(), "val3".to_string()),
    ];

    let count = set_meta_batch(&conn, &account_id, &metas).unwrap();
    assert_eq!(count, 3);

    let result = get_meta_for_account(&conn, &account_id).unwrap();
    assert_eq!(result.len(), 3);
    let values: std::collections::HashMap<&str, &str> = result
        .iter()
        .map(|m| (m.key.as_str(), m.value.as_str()))
        .collect();
    assert_eq!(values.get("key1"), Some(&"val1"));
    assert_eq!(values.get("key2"), Some(&"val2"));
    assert_eq!(values.get("key3"), Some(&"val3"));

    // Second call with same keys but different values - upsert behavior
    let metas2 = vec![
        ("key1".to_string(), "updated1".to_string()),
        ("key2".to_string(), "updated2".to_string()),
        ("key3".to_string(), "updated3".to_string()),
    ];

    let count2 = set_meta_batch(&conn, &account_id, &metas2).unwrap();
    assert_eq!(count2, 3);

    let result2 = get_meta_for_account(&conn, &account_id).unwrap();
    assert_eq!(result2.len(), 3);
    let values2: std::collections::HashMap<&str, &str> = result2
        .iter()
        .map(|m| (m.key.as_str(), m.value.as_str()))
        .collect();
    assert_eq!(values2.get("key1"), Some(&"updated1"));
    assert_eq!(values2.get("key2"), Some(&"updated2"));
    assert_eq!(values2.get("key3"), Some(&"updated3"));
}

#[test]
fn test_get_account_transactions_multiple() {
    let (_dir, conn) = test_conn();

    // Create account
    let acct = make_account("TestAccount", "asset");
    let acct_id = insert_account(&conn, &acct).unwrap();

    // Create 3 transactions with postings
    for i in 1..=3 {
        let tx_id = format!("tx-{}", i);
        let now = crate::utils::time::now_rfc3339();
        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
                 VALUES (?1, ?2, ?3, NULL, ?4, ?4)",
            rusqlite::params![
                &tx_id,
                format!("2024-0{}-01", i),
                format!("Transaction {}", i),
                &now
            ],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![format!("posting-{}", i), &tx_id, &acct_id, i * 1000, &now],
        )
        .unwrap();
    }

    let transactions = get_account_transactions(&conn, &acct_id, None, None).unwrap();
    assert_eq!(transactions.len(), 3);

    // Verify amounts are correct
    let amounts: Vec<i64> = transactions.iter().map(|t| t.amount).collect();
    assert!(amounts.contains(&1000));
    assert!(amounts.contains(&2000));
    assert!(amounts.contains(&3000));
}

#[test]
fn test_update_account_duplicate_type_name_blocked() {
    let (_dir, conn) = test_conn();

    // 创建两个同类型账户
    let acct1 = make_account("账户A", "asset");
    let _id1 = insert_account(&conn, &acct1).unwrap();
    let acct2 = make_account("账户B", "asset");
    let id2 = insert_account(&conn, &acct2).unwrap();

    // 尝试将账户B的名称改为账户A的名称（同类型）
    let result = update_account(
        &conn,
        &UpdateAccountParams {
            id: &id2,
            name: Some("账户A"),
            description: None,
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    );

    // 应返回 ValidationError（同类型同名冲突）
    assert!(
        result.is_err(),
        "Should block duplicate (type, name) on update"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, AppError::ValidationError(msg) if msg == "errors.accountNameAlreadyExists"),
        "Error should be ValidationError with correct key"
    );
}

#[test]
fn test_update_account_same_name_different_type_allowed() {
    let (_dir, conn) = test_conn();

    // 创建不同类型的账户
    let acct1 = make_account("账户X", "asset");
    let _id1 = insert_account(&conn, &acct1).unwrap();
    let acct2 = make_account("账户Y", "liability");
    let id2 = insert_account(&conn, &acct2).unwrap();

    // 尝试将 liability 账户名称改为与 asset 账户相同（不同类型）
    let result = update_account(
        &conn,
        &UpdateAccountParams {
            id: &id2,
            name: Some("账户X"),
            description: None,
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    );

    // 应成功（不同类型可以同名）
    assert!(
        result.is_ok(),
        "Same name with different type should be allowed on update"
    );
}

#[test]
fn test_update_account_same_name_allowed() {
    let (_dir, conn) = test_conn();

    // 创建一个账户
    let acct = make_account("账户A", "asset");
    let id = insert_account(&conn, &acct).unwrap();

    // 更新账户的其他字段，名称保持不变
    let result = update_account(
        &conn,
        &UpdateAccountParams {
            id: &id,
            name: Some("账户A"), // 同名
            description: Some("新描述"),
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    );

    // 应成功
    assert!(result.is_ok(), "Should allow update with same name");

    // 验证描述已更新
    let updated = find_account(&conn, &id).unwrap().unwrap();
    assert_eq!(updated.description, "新描述");
}

#[test]
fn test_get_account_balances_batch_empty() {
    let (_dir, conn) = test_conn();

    let result = get_account_balances_batch(&conn, &[]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_get_account_balances_batch_multiple() {
    let (_dir, conn) = test_conn();

    let acct1 = make_account("银行", "asset");
    let acct2 = make_account("现金", "asset");
    let id1 = insert_account(&conn, &acct1).unwrap();
    let id2 = insert_account(&conn, &acct2).unwrap();

    // Accounts without postings won't appear in batch results
    let balances = get_account_balances_batch(&conn, &[id1.clone(), id2.clone()]).unwrap();
    assert_eq!(balances.len(), 0);

    let tx_id = "test-tx";
    let now = crate::utils::time::now_rfc3339();
    conn.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES (?1, '2024-01-01', 'Test', NULL, ?2, ?2)",
        rusqlite::params![tx_id, &now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, 1000, ?4)",
        rusqlite::params!["posting-1", tx_id, &id1, &now],
    )
    .unwrap();

    let balances = get_account_balances_batch(&conn, &[id1.clone(), id2.clone()]).unwrap();
    assert_eq!(balances.len(), 1);
    assert_eq!(balances[0].0, id1);
    assert_eq!(balances[0].1, 1000);
}

#[test]
fn test_create_account_with_path_invalid_root() {
    let (_dir, conn) = test_conn();
    let result = create_account_with_path(&conn, "Foo/Cash", None);
    assert!(result.is_err());
    match result.unwrap_err() {
        AppError::ValidationError(msg) => assert!(msg.contains("Invalid account root type")),
        other => panic!("Expected ValidationError, got: {:?}", other),
    }
}

#[test]
fn test_create_account_with_path_duplicate_blocked() {
    let (_dir, conn) = test_conn();
    create_account_with_path(&conn, "Assets/Cash", None).unwrap();
    let result = create_account_with_path(&conn, "Assets/Cash", None);
    assert!(result.is_err());
}

#[test]
fn test_update_account_optional_fields() {
    let (_dir, conn) = test_conn();
    let id = insert_account(&conn, &make_account("Cash", "asset")).unwrap();

    // Set optional fields
    update_account(
        &conn,
        &UpdateAccountParams {
            id: &id,
            name: None,
            description: Some("desc"),
            account_number: Some(Some("6222-0001")),
            iban: Some(Some("CN-IBAN-1")),
            is_active: Some(false),
            include_net_worth: Some(false),
        },
    )
    .unwrap();
    let updated = find_account(&conn, &id).unwrap().unwrap();
    assert_eq!(updated.description, "desc");
    assert_eq!(updated.account_number.as_deref(), Some("6222-0001"));
    assert!(!updated.is_active);

    // Clear optional fields
    update_account(
        &conn,
        &UpdateAccountParams {
            id: &id,
            name: None,
            description: None,
            account_number: Some(None),
            iban: Some(None),
            is_active: None,
            include_net_worth: None,
        },
    )
    .unwrap();
    let updated = find_account(&conn, &id).unwrap().unwrap();
    assert!(updated.account_number.is_none());
    assert!(updated.iban.is_none());

    // No-op update returns early
    update_account(
        &conn,
        &UpdateAccountParams {
            id: &id,
            name: None,
            description: None,
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    )
    .unwrap();
}

#[test]
fn test_update_account_name_conflict_blocked() {
    let (_dir, conn) = test_conn();
    let id1 = insert_account(&conn, &make_account("Cash", "asset")).unwrap();
    insert_account(&conn, &make_account("Bank", "asset")).unwrap();

    let result = update_account(
        &conn,
        &UpdateAccountParams {
            id: &id1,
            name: Some("Bank"),
            description: None,
            account_number: None,
            iban: None,
            is_active: None,
            include_net_worth: None,
        },
    );
    assert!(result.is_err());
}

#[test]
fn test_delete_account_missing_blocked() {
    let (_dir, conn) = test_conn();
    let result = delete_account(&conn, "missing");
    assert!(result.is_err());
}

#[test]
fn test_delete_account_system_blocked() {
    let (_dir, conn) = test_conn();
    conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('sys-1', 'Equity', 'equity', 'CNY', '', 1, '2024-01-01T00:00:00+08:00', '2024-01-01T00:00:00+08:00')",
            [],
        )
        .unwrap();
    let result = delete_account(&conn, "sys-1");
    assert!(result.is_err());
    assert!(find_account(&conn, "sys-1").unwrap().is_some());
}

#[test]
fn test_delete_account_with_opening_balance() {
    let (_dir, conn) = test_conn();
    let id = create_account_with_path(&conn, "Assets/Cash", None).unwrap();
    create_opening_balance(&conn, &id, 100000, "Equity", "Opening Balances", None).unwrap();

    delete_account(&conn, &id).unwrap();
    assert!(find_account(&conn, &id).unwrap().is_none());
    let tx_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM transactions WHERE description = 'Opening Balance'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tx_count, 0);
}

#[test]
fn test_create_transfer_validation() {
    let (_dir, conn) = test_conn();
    let id1 = insert_account(&conn, &make_account("Cash", "asset")).unwrap();
    let id2 = insert_account(&conn, &make_account("Bank", "asset")).unwrap();

    // Non-positive amount
    let result = create_transfer(&conn, &id1, &id2, 0, "x");
    assert!(result.is_err());

    // Same account
    let result = create_transfer(&conn, &id1, &id1, 100, "x");
    assert!(result.is_err());
}

#[test]
fn test_get_account_transactions_date_filters() {
    let (_dir, conn) = test_conn();
    let id1 = insert_account(&conn, &make_account("Cash", "asset")).unwrap();
    let id2 = insert_account(&conn, &make_account("Food", "expense")).unwrap();

    let now = crate::utils::time::now_rfc3339();
    conn.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
             VALUES ('tx-1', '2024-01-15', 'Lunch', NULL, ?1, ?1)",
        [&now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES ('p-1', 'tx-1', ?1, -1000, ?2)",
        params![&id1, &now],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES ('p-2', 'tx-1', ?1, 1000, ?2)",
        params![&id2, &now],
    )
    .unwrap();

    let only_from = get_account_transactions(&conn, &id1, Some("2024-01-01"), None).unwrap();
    assert_eq!(only_from.len(), 1);
    let only_to = get_account_transactions(&conn, &id1, None, Some("2024-01-31")).unwrap();
    assert_eq!(only_to.len(), 1);
    let both =
        get_account_transactions(&conn, &id1, Some("2024-01-01"), Some("2024-01-31")).unwrap();
    assert_eq!(both.len(), 1);
    let none = get_account_transactions(&conn, &id1, None, None).unwrap();
    assert_eq!(none.len(), 1);
}

#[test]
fn test_create_opening_balance_updates_existing() {
    let (_dir, conn) = test_conn();
    let id = create_account_with_path(&conn, "Assets/Cash", None).unwrap();

    create_opening_balance(&conn, &id, 100000, "Equity", "Opening Balances", None).unwrap();
    create_opening_balance(&conn, &id, 200000, "Equity", "Opening Balances", None).unwrap();

    let (balance, _date) = get_opening_balance_for_account(&conn, &id)
        .unwrap()
        .expect("opening balance should exist");
    assert_eq!(balance, 200000);

    // Zero amount deletes the opening balance
    create_opening_balance(&conn, &id, 0, "Equity", "Opening Balances", None).unwrap();
    assert!(get_opening_balance_for_account(&conn, &id)
        .unwrap()
        .is_none());
}

#[test]
fn test_opening_balance_liability_negative() {
    let (_dir, conn) = test_conn();
    let id = create_account_with_path(&conn, "Liabilities/Card", None).unwrap();
    create_opening_balance(&conn, &id, 50000, "Equity", "Opening Balances", None).unwrap();
    let (balance, _date) = get_opening_balance_for_account(&conn, &id)
        .unwrap()
        .expect("opening balance should exist");
    assert_eq!(balance, -50000);
}

#[test]
fn test_batch_create_accounts() {
    let (_dir, conn) = test_conn();
    let records = batch_create_accounts(
        &conn,
        vec![
            BatchAccountInput {
                name: "Assets/Cash".to_string(),
                initial_balance: Some(100000),
                description: Some("main".to_string()),
                account_number: Some("0001".to_string()),
            },
            BatchAccountInput {
                name: "Expenses/Food".to_string(),
                initial_balance: None,
                description: None,
                account_number: None,
            },
        ],
    )
    .unwrap();
    assert_eq!(records.len(), 2);

    // Empty inputs -> empty result
    assert!(batch_create_accounts(&conn, vec![]).unwrap().is_empty());
}
