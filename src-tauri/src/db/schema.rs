use rusqlite::{Connection, Result};

use crate::db::AppError;

/// Initialize database schema with all tables.
/// Idempotent: safe to call on an already-initialized database.
pub fn init_schema(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;

        -- 账户
        CREATE TABLE IF NOT EXISTS accounts (
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

        -- 账户元数据
        CREATE TABLE IF NOT EXISTS account_meta (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            key TEXT NOT NULL,
            value TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(account_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_account_meta_account ON account_meta(account_id);

        -- 交易
        -- **软删除设计**: deleted_at 字段支持交易撤销恢复
        -- ** reconciliation 设计**: is_reconciled 标记已核对的交易
        CREATE TABLE IF NOT EXISTS transactions (
            id          TEXT PRIMARY KEY,
            date        TEXT NOT NULL,
            description TEXT NOT NULL,
            category_id TEXT REFERENCES categories(id),
            is_reconciled INTEGER NOT NULL DEFAULT 0 CHECK (is_reconciled IN (0, 1)),
            deleted_at  TEXT,  -- NULL 表示未删除，ISO 8601 时间戳表示已删除
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        -- 交易分录
        -- **sequence 设计**: 记录分录顺序（用于显示排序）
        CREATE TABLE IF NOT EXISTS postings (
            id             TEXT PRIMARY KEY,
            transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
            account_id     TEXT NOT NULL REFERENCES accounts(id),
            amount         INTEGER NOT NULL,
            sequence       INTEGER NOT NULL DEFAULT 0,  -- 分录顺序（0=第一条）
            created_at     TEXT NOT NULL
        );

        -- 分类
        CREATE TABLE IF NOT EXISTS categories (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            type       TEXT NOT NULL CHECK (type IN ('income', 'expense')),
            icon       TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        -- 预算
        CREATE TABLE IF NOT EXISTS budgets (
            id          TEXT PRIMARY KEY,
            category_id TEXT NOT NULL REFERENCES categories(id),
            amount      INTEGER NOT NULL,
            period      TEXT NOT NULL,
            rollover    INTEGER NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );

        -- 应用设置
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        -- Transaction hashes for duplicate detection
        CREATE TABLE IF NOT EXISTS transaction_hashes (
            transaction_id TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL UNIQUE,
            import_source TEXT,
            imported_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_transaction_hashes_hash ON transaction_hashes(content_hash);

        -- Indexes
        CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_type_name
            ON accounts(type, name);
        CREATE INDEX IF NOT EXISTS idx_postings_transaction ON postings(transaction_id);
        CREATE INDEX IF NOT EXISTS idx_postings_account ON postings(account_id);
        CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
        -- 复合索引优化导出查询：WHERE deleted_at IS NULL AND date >= ? AND date <= ?
        CREATE INDEX IF NOT EXISTS idx_transactions_date_deleted
            ON transactions(date, deleted_at);
        CREATE INDEX IF NOT EXISTS idx_accounts_type ON accounts(type);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_type_name
            ON categories(type, name);
        CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(type);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_budgets_category ON budgets(category_id);
    "#,
    )
    .map_err(|e| AppError::DatabaseError(format!("Failed to initialize schema: {}", e)))?;

    Ok(())
}

/// Ensure the transaction_hashes table exists.
/// Used for migrations on existing databases.
pub fn ensure_transaction_hashes_table(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS transaction_hashes (
            transaction_id TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL UNIQUE,
            import_source TEXT,
            imported_at TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_transaction_hashes_hash ON transaction_hashes(content_hash);",
    )
    .map_err(|e| {
        AppError::DatabaseError(format!(
            "Failed to create transaction_hashes: {}",
            e
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    struct TestEnv {
        _dir: TempDir,
        conn: Connection,
    }

    fn test_env() -> TestEnv {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        TestEnv { _dir: dir, conn }
    }

    #[test]
    fn test_init_schema_creates_tables() {
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let tables: Vec<String> = env
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(tables.contains(&"accounts".to_string()));
        assert!(tables.contains(&"transactions".to_string()));
        assert!(tables.contains(&"postings".to_string()));
        assert!(tables.contains(&"categories".to_string()));
        assert!(tables.contains(&"budgets".to_string()));
        assert!(tables.contains(&"settings".to_string()));
    }

    #[test]
    fn test_init_schema_idempotent() {
        let env = test_env();
        init_schema(&env.conn).unwrap();
        init_schema(&env.conn).unwrap();
    }

    #[test]
    fn test_foreign_keys_enabled() {
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let enabled: i32 = env
            .conn
            .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
    }

    #[test]
    fn test_wal_mode_enabled() {
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let journal_mode: String = env
            .conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .unwrap();
        assert_eq!(journal_mode, "wal");
    }

    #[test]
    fn test_accounts_has_new_columns() {
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let pragma: Vec<String> = env
            .conn
            .prepare("PRAGMA table_info(accounts)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        assert!(pragma.contains(&"description".to_string()));
        assert!(pragma.contains(&"account_number".to_string()));
        assert!(pragma.contains(&"is_system".to_string()));
    }

    #[test]
    fn test_accounts_unique_index() {
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let now = crate::utils::time::now_rfc3339();
        env.conn.execute(
"INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
         VALUES ('1', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        ).unwrap();

        // Duplicate (type, name) should fail
        let result = env.conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
             VALUES ('2', 'Test', 'asset', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_err(),
            "Duplicate (type, name) should violate unique index"
        );

        // Same name but different type should succeed
        let result = env.conn.execute(
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
        let env = test_env();
        init_schema(&env.conn).unwrap();

        let now = crate::utils::time::now_rfc3339();
        let result = env.conn.execute(
"INSERT INTO accounts (id, name, type, currency, description, is_system, created_at, updated_at)
         VALUES ('1', 'Bad', 'invalid_type', 'CNY', '', 0, ?1, ?1)",
            [&now],
        );
        assert!(
            result.is_err(),
            "Invalid type should violate CHECK constraint"
        );
    }
}
