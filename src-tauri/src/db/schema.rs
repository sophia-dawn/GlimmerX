use rusqlite::{Connection, Result};

use crate::db::AppError;

/// Initialize database schema with all tables.
/// Idempotent: safe to call on an already-initialized database.
#[allow(dead_code)]
pub fn init_schema(conn: &Connection) -> Result<(), AppError> {
    crate::db::migrations::init_baseline(conn)
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
