//! Versioned schema migrations for the SQLCipher database.
//!
//! Uses `rusqlite_migration`, which stores the current version in
//! `PRAGMA user_version` instead of a bookkeeping table. Every migration runs
//! inside one transaction; connection-level PRAGMAs are applied outside it.

use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use rusqlite_migration::{HookResult, Migrations, M};

use crate::db::AppError;
use uuid::Uuid;

/// Current schema version stamped into `PRAGMA user_version`.
pub(crate) const SCHEMA_VERSION: i64 = 3;

/// Schema v1 shipped with 0.1.2. Existing databases have `user_version = 0`;
/// applying this idempotent baseline stamps them as v1 without touching data.
///
/// `Migrations` cannot be `const` any more because v2 carries a Rust hook, so it
/// is built once on first use. `MIGRATIONS.validate()` runs in the test suite.
static MIGRATIONS: LazyLock<Migrations> = LazyLock::new(|| {
    Migrations::new(vec![
        M::up(include_str!("schema_v1.sql")),
        M::up_with_hook(include_str!("schema_v2.sql"), up_v2_hook),
        M::up(include_str!("schema_v3.sql")),
    ])
});

/// Apply connection-level PRAGMAs and every schema version directly.
///
/// Test-only: the version is stamped, but no backup is written and no
/// `Migrations` bookkeeping runs. Production code always goes through
/// [`migrate`]; keeping this behind `cfg(test)` makes it impossible to
/// accidentally create an untracked schema.
#[cfg(test)]
pub fn init_baseline(conn: &Connection) -> Result<(), AppError> {
    apply_pragmas(conn)?;

    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| AppError::DatabaseError(format!("数据库初始化失败：{e}")))?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    conn.execute_batch(include_str!("schema_v1.sql"))
        .and_then(|_| conn.execute_batch(include_str!("schema_v2.sql")))
        .and_then(|_| backfill_category_accounts(conn))
        .and_then(|_| conn.execute_batch(include_str!("schema_v3.sql")))
        .and_then(|_| conn.execute_batch(&format!("PRAGMA user_version = {SCHEMA_VERSION};")))
        .map_err(|e| AppError::DatabaseError(format!("数据库初始化失败：{e}")))
}

// ---------------------------------------------------------------------------
// v2 data migration
// ---------------------------------------------------------------------------

fn up_v2_hook(tx: &rusqlite::Transaction<'_>) -> HookResult {
    backfill_category_accounts(tx)?;
    Ok(())
}

/// Bind every category to exactly one P&L account, in both directions.
///
/// Runs inside the migration transaction: any error rolls the whole upgrade
/// back, leaving the pre-migration `.bak-*` file as the only recovery path.
///
/// Why this exists: before v2 the link between a category and its account was
/// implicit (`accounts.type + accounts.name`). Renaming a category left the old
/// account behind, and the next quick-add created a second one. The backfill
/// makes the link explicit, so `categories.account_id` becomes the only truth.
fn backfill_category_accounts(conn: &Connection) -> rusqlite::Result<()> {
    let now = crate::utils::time::now_rfc3339();

    // ① category -> account: adopt an unclaimed account of the same
    //    (type, name) or create one. A missing account is the common legacy
    //    case (categories created before they were ever used).
    let pending: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT id, name, type FROM categories WHERE account_id IS NULL ORDER BY type, name",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    for (category_id, name, category_type) in pending {
        let existing: Option<String> = conn
            .query_row(
                "SELECT id FROM accounts WHERE type = ?1 AND name = ?2",
                params![&category_type, &name],
                |row| row.get(0),
            )
            .optional()?;

        let account_id = match existing {
            Some(id) => id,
            None => {
                let id = Uuid::new_v4().to_string();
                insert_category_account(conn, &id, &name, &category_type, &now)?;
                id
            }
        };

        conn.execute(
            "UPDATE categories SET account_id = ?1 WHERE id = ?2",
            params![&account_id, &category_id],
        )?;
    }

    // ② account -> category: a P&L account nobody points at is an orphan from
    //    the old name-matching code. Step ① already claimed every account whose
    //    (type, name) matches a category, so whatever is left has no category
    //    of that name — creating one cannot violate UNIQUE(type, name).
    let orphans: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare(
            "SELECT a.id, a.name, a.type
               FROM accounts a
              WHERE a.type IN ('income', 'expense')
                AND NOT EXISTS (SELECT 1 FROM categories c WHERE c.account_id = a.id)
              ORDER BY a.type, a.name",
        )?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };

    for (account_id, name, account_type) in orphans {
        conn.execute(
            "INSERT INTO categories (id, name, type, icon, account_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?5)",
            params![
                Uuid::new_v4().to_string(),
                &name,
                &account_type,
                &account_id,
                &now
            ],
        )?;
    }

    Ok(())
}

