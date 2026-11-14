# 21 - 复式记账能力升级（schema v2 方案）

> 返回 [DESIGN.md](../DESIGN.md)

本文对应 8 条产品决策的落地方案。所有涉及表结构的改动统一进 **schema v2**，
`src-tauri/src/db/schema_v1.sql` 保持冻结不动（见 `test_schema_v1_is_frozen`）。

**状态：批 A / B / C 均已实现**（2026-11-13），`user_version = 3`。

## 0. 决策摘要

| # | 需求 | 落地动作 | 批次 | 动 schema |
| --- | --- | --- | --- | --- |
| 1 | 不做多币种 | 删除币种输入与选择器，全库固定 CNY，数据归一 | A + v2 | 是（UPDATE） |
| 2 | 保留支出账户 | 分类 ↔ 损益账户 **1:1 绑定**，账本真相唯一定义在 posting | A + v2 | 是 |
| 3 | 完善对账 | `postings.cleared` + `reconciliations`，对账粒度下沉到分录，**首版带自动匹配** | v2 | 是 |
| 4 | 封禁导入导出 | 删代码、命令、依赖、UI、i18n、文档 | A | 是（drop 表） |
| 5 | 自动锁定 | 空闲计时 + 失焦锁定 + 清空查询缓存 | A | 否 |
| 6 | 储蓄目标 | `savings_goals`，以专用资产账户为载体 | v2 | 是 |
| 7 | 只做硬删除 | 移除 `deleted_at` 软删语义与残留过滤 | v2 | 是（drop 列） |
| 8 | 全局快捷键 | 接线已有 `useGlobalShortcut` → 唤起快速记账 | A | 否 |

---

## 1. 核心架构决策：账户是账本，分类是账户的展示层

### 1.1 现状问题

`categories` 与 `accounts` 是两套并行维度，靠**名字**隐式关联：

```rust
// db/transactions/quick_add.rs
fn ensure_category_account(conn, category_id) -> account_id {
    // ① 按 name + type 找账户，找不到就新建一个
}
```

而 `db/categories.rs::update_category` 只更新 `categories.name`，不碰账户。后果：

- 分类「餐饮」改名为「吃饭」→ 下次记账又建一个「吃饭」账户，旧「餐饮」账户带着历史余额留在账户树里，成为孤儿；
- 账户名与分类名一旦不同步，任何按名字的匹配都开始产生新账户，随时间累积垃圾。

### 1.2 标准参照

| 产品 | 模型 |
| --- | --- |
| GnuCash / Beancount / Ledger | **没有"分类"**，收入/支出就是账户，一切按账户记账 |
| Firefly III | accounts（含 expense/income）+ categories，但 categories 是**独立标签维度**，不与账户绑定，因此不存在镜像 |
| YNAB / Actual | 单式记账 + 信封，不涉及账户体系 |

我们既要求复式（必须保留损益账户），又已经在用分类做了报表/预算/UI，
所以既不能删账户，也不该删分类。**正确解法是让两者成为一对一，去掉"按名字猜"的隐式关联。**

### 1.3 方案：1:1 绑定，账本真相在 posting

- `categories.account_id` → 指向唯一的同名损益账户，`UNIQUE INDEX` 保证一对一；
- 创建分类 = 同一事务内建分类行 + 账户；
- 重命名 = 同一事务内改两张表的 name；
- 删除分类 = 仅当该账户无 posting，有则提示交易笔数；
- `ensure_category_account` 改为读 `categories.account_id`，**不再按名字匹配** → 漂移根除；
- 反向补齐：没有对应分类的损益账户，迁移时自动建一个分类，保证双向完备。

```rust
// 改造后
fn category_account(conn: &Connection, category_id: &str) -> Result<String, AppError> {
    conn.query_row(
        "SELECT account_id FROM categories WHERE id = ?1",
        [category_id],
        |r| r.get::<_, Option<String>>(0),
    )?
    .ok_or_else(|| AppError::NotFound("errors.categoryNotBound".into()))
}
```

### 1.4 为什么这样最省

| 方案 | 改动量 | 解决漂移 | 支持拆分 |
| --- | --- | --- | --- |
| 删掉 categories（账户即分类） | 后端 718 处 + 前端 312 处引用 | 是 | 是 |
| 删掉损益账户 | 违反复式，不可行 | — | — |
| **1:1 绑定（本方案）** | 新增 1 列 + 5 个函数改造 | 是 | 是（见 1.5） |

### 1.5 拆分交易怎么做（不新增字段）

分类既然 1:1 账户，**拆分 = 一笔交易多条损益分录**，`TransactionForm` 现有
「+ 添加分录」已经支持任意条数，后端 `create_transaction` 只校验合计为零。
所以拆分不需要给 `postings` 加 `category_id`。

