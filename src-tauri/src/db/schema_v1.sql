-- Baseline schema for schema version 1 (GlimmerX 0.1.2).
--
-- FROZEN: this file has already shipped in 0.1.2 and is recorded as
-- `user_version = 1` on every existing database. Do NOT edit it — a column
-- added here would appear for fresh installs but never for upgraded ones,
-- silently producing two different schemas. Any change belongs in a new
-- `M::up(include_str!("schema_v2.sql"))` step in db/migrations.rs.
-- tests::test_schema_v1_is_frozen fails on the first edit on purpose.
--
-- Every statement is idempotent, so applying this as migration #1 to an
-- already-released 0.1.2 database only fills in missing objects and never
-- touches existing data. Connection-level PRAGMAs are applied separately by
-- db::migrations::migrate (they cannot run inside a migration transaction).

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
-- deleted_at 字段为未来软删除/撤销扩展保留。
-- 当前实现为硬删除（DELETE），所有统计查询仍过滤 deleted_at IS NULL 以保证一致性。
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
