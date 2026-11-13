# 05 - 数据模型

> 返回 [DESIGN.md](../DESIGN.md)

## SQL 表结构

```sql
-- 账户（扁平模型，扩展字段通过 account_meta 存储）
CREATE TABLE accounts (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL UNIQUE,       -- 格式: "{type}/{账户名}"，如 "asset/招商银行"
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

-- 账户元数据（灵活扩展字段）
CREATE TABLE account_meta (
    id          TEXT PRIMARY KEY,
    account_id  TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key         TEXT NOT NULL,
    value       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE(account_id, key)
);

-- 已知 meta 键及其用途：
-- account_role        -- asset 子类型: defaultAsset/sharedAsset/savingAsset/ccAsset/cashWalletAsset
-- liability_type      -- liability 子类型: loan/debt/mortgage
-- credit_card_type    -- 信用卡类型（ccAsset 时显示）
-- monthly_payment_date -- 信用卡还款日（ccAsset 时显示）
-- interest            -- 利率（liability 时显示）
-- interest_period     -- 利息周期: monthly/yearly（liability 时显示）

-- 交易
-- **设计说明**: 本系统采用纯自由复式记账，不存储 transaction_type。
-- 交易类型仅在 UI 层推导 Posting，数据库仅存储 postings 列表。
CREATE TABLE transactions (
    id              TEXT PRIMARY KEY,
    date            TEXT NOT NULL,          -- ISO 8601
    description     TEXT NOT NULL,
    category_id     TEXT REFERENCES categories(id),
    is_reconciled   INTEGER NOT NULL DEFAULT 0 CHECK (is_reconciled IN (0, 1)),
                                        -- 核对状态：0=未核对，1=已核对
                                        -- 已核对交易限制金额修改，防止账目混乱
    deleted_at      TEXT,                   -- 软删除时间戳：NULL=未删除，ISO 8601=已删除
                                        -- 支持交易撤销恢复
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);

-- 交易分录（核心：复式记账）
-- **借贷平衡**: sum(postings.amount) == 0 必须满足
CREATE TABLE postings (
    id             TEXT PRIMARY KEY,
    transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
    account_id     TEXT NOT NULL REFERENCES accounts(id),
    amount         INTEGER NOT NULL,       -- 以分为单位存储，避免浮点精度
                                        -- 正数=借方（资产增加/费用增加）
                                        -- 负数=贷方（资产减少/收入增加）
    sequence       INTEGER NOT NULL DEFAULT 0,
                                        -- 分录顺序：用于控制显示排序
                                        -- 通常借方在前（sequence=0），贷方在后（sequence=1）
    created_at     TEXT NOT NULL
);

-- 分类（扁平结构）
CREATE TABLE categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    type        TEXT NOT NULL CHECK (type IN ('income', 'expense')),
    icon        TEXT,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE UNIQUE INDEX idx_categories_type_name ON categories(type, name);

-- 预算
CREATE TABLE budgets (
    id          TEXT PRIMARY KEY,
    category_id TEXT NOT NULL REFERENCES categories(id),
    amount      INTEGER NOT NULL,       -- 分为单位
    period      TEXT NOT NULL,          -- monthly/weekly
    rollover    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_budgets_category ON budgets(category_id);
-- 每个分类只能有一个预算



-- 应用设置
CREATE TABLE settings (
    key     TEXT PRIMARY KEY,
    value   TEXT NOT NULL
);

-- 索引（性能优化）
CREATE UNIQUE INDEX IF NOT EXISTS idx_accounts_type_name ON accounts(type, name);
CREATE UNIQUE INDEX IF NOT EXISTS idx_categories_type_name ON categories(type, name);
CREATE INDEX IF NOT EXISTS idx_account_meta_account ON account_meta(account_id);
CREATE INDEX IF NOT EXISTS idx_postings_transaction ON postings(transaction_id);
CREATE INDEX IF NOT EXISTS idx_postings_account ON postings(account_id);
CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date);
-- 复合索引优化导出查询：WHERE deleted_at IS NULL AND date >= ? AND date <= ?
CREATE INDEX IF NOT EXISTS idx_transactions_date_deleted ON transactions(date, deleted_at);
CREATE INDEX IF NOT EXISTS idx_accounts_type ON accounts(type);
CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(type);
```

> **关键设计决策**: 金额以**整数（分）**存储，避免浮点精度问题。所有金额计算在后端 Rust 层完成，前端仅负责展示和格式化。

> **索引说明**:
>
> - `idx_transactions_date_deleted`: 复合索引，优化导出查询（WHERE deleted_at IS NULL AND date 范围）
> - `idx_postings_transaction`: 优化交易分录查询（JOIN postings）
> - `idx_postings_account`: 优化账户余额计算（按账户聚合）
> - `idx_transactions_date`: 优化列表排序（ORDER BY date DESC）