v2 建一个视图作为过渡，让报表可以逐步从 `transactions.category_id` 切到分录维度：

```sql
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
```

> v2 阶段 `transactions.category_id` **保留**，作为"未拆分交易的主分类"缓存，
> 报表暂时不动；v3 已移除该列，并把 11 处分类聚合改走分录（实现细节与「为什么不切
> 视图」见 §10 批 C）。

---

## 2. 对账：粒度必须下沉到分录

### 2.1 现状问题

`transactions.is_reconciled` 是**整笔交易**级别，且 `src/` 里零引用 —— 有字段没功能。
更根本的是粒度错了：一笔转账 A→B，可能 A 账户的银行流水已核对、B 账户还没有，
按整笔标记必然出错。

### 2.2 标准做法（GnuCash）

- 对账单位是 **split（分录）**，状态 `cleared`；
- 每次对账生成一条 reconcile 记录：账户、对账单日期、期初余额、期末余额；
- 差额 = 期末余额 −（期初已对账余额 + 本次勾选合计），必须为 0 才能完成；
- 完成后该批分录锁定，改/删需先"撤销对账"；
- 账户余额区分 **cleared / uncleared / total** 三个口径。

### 2.3 v2 DDL

```sql
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

CREATE INDEX IF NOT EXISTS idx_postings_reconcile
    ON postings(reconcile_id);
CREATE INDEX IF NOT EXISTS idx_postings_account_cleared
    ON postings(account_id, cleared);
```

### 2.4 交互流程

1. 账户详情（仅 asset / liability）→「对账」；
2. 填对账单日期 + 期末余额（手填，来自银行/信用卡/支付平台账单）；
3. 列出该账户 `date <= 对账日 AND cleared = 0` 的分录，按日期升序；
4. 录入账单金额列表 → **自动匹配预勾选**（金额相等 + 日期就近，规则见 12.3），
   用户逐条确认或纠正；底部实时显示：

   ```
   期初已对账余额  ¥3,200.00
   本次勾选        ¥1,850.00
   对账余额        ¥5,050.00
   账单余额        ¥5,050.00   差额 ¥0.00
   ```

5. 差额为 0 才允许提交；`opening_balance` 取上一次对账的 `closing_balance`（首次为 0 或首笔分录前余额）；
6. 提交 = 写 `reconciliations` + 把勾选分录 `cleared = 1, reconcile_id = <id>`（单事务）。

### 2.5 锁定语义（后端强制）

- `update_transaction` / `delete_transaction` / 分录增删改：
  若涉及 `reconcile_id IS NOT NULL` 的分录 → 拒绝，返回
  `errors.reconcile.transactionLocked`（"该交易已对账，请先撤销对账"）；
- 撤销：`reconcile_undo(id)` → 分录 `cleared = 0, reconcile_id = NULL`，删除 reconcile 记录（同样单事务）；
- 报表侧：`AccountBalance` 增加 `clearedBalance` / `unclearedBalance`，账户页文案区分。

### 2.6 迁移

```sql
-- 老的整笔对账标记平移到分录
UPDATE postings SET cleared = 1
WHERE transaction_id IN (SELECT id FROM transactions WHERE is_reconciled = 1);
```

已对账但没有 reconcile 记录的历史数据：`reconcile_id` 留空、`cleared = 1`，
界面上显示为"已核对（无对账单）"，允许后续正式对账时再次纳入。

---

## 3. 储蓄目标：用资产账户当载体

### 3.1 设计取舍

储蓄目标有两种建模：

| 方式 | 优点 | 缺点 |
| --- | --- | --- |
| 独立"分配"表（YNAB 式） | 不动账本 | 与复式账本脱节，余额和真实账户对不上 |
| **专用资产账户 + 目标属性** | 进度 = 账户余额，天然复式，转入转出都是资产间转账 | 每个目标多一个账户 |

选后者：复式记账下"存钱"本来就是 `活期 → 旅行基金` 的资产转账，不产生收支。
这样零额外记账逻辑，账户余额自动就是进度。

### 3.2 v2 DDL

```sql
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
```

### 3.3 行为

- 创建目标：单事务内建资产账户 `资产/储蓄目标/<名称>`（`include_net_worth = 1`）+ 目标行；
- 进度 = 该账户余额（`SUM(postings.amount)`，资产类为正）；
- 目标卡片显示：进度条、已存/目标、"还差 X"、"每月需存 Y"（按 `target_date` 折算）；
- 「存入 / 取出」按钮直接调 `quick_add` 的 `transfer` 模式（来源账户 ↔ 目标账户），不新增接口；
- 余额 ≥ 目标 → `status = 'achieved'`（读取时判定并回写，不做定时任务）；
- 删除目标：先校验账户余额为 0，否则要求先转出；归档 = `archived`（账户可继续存在）。

