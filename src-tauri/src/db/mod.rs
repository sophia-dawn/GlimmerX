use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;
use thiserror::Error;

pub mod accounts;
pub mod budgets;
pub mod categories;
pub mod dashboard;
pub mod export;
pub mod import;
mod recent_dbs;
pub mod reports;
mod schema;
pub mod transactions;
#[allow(unused_imports)]
pub use export::{export_beancount, export_csv, ExportResult};
#[allow(unused_imports)]
pub use recent_dbs::{RecentDb, RecentDbs};
pub use schema::init_schema;

/// Application-level error type.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("errors.invalidPassword")]
    InvalidPassword,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("I/O error: {0}")]
    IoError(String),
}

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
        .map_err(|e| AppError::DatabaseError(format!("Failed to set encryption key: {}", e)))
}

impl Database {
    /// Open an existing SQLCipher database at the given path with the given password.
    /// Returns an error if the password is incorrect or the file doesn't exist.
    pub fn open(path: &PathBuf, password: &str) -> Result<Self, AppError> {
        if !path.exists() {
            return Err(AppError::NotFound(format!(
                "Database file not found: {}",
                path.display()
            )));
        }

        let conn = Connection::open(path)
            .map_err(|e| AppError::DatabaseError(format!("Failed to open database file: {}", e)))?;

        set_pragma_key(&conn, password)?;

        // Verify the password by running a simple query.
        // SQLCipher will return garbage or fail if the key is wrong.
        conn.query_row("SELECT count(*) FROM sqlite_master;", [], |_| Ok(()))
            .map_err(|_| AppError::InvalidPassword)?;

        Ok(Self {
            path: path.clone(),
            conn: Mutex::new(conn),
        })
    }

    /// Create a new SQLCipher database at the given path and initialize the schema.
    pub fn create(path: &PathBuf, password: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(|e| {
            AppError::DatabaseError(format!("Failed to create database file: {}", e))
        })?;

        set_pragma_key(&conn, password)?;

        init_schema(&conn)?;

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
            .map_err(|e| AppError::DatabaseError(format!("Failed to change password: {}", e)))
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
        self.conn
            .lock()
            .map_err(|e| AppError::DatabaseError(format!("Database mutex poisoned: {}", e)))
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        // Attempt to acquire connection and execute checkpoint
        // Handle mutex poisoning gracefully - if mutex is poisoned,
        // another thread panicked while holding it, but we can still recover
        if let Ok(guard) = self.conn.lock() {
            let result = guard.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
            if let Err(e) = result {
                eprintln!(
                    "[Database::drop] checkpoint failed for {}: {}",
                    self.path.display(),
                    e
                );
            } else {
                eprintln!(
                    "[Database::drop] checkpoint succeeded for {}",
                    self.path.display()
                );
            }
        } else {
            eprintln!(
                "[Database::drop] mutex poisoned, cannot checkpoint for {}",
                self.path.display()
            );
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
}
