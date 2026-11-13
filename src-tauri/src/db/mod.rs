use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;

pub mod accounts;
pub mod budgets;
pub mod categories;
pub mod dashboard;
mod migrations;
mod recent_dbs;
pub mod reports;
mod schema;
pub mod settings;
pub mod transactions;
#[allow(unused_imports)]
pub use recent_dbs::{RecentDb, RecentDbs};
#[cfg(test)]
#[allow(unused_imports)]
pub use schema::init_schema;

/// Application-level error type.
///
/// `Display` is the single boundary where internal keys / English internals are
/// turned into friendly, localized messages (AGENTS.md hard rule). The frontend
/// must never transform, translate or reconstruct what it receives.
#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    InvalidPassword,
    NotFound(String),
    ValidationError(String),
    IoError(String),
    AiError(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::DatabaseError(s)
            | Self::NotFound(s)
            | Self::ValidationError(s)
            | Self::IoError(s) => crate::errors::user_text(s),
            Self::InvalidPassword => crate::errors::message("errors.invalidPassword"),
            Self::AiError(code) => crate::errors::message(&format!("errors.ai.{code}")),
        };
        f.write_str(&text)
    }
}

impl std::error::Error for AppError {}

impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

/// SQLCipher-encrypted database connection manager.
///
/// Provides `create`, `open`, and `change_password` operations.
/// The connection is wrapped in a `Mutex` for thread-safe access.
pub struct Database {
    pub path: PathBuf,
    conn: Mutex<Connection>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Database")
            .field("path", &self.path)
            .finish()
    }
}

/// Escape single quotes in password strings for SQL safety.
fn escape_password(password: &str) -> String {
    password.replace('\'', "''")
}

/// Execute PRAGMA key on an existing connection.
fn set_pragma_key(conn: &Connection, password: &str) -> Result<(), AppError> {
    let escaped = escape_password(password);
    conn.execute_batch(&format!("PRAGMA key = '{}';", escaped))
        .map_err(|e| {
            eprintln!("[glimmerx] failed to set encryption key: {e}");
            AppError::DatabaseError(crate::errors::message("errors.databaseError"))
        })
}