---

## 4. 自动锁定

### 4.1 现状

`db_lock` 命令已存在（checkpoint + 丢弃 `Database`，连接关闭、SQLCipher 密钥随之释放），
但没有任何自动触发。桌面端离开工位时，加密库长期以明文驻留内存。

### 4.2 方案（已实现，批 A）

配置存在 **localStorage**（`securityStore`），不是 `settings` 表：

| key | 默认 | 说明 |
| --- | --- | --- |
| `glimmerx:security:autoLockMinutes` | `5` | 0 = 关闭 |

> 取舍说明：这项是**设备行为**，不是账本数据。放进 `settings` 表会带来两个副作用 ——
> 计时器要等异步读取完成才能启动；打开另一个账本会悄悄改掉这台机器的锁定习惯。
> localStorage 让配置在解锁前就可用，且天然按设备隔离。

`hooks/useAutoLock.ts`：

- `pointerdown / keydown / wheel / touchstart` 重置 `lastActivity`（节流 1s）；
- 每 10s 一次 tick，判定逻辑抽成纯函数 `evaluateIdle(now, lastTick, lastActivity, minutes)`：
  返回 `idle`（空闲到点）、`suspend`（tick 迟到 > 2×间隔，说明系统休眠过）或 `none`；
- 锁定序列统一走 `utils/session.ts::lockSession()`：
  `await dbLock()` → `queryClient.clear()` → 关闭快速记账弹窗 → `setLocked()`；
  **顺序不能反**，否则跳转后短暂仍能读到缓存里的金额。
  侧边栏锁按钮、改密流程原本各写了一遍这个序列（且都漏了 `queryClient.clear()`），已一并收口；

`UnlockPage` 已复核：不渲染任何余额、账户名或交易信息。

---

## 5. 全局快捷键

`tauri-plugin-global-shortcut` 已在 `lib.rs` 注册，`capabilities/default.json` 已放行，
`useGlobalShortcut` 已写好但**没有任何调用点**。

已实现（批 A）：

- 默认 `CommandOrControl+Shift+A`，存 `glimmerx:security:quickAddShortcut`（localStorage）；
- 注册在 `App.tsx` 的 `GlobalQuickAddShortcut` —— 位于路由之上，因此**未解锁时也生效**：
  先 `unminimize/show/setFocus` 把窗口带到前台，已解锁才打开快速记账弹窗。
  快捷键的最小用途是「把最小化的应用叫回来」，锁屏状态下这恰恰是最需要的；
- 快速记账的开关状态从 `AppShell` 局部 state 提到 `uiStore`，否则路由之上的快捷键无法打开它；
- 注册失败（被其他应用占用）→ toast「快捷键 X 注册失败」；
- 设置页 `ShortcutRecorder`：点击进入录制 → 捕获阶段拦截按键（`preventDefault` +
  `stopPropagation`，避免录制时立刻触发旧快捷键）→ 只接受带修饰键的组合；
- `utils/shortcut.ts` 提供 `matchesAccelerator`，让**同一个**配置在窗口内也能命中
  （全局注册失败或浏览器调试时的兜底）。原先写死的 `Ctrl+Shift+N` 已删除 ——
  两个不同组合做同一件事必然对不上。

---

## 6. 单币种清理

保留 DB 列（v1 冻结 + 重建表成本高于收益），但**所有入口收敛为固定 CNY**：

| 位置 | 动作 |
| --- | --- |
| `AccountWizard.tsx` | 删除币种步骤与 `CURRENCY_SYMBOLS` 多值选择 |
| `AccountFields.tsx` | 删除只读币种展示 |
| `CreateAccountInput` / `UpdateAccountInput` | 删除 `currency` 字段，后端固定写 `"CNY"` |
| `AccountRecord` / `AccountDto` | 删除 `currency` 字段（SELECT 不再读该列，INSERT 写死 `'CNY'`） |
| `commands/accounts.rs` | 去掉 `input.currency.unwrap_or("CNY")` |
| v2 迁移 | `UPDATE accounts SET currency = 'CNY' WHERE currency <> 'CNY'` |
| 文档 / i18n | 删除"多币种"描述与键 |

> 判定依据：`reports/*` 与 `dashboard.rs` 汇总时从未读取 `currency`，
> 即"支持多币种"从未真正成立，直接清理不会丢失任何能力。

---

## 7. 导入导出下线

保留**数据库备份**（`db_backup`），它属于安全兜底、不是导入导出；其余全部移除。