## TypeScript 类型定义

```typescript
type AccountType = "asset" | "liability" | "income" | "expense" | "equity";

// **注**: 本系统采用纯自由复式记账，不存储 transaction_type。
// 快速记账模式（withdrawal/deposit/transfer）仅在 UI 层用于推导 Posting，
// 后端不验证、不存储交易类型。

interface AccountDto {
  id: string;
  name: string; // 格式: "{type}/{账户名}"
  account_type: AccountType;
  currency: string;
  description: string;
  account_number: string | null;
  is_system: boolean;
  iban: string | null;
  is_active: boolean;
  include_net_worth: boolean;
  created_at: string;
  updated_at: string;
  meta: AccountMeta[]; // 扩展字段
  initial_balance?: number; // 期初余额（分）
  initial_balance_date?: string; // 期初日期
}

interface AccountMeta {
  id: string;
  account_id: string;
  key: string; // account_role, liability_type, credit_card_type 等
  value: string;
  created_at: string;
}

interface CreateAccountInput {
  name: string; // 路径格式: "资产/银行/招商银行" 或 "{type}/{name}"
  currency?: string;
  initial_balance?: string | number; // 十进制字符串或整数
  initial_balance_date?: string;
  description?: string;
  account_number?: string;
  iban?: string;
  is_active?: boolean;
  include_net_worth?: boolean;
  meta?: Record<string, string>; // 扩展字段
  equity_account_name?: string; // 权益账户名（用于期初余额）
  opening_balance_name?: string; // 期初余额账户名
}

interface UpdateAccountInput {
  name?: string;
  description?: string;
  account_number?: string;
  initial_balance?: string;
  initial_balance_date?: string;
  iban?: string;
  is_active?: boolean;
  include_net_worth?: boolean;
  meta?: Record<string, string>;
}

interface AccountTransaction {
  id: string;
  date: string;
  description: string;
  category_id: string | null;
  amount: number; // 分，正=借方，负=贷方
  is_reconciled: boolean; // 核对状态
  created_at: string;
  updated_at: string;
}

interface Transaction {
  id: string;
  date: string; // ISO 8601
  description: string;
  categoryId: string | null;
  isReconciled: boolean; // 核对状态：已核对交易限制金额修改
  deletedAt: string | null; // 软删除时间戳：NULL=未删除，ISO 8601=已删除
  postings: Posting[];
  createdAt: string;
  updatedAt: string;
}

interface Posting {
  id: string;
  accountId: string;
  amount: number; // 分，正数为借方，负数为贷方
  sequence: number; // 分录顺序：用于显示排序
}

interface Category {
  id: string;
  name: string;
  type: "income" | "expense";
  icon: string | null;
  createdAt: string;
  updatedAt: string;
}

interface CreateCategoryInput {
  name: string;
  type: "income" | "expense";
  icon?: string | null;
}

interface UpdateCategoryInput {
  name?: string;
  icon?: string | null;
}

interface DeletePreview {
  budgetCount: number;
  transactionCount: number;
  canDelete: boolean;
}

interface Budget {
  id: string;
  categoryId: string;
  amount: number; // 分为单位
  period: "monthly" | "weekly";
  rollover: boolean;
  createdAt: string;
  updatedAt: string;
}

interface BudgetStatus extends Budget {
  categoryName: string; // 分类名称（用于显示）
  categoryIcon: string | null; // 分类图标
  spent: number; // 当前周期已支出（分）
  remaining: number; // 剩余预算 = available - spent（分）
  overBudget: boolean; // 是否超支（spent > available）
  rolloverAmount: number; // 上期结余滚存（当前为 0，BUD-4 预留）
  available: number; // 可用预算 = amount + rolloverAmount（分）
}

interface DbInfo {
  path: string;
  label: string;
  createdAt: string;
}

interface RecentDbEntry {
  path: string;
  label: string;
  lastOpened: string;
}
```

## 新增字段说明

### transactions 表

| 字段              | 类型          | 默认值 | 说明                             |
| ----------------- | ------------- | ------ | -------------------------------- |
| **is_reconciled** | INTEGER (0/1) | 0      | 核对状态：已核对交易限制金额修改 |
| **deleted_at**    | TEXT          | NULL   | 软删除时间戳：支持交易撤销恢复   |

### postings 表

| 字段         | 类型    | 默认值 | 说明                               |
| ------------ | ------- | ------ | ---------------------------------- |
| **sequence** | INTEGER | 0      | 分录顺序：控制显示排序（借方在前） |

> **设计决策**:
>
> - **软删除**: 用户删除交易时先标记 deleted_at，支持恢复；硬删除需用户二次确认
> - **核对状态**: 已核对交易不可修改金额，确保账目与外部记录一致
> - **分录顺序**: 支持多分录交易的有序显示，提升用户体验
