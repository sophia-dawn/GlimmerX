-- Schema version 2: double-entry upgrade.
--
-- Applied exactly once, by `db/migrations.rs`, inside a single transaction; the
-- Rust half of the migration (category <-> account backfill) runs as a hook
-- afterwards. Never edit this file once released — add schema_v3 instead.
--
-- Ordering constraints that are easy to get wrong:
--   * `postings.cleared` must be added *before* it is backfilled from
--     `transactions.is_reconciled`, and that backfill must happen before the
--     old column is dropped.
--   * `idx_transactions_date_deleted` must be dropped before `deleted_at`:
--     SQLite refuses DROP COLUMN while an index still references it.
--   * The view stays valid across the DROP COLUMNs because it never mentions
--     `transactions`.

-- ---------------------------------------------------------------------------
-- ① 对账：粒度下沉到分录
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS reconciliations (
    id               TEXT PRIMARY KEY,
    account_id       TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    statement_date   TEXT NOT NULL,
    opening_balance  INTEGER NOT NULL,
    closing_balance  INTEGER NOT NULL,
    created_at       TEXT NOT NULL,
    UNIQUE(account_id, statement_date)
);

ALTER TABLE postings ADD COLUMN cleared INTEGER NOT NULL DEFAULT 0
    CHECK (cleared IN (0, 1));
ALTER TABLE postings ADD COLUMN reconcile_id TEXT REFERENCES reconciliations(id);

-- 旧的整笔对账标记平移到分录。没有对账单记录，所以 reconcile_id 留空：
-- 界面显示为"已核对（无对账单）"，后续仍可纳入正式对账。
UPDATE postings
   SET cleared = 1
 WHERE transaction_id IN (SELECT id FROM transactions WHERE is_reconciled = 1);

-- ---------------------------------------------------------------------------
-- ② 数据归一：单币种
-- ---------------------------------------------------------------------------

UPDATE accounts SET currency = 'CNY' WHERE currency <> 'CNY';

-- ---------------------------------------------------------------------------
-- ③ 分类 ↔ 损益账户 1:1（account_id 由迁移 hook 回填）
-- ---------------------------------------------------------------------------

ALTER TABLE categories ADD COLUMN account_id TEXT REFERENCES accounts(id);

-- 部分唯一索引：NULL（尚未绑定）不参与唯一性，绑定后一个账户只对应一个分类。
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_account
    ON categories(account_id) WHERE account_id IS NOT NULL;

-- ---------------------------------------------------------------------------
-- ④ 储蓄目标：以专用资产账户为载体
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS savings_goals (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    account_id     TEXT NOT NULL UNIQUE REFERENCES accounts(id) ON DELETE CASCADE,
    target_amount  INTEGER NOT NULL CHECK (target_amount > 0),
    target_date    TEXT,
    monthly_plan   INTEGER,
    status         TEXT NOT NULL DEFAULT 'active'
                   CHECK (status IN ('active', 'achieved', 'archived')),
    created_at     TEXT NOT NULL,
    updated_at     TEXT NOT NULL
);

-- ---------------------------------------------------------------------------
-- ⑤ 分录维度的分类视图（v3 报表迁移用，v2 不动报表）
-- ---------------------------------------------------------------------------

CREATE VIEW IF NOT EXISTS v_posting_category AS
SELECT p.id            AS posting_id,
       p.transaction_id,
       p.account_id,
       p.amount,
       p.cleared,
       c.id            AS category_id
FROM postings p
JOIN accounts   a ON a.id = p.account_id
LEFT JOIN categories c ON c.account_id = a.id;

-- ---------------------------------------------------------------------------
-- ⑥ 硬删除：移除软删与整笔对账的残留列
-- ---------------------------------------------------------------------------

DROP INDEX IF EXISTS idx_transactions_date_deleted;
ALTER TABLE transactions DROP COLUMN deleted_at;
ALTER TABLE transactions DROP COLUMN is_reconciled;

-- 仅 CSV 导入去重使用，导入导出已下线。
DROP TABLE IF EXISTS transaction_hashes;

-- ---------------------------------------------------------------------------
-- ⑦ 新索引
-- ---------------------------------------------------------------------------

CREATE INDEX IF NOT EXISTS idx_postings_reconcile ON postings(reconcile_id);
CREATE INDEX IF NOT EXISTS idx_postings_account_cleared
    ON postings(account_id, cleared);
CREATE INDEX IF NOT EXISTS idx_reconciliations_account
    ON reconciliations(account_id, statement_date);