| 层 | 删除对象 |
| --- | --- |
| 后端 | `db/import.rs`、`db/export.rs`、`commands/data.rs` 的 `export_transactions_csv` / `export_transactions_beancount` / `import_transactions_csv` 及其测试、`db/mod.rs` 的 re-export、`lib.rs` 的 handler 注册 |
| 依赖 | `csv = "1.3"`（仅这两处使用）；`sha2` 保留（迁移测试的 schema 冻结哈希在用） |
| 前端 | `src/utils/export.ts`、`DataManagementSection` 的导入/导出区块、`api.ts` 三个函数、`types` 中相关类型、i18n `data.*` 导入导出键 |
| 数据库 | v2 里 `DROP TABLE IF EXISTS transaction_hashes`（仅导入去重用） |
| 文档 | `19-data-export.md` / `20-data-import.md` 标注「已下线」，roadmap 相应条目改为已移除 |

---

## 8. 硬删除

产品语义已经是硬删（`DELETE`），但代码里残留软删痕迹：95 处 `deleted_at` / `is_reconciled`
引用，大量 `AND t.deleted_at IS NULL` 是永远为真的噪音，容易让人误以为有回收站。

v2 一并清理：

```sql
ALTER TABLE transactions DROP COLUMN is_reconciled;  -- 已平移到 postings.cleared
ALTER TABLE transactions DROP COLUMN deleted_at;
```

配套：

- 删除所有 `deleted_at IS NULL` 条件（`accounts/crud.rs`、`budgets.rs`、`categories.rs` 等）；
- 删除 `TransactionRecord.deleted_at` 字段与前后端类型；
- 手写 schema 的测试夹具（约 20 处 `INSERT INTO transactions (... deleted_at ...)`）同步改列；
- SQLite `DROP COLUMN` 要求 3.35+，本项目 bundled SQLite 远高于此，且两列无索引/触发器依赖。

> 代价：约 30 处测试夹具要改列名。收益：语义唯一（删了就是没了），
> 且不再有人问"为什么有 deleted_at 却查不到回收站"。

---

## 9. schema v2 全量 DDL（已落地）

实际文件：`src-tauri/src/db/schema_v2.sql`。下面的顺序是**必须**的，两处顺序错误
在实现时都真实触发过报错（见 §9.1）：

```sql
-- ① 对账：粒度下沉到分录
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

-- 平移整笔对账标记（必须在 DROP COLUMN is_reconciled 之前）
UPDATE postings SET cleared = 1
 WHERE transaction_id IN (SELECT id FROM transactions WHERE is_reconciled = 1);

-- ② 数据归一：单币种
UPDATE accounts SET currency = 'CNY' WHERE currency <> 'CNY';

-- ③ 分类 ↔ 损益账户 1:1
ALTER TABLE categories ADD COLUMN account_id TEXT REFERENCES accounts(id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_account
    ON categories(account_id) WHERE account_id IS NOT NULL;

-- ④ 储蓄目标
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

-- ⑤ 分录维度的分类视图（v3 报表迁移用，v2 不动报表）
CREATE VIEW IF NOT EXISTS v_posting_category AS
SELECT p.id AS posting_id, p.transaction_id, p.account_id, p.amount, p.cleared,
       c.id AS category_id
  FROM postings p
  JOIN accounts a ON a.id = p.account_id
  LEFT JOIN categories c ON c.account_id = a.id;

-- ⑥ 硬删除残留列（索引先删，否则 DROP COLUMN 被拒）
DROP INDEX IF EXISTS idx_transactions_date_deleted;
ALTER TABLE transactions DROP COLUMN deleted_at;
ALTER TABLE transactions DROP COLUMN is_reconciled;
DROP TABLE IF EXISTS transaction_hashes;   -- 仅 CSV 导入去重用，已下线

-- ⑦ 新索引
CREATE INDEX IF NOT EXISTS idx_postings_reconcile ON postings(reconcile_id);
CREATE INDEX IF NOT EXISTS idx_postings_account_cleared ON postings(account_id, cleared);
CREATE INDEX IF NOT EXISTS idx_reconciliations_account ON reconciliations(account_id, statement_date);
```

需要 Rust 侧逻辑的部分走 `M::up_with_hook(schema_v2, up_v2_hook)`（带 hook 的迁移
无法 `const`，因此 `MIGRATIONS_SLICE` 改为 `static MIGRATIONS: LazyLock<Migrations>`）：

- 分类回填：按 `type + name` 匹配账户 → 命中则写 `account_id`；
  未命中则建账户（uuid、`include_net_worth = 0`）；
- 反向补齐：`type IN ('income','expense')` 且无分类指向的账户 → 建对应分类。

整个 hook 在迁移事务内执行，失败即整体回滚（`rusqlite_migration` 保证），
且 `db/migrations.rs` 已有的 `*.db.bak-YYYYmmdd-HHMMSS` 备份仍在迁移前落盘。