/// Insert the mirror account for a P&L category.
///
/// `include_net_worth` is 0 on purpose: a P&L account is not an asset and must
/// never enter net-worth totals.
fn insert_category_account(
    conn: &Connection,
    id: &str,
    name: &str,
    account_type: &str,
    now: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO accounts
             (id, name, type, currency, description, is_system, is_active,
              include_net_worth, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'CNY', '', 0, 1, 0, ?4, ?4)",
        params![id, name, account_type, now],
    )?;
    Ok(())
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
        .map_err(|e| AppError::DatabaseError(format!("数据库迁移失败，原文件已保留备份：{e}")))?;

    // `to_latest` is the only writer of `user_version`; if it ever stops
    // matching the constant the next open would try to migrate all over again.
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|e| AppError::DatabaseError(format!("无法读取数据库版本：{e}")))?;
    if version != SCHEMA_VERSION {
        eprintln!("[glimmerx] schema version {version}, expected {SCHEMA_VERSION}");
        return Err(AppError::DatabaseError(crate::errors::message(
            "errors.databaseError",
        )));
    }
    Ok(())
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
    use sha2::{Digest, Sha256};
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

    /// `schema_v1.sql` is frozen: it already shipped in 0.1.2 and every existing
    /// database is stamped `user_version = 1`.
    ///
    /// Editing it would give fresh installs a different schema than upgraded
    /// ones — and because the file is only ever applied once, nothing else would
    /// notice. If this test fails you edited a released baseline: revert it and
    /// add `M::up(include_str!("schema_v2.sql"))` instead.
    #[test]
    fn test_schema_v1_is_frozen() {
        const EXPECTED_SHA256: &str =
            "50c32c6b37c886ef028a26a169ca95e0b535dd37e3d072ef2da8efc8897d0b01";
        // Normalize line endings so a CRLF checkout does not trip the guard.
        let normalized = include_str!("schema_v1.sql").replace("\r\n", "\n");
        let digest = hex::encode(Sha256::digest(normalized.as_bytes()));
        assert_eq!(
            digest, EXPECTED_SHA256,
            "schema_v1.sql is frozen (shipped in 0.1.2). Add a new M::up migration \
             instead of editing the released baseline"
        );
    }

    #[test]
    fn test_legacy_database_migrated_and_data_preserved() {
        let (dir, path, mut conn) = legacy_db();
        migrate(&mut conn, &path, false).unwrap();

        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);

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

        let hashes: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'transaction_hashes'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hashes, 0, "import bookkeeping table is dropped in v2");

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
            assert_eq!(version, SCHEMA_VERSION);
        }
    }

    /// Build a real v1 database (schema only, stamped v1) so the v1 -> v2 step
    /// can be exercised without `init_baseline` fast-forwarding to v2.
    fn v1_db() -> (TempDir, PathBuf, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("v1.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;",
        )
        .unwrap();
        conn.execute_batch(include_str!("schema_v1.sql")).unwrap();
        conn.execute_batch("PRAGMA user_version = 1;").unwrap();
        (dir, path, conn)
    }

    fn insert_account(conn: &Connection, id: &str, name: &str, kind: &str) {
        conn.execute(
            "INSERT INTO accounts (id, name, type, currency, description, is_system,
                                   is_active, include_net_worth, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'USD', '', 0, 1, 0, '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            rusqlite::params![id, name, kind],
        )
        .unwrap();
    }

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn test_migrate_v1_to_v3() {
        let (_dir, path, mut conn) = v1_db();

        // A bound pair (category 餐饮 with an account of the same name), an
        // orphan expense account (a renamed category left it behind), and an
        // income category that never had an account.
        insert_account(&conn, "acc-bank", "招商银行", "asset");
        insert_account(&conn, "acc-food", "餐饮", "expense");
        insert_account(&conn, "acc-orphan", "吃饭", "expense");
        conn.execute(
            "INSERT INTO categories (id, name, type, created_at, updated_at)
             VALUES ('cat-food', '餐饮', 'expense', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'),
                    ('cat-salary', '工资', 'income', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z')",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO transactions (id, date, description, category_id, is_reconciled,
                                       deleted_at, created_at, updated_at)
             VALUES ('tx-1', '2024-02-01', '午饭', 'cat-food', 1, '2024-03-01T00:00:00Z',
                     '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z'),
                    ('tx-2', '2024-02-02', '打车', 'cat-food', 0, NULL,
                     '2024-02-02T00:00:00Z', '2024-02-02T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p-1', 'tx-1', 'acc-food', 3500, 0, '2024-02-01T00:00:00Z'),
                    ('p-2', 'tx-1', 'acc-bank', -3500, 1, '2024-02-01T00:00:00Z'),
                    ('p-3', 'tx-2', 'acc-food', 1200, 0, '2024-02-02T00:00:00Z'),
                    ('p-4', 'tx-2', 'acc-bank', -1200, 1, '2024-02-02T00:00:00Z')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO transaction_hashes (transaction_id, content_hash, imported_at, created_at)
             VALUES ('tx-1', 'deadbeef', '2024-02-01T00:00:00Z', '2024-02-01T00:00:00Z')",
            [],
        )
        .unwrap();

        migrate(&mut conn, &path, false).unwrap();

        assert_eq!(count(&conn, "PRAGMA user_version"), 3);

        // Every category is bound, and no account is claimed twice.
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM categories WHERE account_id IS NULL"
            ),
            0
        );
        assert_eq!(
            count(&conn, "SELECT COUNT(DISTINCT account_id) FROM categories"),
            3
        );

        // 餐饮 kept its own account; 工资 got one created on the fly.
        let bound: String = conn
            .query_row(
                "SELECT account_id FROM categories WHERE id = 'cat-food'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(bound, "acc-food");
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM accounts WHERE type = 'income' AND name = '工资'"
            ),
            1
        );

        // The orphan account is reachable again from the category page.
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM categories WHERE name = '吃饭'"),
            1
        );

        // Reconciliation moved from the transaction to its postings.
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM postings WHERE cleared = 1"),
            2
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM postings WHERE reconcile_id IS NOT NULL"
            ),
            0
        );

        // Single currency, hard delete.
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM accounts WHERE currency <> 'CNY'"
            ),
            0
        );
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(transactions)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(!columns.contains(&"deleted_at".to_string()));
        assert!(!columns.contains(&"is_reconciled".to_string()));
        // v3: the cached category is gone, categories come from the postings.
        assert!(!columns.contains(&"category_id".to_string()));

        // Rows survive the column drops.
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM transactions"), 2);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM postings"), 4);
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'transaction_hashes'"
            ),
            0
        );

        // New tables exist.
        for table in ["reconciliations", "savings_goals"] {
            assert_eq!(
                count(
                    &conn,
                    &format!("SELECT COUNT(*) FROM sqlite_master WHERE name = '{table}'")
                ),
                1
            );
        }
    }
}
