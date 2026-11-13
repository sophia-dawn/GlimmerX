//! Versioned schema migrations for the SQLCipher database.
//!
//! Uses `rusqlite_migration`, which stores the current version in
//! `PRAGMA user_version` instead of a bookkeeping table. Every migration runs
//! inside one transaction; connection-level PRAGMAs are applied outside it.

use std::path::PathBuf;

use chrono::Local;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::db::AppError;

/// Schema v1 is the schema shipped with 0.1.2. Existing databases have
/// `user_version = 0`; applying this idempotent baseline stamps them as v1
/// without modifying any data.
const MIGRATIONS_SLICE: &[M] = &[M::up(include_str!("schema_v1.sql"))];
const MIGRATIONS: Migrations = Migrations::from_slice(MIGRATIONS_SLICE);

/// Apply connection-level PRAGMAs and the baseline SQL directly.
///
/// Used by `init_schema` for bare test/temporary databases. Real databases go
/// through [`migrate`], which records the schema version.
#[allow(dead_code)]
pub fn init_baseline(conn: &Connection) -> Result<(), AppError> {
    apply_pragmas(conn)?;
    conn.execute_batch(include_str!("schema_v1.sql"))
        .map_err(|e| AppError::DatabaseError(format!("数据库初始化失败：{e}")))
}

fn apply_pragmas(conn: &Connection) -> Result<(), AppError> {
    // These must stay outside a transaction: journal_mode has no effect inside
    // one, and foreign_keys is a per-connection setting.
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )
    .map_err(|e| AppError::DatabaseError(format!("数据库初始化失败：{e}")))
}

/// Apply PRAGMAs and migrate to the latest schema version.
///
/// Must run **after** `PRAGMA key` on an encrypted database, otherwise the
/// `user_version` stored in the encrypted first page cannot be read.
/// `is_new` is true for freshly created databases, which need no backup.
pub fn migrate(conn: &mut Connection, path: &PathBuf, is_new: bool) -> Result<(), AppError> {
    apply_pragmas(conn)?;

    let pending = MIGRATIONS
        .pending_migrations(conn)
        .map_err(|e| AppError::DatabaseError(format!("无法读取数据库版本：{e}")))?;
    if pending < 0 {
        return Err(AppError::DatabaseError(
            "数据库由更新版本创建，请升级应用后再打开".to_string(),
        ));
    }
    if pending == 0 {
        return Ok(());
    }

    if !is_new {
        backup_before_migration(conn, path)?;
    }
    MIGRATIONS
        .to_latest(conn)
        .map_err(|e| AppError::DatabaseError(format!("数据库迁移失败，原文件已保留备份：{e}")))
}

fn backup_before_migration(conn: &Connection, path: &PathBuf) -> Result<(), AppError> {
    // Fold the WAL into the main file so the copy is self-contained.
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
        .map_err(|e| AppError::DatabaseError(format!("迁移前整理数据库失败：{e}")))?;

    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_path = PathBuf::from(format!("{}.bak-{stamp}", path.display()));
    let tmp_path = PathBuf::from(format!("{}.bak-{stamp}.tmp", path.display()));
    std::fs::copy(path, &tmp_path)
        .map_err(|e| AppError::DatabaseError(format!("迁移前备份失败，已取消迁移：{e}")))?;
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }
    std::fs::rename(&tmp_path, &backup_path)
        .map_err(|e| AppError::DatabaseError(format!("迁移前备份失败，已取消迁移：{e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A pre-0.1.2 database: partially initialized, user_version still 0.
    fn legacy_db() -> (TempDir, PathBuf, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("legacy.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO settings (key, value) VALUES ('keep', 'me');",
        )
        .unwrap();
        (dir, path, conn)
    }

    fn backup_count(dir: &TempDir) -> usize {
        std::fs::read_dir(dir.path())
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains(".bak-")
            })
            .count()
    }

    #[test]
    fn test_migrations_validate() {
        MIGRATIONS.validate().unwrap();
    }

    #[test]
    fn test_legacy_database_migrated_and_data_preserved() {
        let (dir, path, mut conn) = legacy_db();
        migrate(&mut conn, &path, false).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 1);

        let value: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'keep'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(value, "me", "existing data must survive the migration");

        for table in [
            "accounts",
            "account_meta",
            "transactions",
            "postings",
            "categories",
            "budgets",
            "transaction_hashes",
        ] {
            let exists: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(exists, 1, "missing table {table}");
        }

        assert_eq!(backup_count(&dir), 1, "one backup before migration");
        migrate(&mut conn, &path, false).unwrap();
        assert_eq!(backup_count(&dir), 1, "second migration is a no-op");
    }

    #[test]
    fn test_newer_database_is_rejected() {
        let (_dir, path, mut conn) = legacy_db();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
        let err = migrate(&mut conn, &path, false).unwrap_err();
        assert!(format!("{err}").contains("版本"));
    }

    #[test]
    fn test_create_and_open_apply_connection_pragmas() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("app.db");
        {
            let db = crate::db::Database::create(&path, "secret").unwrap();
            let conn = db.get_conn().unwrap();
            let foreign_keys: i64 = conn
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .unwrap();
            assert_eq!(foreign_keys, 1);
        }
        {
            let db = crate::db::Database::open(&path, "secret").unwrap();
            let conn = db.get_conn().unwrap();
            let foreign_keys: i64 = conn
                .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
                .unwrap();
            assert_eq!(foreign_keys, 1, "open() must configure PRAGMAs too");
            let version: i64 = conn
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .unwrap();
            assert_eq!(version, 1);
        }
    }
}