### 9.1 实现踩到的顺序 / 细节坑

| 现象 | 原因 | 处理 |
| --- | --- | --- |
| `no such column: cleared` | `UPDATE postings SET cleared = 1` 写在了 `ADD COLUMN` 之前 | 先建表加列，再回填，最后删旧列 |
| `error in index idx_transactions_date_deleted` | SQLite 不允许删除被索引引用的列 | 迁移开头 `DROP INDEX IF EXISTS` |
| `8 values for 6 columns` | `deleted_at`/`is_reconciled` 删列后 `create_transaction` 仍按 8 个占位符绑定 | 改 `(?1,?2,?3,?4,?5,?5)`，`TRANSACTION_COLUMNS` 减为 6 列 |
| `near "AND": syntax error` | 删掉 `deleted_at` 过滤后 `list_transactions` 的 `AND` 拼在没有 `WHERE` 的语句后 | 基础 SQL 加 `WHERE 1 = 1` |
| 夹具 `UNIQUE constraint failed: accounts.type, name` | 分类镜像账户与手写夹具同名 | 所有夹具改走 `crate::db::init_schema`，且建账户时先查后建 |

`insert_category_account` 与 `backfill_category_accounts` 都是幂等的，`init_baseline`
（测试夹具入口）直接跑 v1 + v2 + 回填 + v3 并写 `user_version = SCHEMA_VERSION`
（批 C 之后为 3），避免手写 DDL 与真实 schema 漂移。

---

## 10. 交付顺序与验收

### 批 A — 无 schema 变更（✅ 2026-11-13 完成）

1. 单币种清理（第 6 节）—— 领域层彻底移除 `currency` 字段，DB 列仅写 `'CNY'`；
2. 导入导出下线（第 7 节）—— `db/import.rs`、`db/export.rs`、3 个命令、`csv` 依赖全部删除；
3. 自动锁定（第 4 节）—— `useAutoLock` + `lockSession`；
4. 全局快捷键（第 5 节）—— `GlobalQuickAddShortcut` + 可录制改键。

验收：`cargo test` 369 passed、前端 98 passed、`make check` 全绿；
手动验证：闲置 5 分钟回解锁页；`Ctrl+Shift+A` 唤起快速记账；账户向导无币种项；数据管理页只剩备份。

### 批 B — schema v2（✅ 2026-11-13 完成）

5. 分类 1:1 绑定 + 迁移回填；
6. 对账（表 + 命令 + 界面 + 锁定校验）；
7. 储蓄目标（表 + 命令 + 页面）；
8. 硬删除清理；
9. **账户页收敛（决策 1）**：列表只出 `asset / liability`，向导不再提供损益类型，
   分类页展示并管理对应的损益账户。

> 第 9 项**必须与第 5 项同批**：在 `categories.account_id` 落地之前先隐藏损益账户，
> 用户会陷入「建得出来、看不到、也改不了」的中间态。

验收：`cargo test` **400 passed**、`vitest` **99 passed**、`make check` 全绿、
行覆盖率 **94.46%**（门禁 94%）。

落地清单：

| 项 | 主要落点 |
| --- | --- |
| 迁移 | `db/schema_v2.sql`、`db/migrations.rs`（`MIGRATIONS` 改 `LazyLock`、`SCHEMA_VERSION = 2`、`up_v2_hook`、`backfill_category_accounts`、`init_baseline`） |
| 分类绑定 | `db/categories.rs`（`bind_account` / `account_id_for_category` / 改名时同步账户名 / 删除时校验分录引用）、`commands/categories.rs`（DTO 增 `accountId` + `balance`，余额一次聚合查询） |
| 对账 | `db/reconcile.rs`、`commands/reconcile.rs`（6 命令）、`components/accounts/ReconcileDialog.tsx` |
| 储蓄目标 | `db/savings.rs`、`commands/savings.rs`（5 命令）、`pages/SavingsPage.tsx` |
| 硬删除 | 删 `transactions.deleted_at` / `is_reconciled` 及全库 ~30 处过滤（报表、dashboard、预算、分页、审计） |
| 账户页收敛 | `AccountsPage` 只查 `["asset","liability"]`、`AccountWizard` 去掉损益类型、`CategoryList` 显示分类余额 |

与方案的差异与取舍：

- **对账总额一律由后端算**（`reconcile_totals`）：勾选变化即重新请求，前端不参与加法，
  提交时用同一份整数再校验一次，避免「界面显示平了、写库时不为零」。
- **`cleared` 平移不生成对账单**：旧 `is_reconciled = 1` 的分录只置 `cleared = 1`，
  `reconcile_id` 留空，界面显示「已核对（无对账单）」，之后仍可纳入正式对账。