impl Database {
    /// Open an existing SQLCipher database at the given path with the given password.
    /// Returns an error if the password is incorrect or the file doesn't exist.
    pub fn open(path: &PathBuf, password: &str) -> Result<Self, AppError> {
        if !path.exists() {
            eprintln!("[glimmerx] database file not found: {}", path.display());
            return Err(AppError::NotFound(crate::errors::message(
                "errors.databaseFileNotFound",
            )));
        }

        let mut conn = Connection::open(path).map_err(|e| {
            eprintln!("[glimmerx] failed to open database file: {e}");
            AppError::DatabaseError(crate::errors::message("errors.databaseOpenFailed"))
        })?;

        set_pragma_key(&conn, password)?;

        // Verify the password by running a simple query.
        // SQLCipher will return garbage or fail if the key is wrong.
        conn.query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))
            .map_err(|_| AppError::InvalidPassword)?;

        // Apply connection PRAGMAs and migrate released 0.1.2 databases.
        // open() backs up the file before any pending migration is applied.
        migrations::migrate(&mut conn, path, false)?;

        Ok(Self {
            path: path.clone(),
            conn: Mutex::new(conn),
        })
    }

    /// Create a new SQLCipher database at the given path and initialize the schema.
    pub fn create(path: &PathBuf, password: &str) -> Result<Self, AppError> {
        let mut conn = Connection::open(path).map_err(|e| {
            eprintln!("[glimmerx] failed to create database file: {e}");
            AppError::DatabaseError(crate::errors::message("errors.databaseCreateFailed"))
        })?;

        set_pragma_key(&conn, password)?;

        // A brand-new database has nothing to lose, so no backup is needed.
        migrations::migrate(&mut conn, path, true)?;

        Ok(Self {
            path: path.clone(),
            conn: Mutex::new(conn),
        })
    }

    /// Change the database password.
    /// Uses `PRAGMA rekey` to re-encrypt the database with the new password.
    pub fn change_password(&self, new_password: &str) -> Result<(), AppError> {
        let conn = self.get_conn()?;
        let escaped = escape_password(new_password);
        conn.execute_batch(&format!("PRAGMA rekey = '{}';", escaped))
            .map_err(|e| {
                eprintln!("[glimmerx] failed to change password: {e}");
                AppError::DatabaseError(crate::errors::message("errors.databaseError"))
            })
    }

    pub fn change_password_with_verification(
        &self,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), AppError> {
        let test_conn = rusqlite::Connection::open(&self.path)?;
        let escaped_old = escape_password(old_password);
        test_conn
            .execute_batch(&format!("PRAGMA key = '{}';", escaped_old))
            .map_err(|_| AppError::InvalidPassword)?;
        test_conn
            .query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))
            .map_err(|_| AppError::InvalidPassword)?;
        drop(test_conn); // Explicit drop before using main connection
        self.change_password(new_password)?;
        Ok(())
    }

    /// Get a thread-safe reference to the underlying connection.
    pub fn get_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.conn.lock().map_err(|e| {
            eprintln!("[glimmerx] database mutex poisoned: {e}");
            AppError::DatabaseError(crate::errors::message("errors.databaseError"))
        })
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        // Attempt to acquire connection and execute checkpoint
        // Handle mutex poisoning gracefully - if mutex is poisoned,
        // another thread panicked while holding it, but we can still recover
        if let Ok(guard) = self.conn.lock() {
            let _ = guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Holds a temp dir and a database path so the dir isn't dropped.
    struct TestEnv {
        _dir: TempDir,
        path: PathBuf,
    }

    fn test_env() -> TestEnv {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        TestEnv { _dir: dir, path }
    }

    #[test]
    fn test_create_database() {
        let env = test_env();
        let db = Database::create(&env.path, "test_password").unwrap();
        assert!(env.path.exists());
        assert_eq!(db.path, env.path);
    }

    #[test]
    fn test_open_correct_password() {
        let env = test_env();
        Database::create(&env.path, "correct_password").unwrap();

        let db = Database::open(&env.path, "correct_password").unwrap();
        assert_eq!(db.path, env.path);
    }

    #[test]
    fn test_open_wrong_password() {
        let env = test_env();
        Database::create(&env.path, "correct_password").unwrap();

        let result = Database::open(&env.path, "wrong_password");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidPassword => {}
            other => panic!("Expected InvalidPassword, got: {:?}", other),
        }
    }

    #[test]
    fn test_open_nonexistent_file() {
        let env = test_env();
        // Use a path inside the temp dir that doesn't exist
        let path = env._dir.path().join("nonexistent.db");
        let result = Database::open(&path, "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_change_password() {
        let env = test_env();
        let db = Database::create(&env.path, "old_password").unwrap();

        db.change_password("new_password").unwrap();

        // Old password should no longer work
        let result = Database::open(&env.path, "old_password");
        assert!(result.is_err());

        // New password should work
        let db2 = Database::open(&env.path, "new_password").unwrap();
        assert_eq!(db2.path, env.path);
    }

    #[test]
    fn test_schema_initialized_via_create() {
        let env = test_env();
        let db = Database::create(&env.path, "password").unwrap();
        let conn = db.get_conn().unwrap();

        let count: i32 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table';",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(count >= 7, "Expected at least 7 tables, got {}", count);
    }

    #[test]
    fn test_password_with_special_characters() {
        let env = test_env();
        let _db = Database::create(&env.path, "password'with'quotes").unwrap();
        assert!(env.path.exists());

        let db2 = Database::open(&env.path, "password'with'quotes");
        assert!(db2.is_ok());
    }

    #[test]
    fn test_change_password_with_wrong_old_password() {
        let env = test_env();
        let db = Database::create(&env.path, "original_password").unwrap();

        // Wrong old password should fail
        let result = db.change_password_with_verification("wrong_password", "new_password");
        assert!(result.is_err());
        match result.unwrap_err() {
            AppError::InvalidPassword => {}
            other => panic!("Expected InvalidPassword, got: {:?}", other),
        }
    }

    #[test]
    fn test_change_password_with_correct_old_password() {
        let env = test_env();
        let db = Database::create(&env.path, "original_password").unwrap();

        // Correct old password should succeed
        db.change_password_with_verification("original_password", "new_password")
            .unwrap();

        // Verify new password works
        let db2 = Database::open(&env.path, "new_password").unwrap();
        assert_eq!(db2.path, env.path);
    }

    #[test]
    fn test_debug_format() {
        let env = test_env();
        let db = Database::create(&env.path, "secret").unwrap();
        let debug = format!("{:?}", db);
        assert!(debug.contains("Database"));
        // PathBuf's Debug output escapes path separators on Windows, so match
        // against the escaped representation instead of the raw path string.
        assert!(debug.contains(&format!("{:?}", env.path)));
    }

    #[test]
    fn test_create_fails_on_missing_directory() {
        let env = test_env();
        let path = env._dir.path().join("no-such-dir").join("test.db");
        let result = Database::create(&path, "secret");
        assert!(result.is_err());
    }
}
