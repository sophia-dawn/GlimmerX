use rusqlite::{params, Connection, OptionalExtension};

use crate::db::AppError;

/// Get a setting value by key. Returns None if the key does not exist.
pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(value)
}

/// Set a setting value, inserting or overwriting.
pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::TempDir;

    /// Holds a temp dir + database so resources aren't dropped prematurely.
    struct TestEnv {
        _dir: TempDir,
        db: Database,
    }

    fn test_env() -> TestEnv {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        TestEnv { _dir: dir, db }
    }

    #[test]
    fn test_set_and_get_setting() {
        let env = test_env();
        let conn = env.db.get_conn().unwrap();
        set_setting(&conn, "ai.provider", "openai").unwrap();
        assert_eq!(
            get_setting(&conn, "ai.provider").unwrap(),
            Some("openai".to_string())
        );
    }

    #[test]
    fn test_get_setting_nonexistent() {
        let env = test_env();
        let conn = env.db.get_conn().unwrap();
        assert_eq!(get_setting(&conn, "nope").unwrap(), None);
    }

    #[test]
    fn test_set_setting_overwrite() {
        let env = test_env();
        let conn = env.db.get_conn().unwrap();
        set_setting(&conn, "ai.model", "gpt-4").unwrap();
        set_setting(&conn, "ai.model", "gpt-4o-mini").unwrap();
        assert_eq!(
            get_setting(&conn, "ai.model").unwrap(),
            Some("gpt-4o-mini".to_string())
        );
    }
}