- **储蓄目标删除区分两种拒绝**：余额非 0 → `errors.savings.accountNotEmpty`；
  有分录历史 → `errors.savings.goalHasHistory`（提示改归档）。硬删账户会触发外键约束，
  所以必须显式拦截。
- **目标状态不自动回退**：取出后余额低于目标，仍保持 `achieved`，由用户手动改回
  `active`——自动翻转会让「达成」变成闪烁的状态。
- **`errors.*` 新增 13 个键**（`reconcile.*` 7 个、`savings.*` 5 个、`categoryAccountInUse`），
  `errors/messages.rs` 由 `tools/gen_error_messages.py` 重新生成（73 键）。

### 批 C — schema v3（✅ 2026-11-13 完成）

10. schema v3：删除 `transactions.category_id`；
11. **分类聚合切到分录维度**（下面「与方案的口径修正」一节说明为什么不是「切视图」）；
12. 拆分交易 UI：去掉交易级「分类」下拉，每行分录按「账户 / 分类」分组选择；
13. 列表与详情从分录派生分类（`categoryNames`），筛选改 `EXISTS`。

验收：`cargo test` **402 passed**、`vitest` **99 passed**、`make check` 全绿、
行覆盖率 **94.67%**（门禁 94%）。

> 发版前复审又改了一轮（第 13 节），当前基线是 `cargo test` **413 passed**、
> `vitest` **104 passed / 12 files**。

改动的 SQL 一共 11 处，全部从 `transactions.category_id` 改走
`postings.account_id → categories.account_id`：

| 位置 | 作用 |
| --- | --- |
| `dashboard::get_category_breakdown` | 首页分类占比 |
| `dashboard::get_top_expenses` | 首页大额支出 |
| `reports::account`（明细行） | 账户流水对侧分类/对侧账户（改为 `GROUP_CONCAT`，拆分交易显示多个） |
| `reports::comparison` | 两月分类对比 |
| `reports::trend`（趋势过滤 / 分类分解 / 年汇总 top2） | 4 处 |
| `budgets::calculate_budget_status` / `list_budget_statuses` | 预算花费（拆分交易必须按分录分摊） |
| `reports::audit`（未分类判定） | 改为「没有任何损益分录」 |

拆分交易的三条硬证据（新增测试）：

- `test_split_transaction_lists_every_category`：一笔 `-300 / +100 / +200` 的交易
  在列表里 `categoryNames = ["Food", "Transport"]`，按任一分类筛选都能命中；
- `test_split_transaction_splits_across_categories`：分类分解得 100 / 200 而不是
  300 / 0，趋势报表按分类过滤得 200；
- `test_split_transaction_counts_toward_each_budget`：两个预算分别记 3000 和 2000。

#### 与方案的口径修正：不切视图，直接 join 基表

方案里写「v3 把报表切到 `v_posting_category` 视图」，实现后**没有照做**，原因：

- 视图输出的列是 `posting_id / transaction_id / account_id / amount / cleared /
  category_id`，而报表需要 `accounts.type`（区分收入/支出）、`categories.name` /
  `icon`（展示）。用它仍要再 join 两张表，等于把同一个 join 拆成两步；
- 视图与 `postings JOIN accounts JOIN categories` 完全等价，不存在性能或一致性收益；
- 真正要改的是**join 条件**（从 `c.id = t.category_id` 改成
  `c.account_id = p.account_id`），而不是套一层视图。

所以视图保留（v2 已发布，删它没有收益），但正式查询不依赖它；它的价值只剩
「一眼看出 posting → category 的对应关系」，可用于排查与手工查询。

#### 拆分交易 UI 的实际形态

表单早就支持任意条数分录（「+ 添加分录」），缺的是**分类选择的位置**：原来顶部
有一个交易级「分类」下拉，与分录里的账户下拉重复，而且一旦用户在顶部选了 A、
分录里选了 B 的账户，数据就自相矛盾。v3 删掉它，改为每行分录一个合并选择器：

- 资产/负债账户 → 归在「账户」分组，选中的是账户；
- 损益账户 → 归在「分类」分组，用分类的图标与名字显示，选中的仍是账户 id。

即「选分类」和「选损益账户」在 UI 上是同一件事的两种叫法，落库永远只有账户。

---

## 11. 风险

| 风险 | 缓解 |
| --- | --- |
| 对账锁定后用户无法改账 | 必须提供「撤销对账」入口，并在错误文案里直接指路 |
| v2 迁移不可逆 | 迁移前自动备份 `*.db.bak-*`；回滚 = 还原备份 + 装回旧版；hook 失败整事务回滚 |
| 分类回填产生重复账户 | 回填前按 `(type, name)` 去重，`idx_accounts_type_name` 已唯一；多余账户置 `is_active = 0` |
| 报表仍按 `transactions.category_id` 统计 | v2 保留该列，拆分交易只在"未拆分视图"提示；v3 切视图后彻底一致 |
| 自动锁误伤长表单录入 | 用户操作（输入/滚动）都算活动；默认 5 分钟，可关 |

---

## 12. 已确认决策（2026-11-13）

| # | 决策 | 结论 |
| --- | --- | --- |
| 1 | 账户页信息架构 | **账户页只展示 `asset / liability`**；`income / expense` 账户收进「分类」页统一管理，账户树不再出现与分类同名的支出账户 |
| 2 | 硬删除的列清理 | **接受**，v2 直接 `DROP COLUMN is_reconciled / deleted_at`，同步改约 30 处测试夹具 |
| 3 | 对账是否自动匹配 | **首版即带自动匹配**：按金额相等 + 日期就近勾选，手动勾选作为补充与纠正手段 |

三条决策对前文的修订见 12.1 ~ 12.3。

### 12.1 账户页只显示资产/负债

- `db/accounts.rs` 的列表查询增加 `type` 过滤参数；账户页固定传 `["asset", "liability"]`；
- 分类页的「关联账户」列展示 `categories.account_id` 对应的损益账户（余额 = 该账户 postings 合计），
  用户在分类页就能看到「餐饮今年花了多少」，不必切到账户页；
- 损益账户仍存在于 `accounts` 表与账户树 API 中（复式的账本真相不变），只是**默认 UI 入口改为分类页**；
- 迁移/查询/报表一律不受影响，纯展示层收敛；
- **实施批次：批 B**（与分类 1:1 绑定同批），原因见第 10 节第 9 项。

> 副作用：账户间转账的下拉框需要显式包含 `income/expense` 吗？不需要 ——
> 资产↔损益的流动本来就该记成收入/支出交易，而不是 transfer。转账目标账户限制为 `asset/liability`。

### 12.2 硬删除列清理照做

第 8 节方案不变。测试夹具改动集中在手写 `INSERT INTO transactions` 的位置，
用一次 `grep -rn "deleted_at\|is_reconciled" src-tauri/src` 定位后逐处清理，
不保留「只清代码、留列」的折中版本。

### 12.3 对账自动匹配

在第 2 节流程的第 3 步之后插入自动匹配，规则如下（按优先级，全部命中才勾选）：

1. **金额精确相等**：`abs(posting.amount − target) = 0`；
2. **日期就近**：候选按 `|julianday(posting.date) − julianday(对账日)|` 升序，
   优先取对账日之前最近的；同金额多条时按「日期由近到远」依次占用；
3. **一对一绑定**：一条候选分录最多匹配一次，已被自动勾选的不再参与后续匹配；
4. **不做跨币种/跨账户**：只在该账户的 `cleared = 0` 分录内匹配。

匹配只是**预勾选**，用户仍可取消任意一条 —— 自动匹配的价值是省掉 90% 的点击，
不是替用户做判断。差额不为 0 时按原逻辑阻止提交。

```rust
// db/reconcile.rs
pub struct AutoMatchResult {
    pub matched: Vec<String>,   // posting ids，已按日期升序
    pub unmatched_targets: usize,
}

/// 贪心匹配：金额相等优先，其次日期就近。纯函数，输入输出都在内存里，便于单测。
pub fn auto_match(
    candidates: &[CandidatePosting],  // 该账户 cleared = 0 的分录
    targets: &[i64],                  // 账单上的流水金额（已批量识别）
    statement_date: &str,
) -> AutoMatchResult { /* ... */ }
```

> 第一版不做「账单文件解析」（PDF/CSV/OFX 识别），`targets` 由用户在界面里逐笔录入或
> 从剪贴板粘贴金额列表；解析属于后续增量，不影响表结构与匹配算法。

---

## 13. 发版前复审加固（2026-11-13）

对 `a3042c9..HEAD`（18 个提交、184 文件）做了一轮五轴复审，修了 2 个 Critical 与 7 个
必须改。完整清单在 `docs/reviews/CODE_REVIEW.md` 的修复记录里，本节只记**改变了设计约定**
的部分——这些是会长期影响后续代码的判断，不只是 bug fix。

### 13.1 v3 归档被丢弃的分类

`DROP COLUMN transactions.category_id` 是不可逆的：升级之后，用户再也回答不了"上个月这笔记的
哪一类"，出问题时也无从对账。所以 v3 在 drop 之前多一步归档：

```sql
CREATE TABLE IF NOT EXISTS legacy_transaction_categories (
    transaction_id TEXT PRIMARY KEY,
    category_id    TEXT NOT NULL,
    archived_at    TEXT NOT NULL
);
INSERT OR IGNORE INTO legacy_transaction_categories (transaction_id, category_id, archived_at)
SELECT id, category_id, datetime('now') FROM transactions WHERE category_id IS NOT NULL;
ALTER TABLE transactions DROP COLUMN category_id;
```

`transaction_id` 做主键是刻意的：v3 之后这张表只写一次、只读，不需要多行历史，
主键同时保证 `INSERT OR IGNORE` 幂等。

> **v3 尚未发版**（`v0.1.2` 不含 `1de4128`），所以归档直接加进 `schema_v3.sql`，没有新开 v4。
> 本地用旧 HEAD 跑过、`user_version` 已到 3 的库不会重跑这条迁移，需要删库重建。

### 13.2 对账只在「对账单符号」下计算

信用卡账单印的是正数（欠 1500 显示 1500），账本里负债账户是负数（`-1500`）。此前两条路径各用
一套符号，勾选金额与差额永远差一个负号。现在 `db/reconcile.rs` 内部**统一用对账单符号**：

```rust
fn statement_sign(conn: &Connection, account_id: &str) -> Result<i64, AppError>
// asset → 1，liability → -1，其它 type → errors.reconcile.accountNotReconcilable
```

`cleared_balance` / `candidates` / `selected_total` / `totals` 全部乘上这个 `sign`，
`ensure_reconcilable` 退化成 `statement_sign(...).map(|_| ())`。前端不做任何翻符号——
金额只由后端算（AGENTS.md）。

同时收紧了两个入口校验：同一个 posting id 传两次会被 `HashSet` 拒掉（否则可以拿"两倍金额"
凑出错误的期末余额）；`t.date > statement_date` 的分录不允许出现在对账单上。

### 13.3 期初余额靠「有没有 equity 分录」识别

原来是 `description = 'Opening Balance'` 字符串匹配，用户把描述一改，期初就退化成普通交易，
删除账户时的清理逻辑也就跟着失效。现在统一走：

```rust
pub(in crate::db::accounts) const OPENING_BALANCE_PREDICATE: &str = "
    EXISTS (
        SELECT 1 FROM postings pe
          JOIN accounts ae ON ae.id = pe.account_id
         WHERE pe.transaction_id = t.id AND ae.type = 'equity'
    )";
```

语义是"这笔交易带一条权益分录"，与期初余额的定义（权益↔资产）严格对应。
更新权益侧时也不再依赖重新解析出的 `equity_id`，而是按 `type = 'equity'` 定位，
并且两条 `UPDATE` 都断言 `rows_affected == 1`——改不到就报错，不允许静默成功。

### 13.4 净资产的账户谓词收敛成一处

`type IN ('asset','liability') AND is_active = 1 AND include_net_worth = 1` 原本散在趋势、
资产负债表、账户变动、首页余额四处，任何一处漏掉 `include_net_worth` 都会让净资产对不上。
现在只有 `reports::net_worth_accounts(alias)` 一个出处（`pub(crate)`，通过 `db::reports` 再导出）。

副作用修掉了一个真 bug：`CreateAccountFullInput` 缺省 `include_net_worth: false`，
所有走内部 API 建的账户（含测试夹具的信用卡）都不进净资产，而 UI 上默认是勾选的。

### 13.5 预算窗口与结转

`budget_window(period, year, month)` 明确了两件事：

- **周预算看"本周"**：包含今天的那周一至周日；查非当前月时取该月最后一周（不猜用户在说哪一周）。
- **月预算看整月**：`first_of_month` ~ `last_of_month`。

`rollover_carry` 逐期累加剩余额度，**每期先 `max(0)`**（超支不倒扣下期，否则一次超支会
拖着后面几个月一起变负），并设 `MAX_ROLLOVER_PERIODS = 260`（5 年）上限防止无限回溯。
周预算同样支持结转——最初想拒绝，但既有测试与产品语义都不支持这个限制。

`spent_in_window` 加 `AND p.amount > 0`：预算的"已花"必须与报表的毛额口径一致，
否则一笔"退款"（损益账户上的负数分录）会把预算花销减下去。

### 13.6 前端金额解析必须与后端同源

`parseFloat(raw) * 100` + `Math.round` 看起来很安全，实际有四处不一致：`2.675` 得 267
（后端 268）、`0.005` 得 0（后端 1）、`1e5` 放行、`1,000` 放行。表现是**用户输入合法金额
却存不进去**。现在 `src/utils/money.ts` 是唯一入口，规则逐条对齐 Rust 侧，包括
`MAX_AMOUNT_CENTS = 10_000_000_000`（1 亿元）上限——超限在表单里就拦，不用等后端报错。
