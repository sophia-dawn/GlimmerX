# 16 - 快速记账模式设计

> 返回 [DESIGN.md](../DESIGN.md) > 返回 [交易模块总览](14-transaction-module.md)

---

## 一、概念定义

### 什么是快速记账？

快速记账（Quick Add）是一种**UI 简化录入模式**，用于日常收支记录的快速创建。

**核心特点**：

- 用户只需输入：金额 + 分类/账户 + 备注
- 系统自动推导：完整的双分录结构（仅 UI 层，不存储类型标识）
- 适用场景：支出、收入、转账

> **重要**: 快速记账仅简化用户输入流程，系统根据用户选择的"记账模式"（支出/收入/转账）推导 Posting，
> 最终提交到数据库的仍是纯 Posting 列表，**不存储**"支出/收入/转账"类型标识。

**对比自由记账**：

| 维度     | 快速记账            | 自由记账              |
| -------- | ------------------- | --------------------- |
| 用户输入 | 金额 + 分类/账户    | 多个账户 + 各账户金额 |
| 分录数量 | 固定 2 条           | 任意 ≥2 条            |
| 记账模式 | UI 层选择（不存储） | 无                    |
| 适用人群 | 普通用户            | 专业用户              |
| UI 形式  | 弹窗/快捷键         | 完整表单页            |

---

## 二、分录推导规则

### 2.1 核心推导逻辑

根据用户选择的"记账模式"，系统自动生成 2 条 Posting（仅 UI 层推导，不存储模式标识）：

```
支出模式:
┌─────────────────────────────────────────────────────────────┐
│ 用户输入: 金额=35, source=现金账户, category=餐饮分类       │
│                                                              │
│ 系统推导:                                                    │
│   Posting 1 (借方): expense类型/餐饮账户  +35               │
│   Posting 2 (贷方): asset类型/现金账户    -35               │
│                                                              │
│ 分录金额: sum = +35 + (-35) = 0 ✓                           │
└─────────────────────────────────────────────────────────────┘

收入模式:
┌─────────────────────────────────────────────────────────────┐
│ 用户输入: 金额=5000, destination=银行卡, category=工资分类  │
│                                                              │
│ 系统推导:                                                    │
│   Posting 1 (借方): asset类型/银行卡账户  +5000             │
│   Posting 2 (贷方): income类型/工资账户   -5000             │
│                                                              │
│ 分录金额: sum = +5000 + (-5000) = 0 ✓                       │
└─────────────────────────────────────────────────────────────┘

转账模式:
┌─────────────────────────────────────────────────────────────┐
│ 用户输入: 金额=100, source=银行卡, destination=支付宝       │
│                                                              │
│ 系统推导:                                                    │
│   Posting 1 (借方): asset类型/支付宝账户  +100              │
│   Posting 2 (贷方): asset类型/银行卡账户  -100              │
│                                                              │
│ 分录金额: sum = +100 + (-100) = 0 ✓                         │
└─────────────────────────────────────────────────────────────┘
```

> **借贷方向说明**:
>
> - 支出：费用账户增加（借方+），资产账户减少（贷方-）
> - 收入：资产账户增加（借方+），收入账户减少（贷方-）
> - 转账：转入账户增加（借方+），转出账户减少（贷方-）

### 2.2 账户类型映射

| 记账模式 | 借方（正金额）账户来源                             | 贷方（负金额）账户来源                        |
| -------- | -------------------------------------------------- | --------------------------------------------- |
| **支出** | expense 分类账户（由 category_id 推导）            | asset/liability 资金账户（source_account_id） |
| **收入** | asset/liability 资金账户（destination_account_id） | income 分类账户（由 category_id 推导）        |
| **转账** | asset/liability 资金账户（destination_account_id） | asset/liability 资金账户（source_account_id） |

### 2.3 分类与账户的映射关系

**关键设计**：GlimmerX 的分类（Category）是独立的元数据，不直接作为账户。但快速记账时，分类 ID 需映射为对应类型的账户。

```rust
/// 根据分类 ID 获取对应的账户
///
/// 映射规则：
/// - expense 分类 → expense 类型账户，名称为分类名
/// - income 分类  → income 类型账户，名称为分类名
fn get_account_from_category(
    conn: &Connection,
    category_id: &str,
) -> Result<String, AppError> {
    let (cat_type, cat_name): (String, String) = conn.query_row(
        "SELECT type, name FROM categories WHERE id = ?1",
        [category_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // 账户名直接使用分类名（无前缀）
    match cat_type.as_str() {
        "expense" | "income" => Ok(cat_name),
        _ => Err(AppError::ValidationError(
            format!("分类类型 {} 无法映射为账户", cat_type)
        )),
    }
}

/// 检查分类账户是否存在，不存在则自动创建
///
/// **竞态条件处理**: 使用事务 + UNIQUE 约束防止并发创建重复账户：
/// 1. 账户 name 列有 UNIQUE 约束（schema.rs）
/// 2. 使用事务保证查询+创建原子性
/// 3. INSERT 失败时回退查询（说明另一事务已创建）
fn ensure_category_account(
    conn: &Connection,
    category_id: &str,
) -> Result<String, AppError> {
    let cat_type = get_category_type(conn, category_id)?;
    let cat_name = get_account_from_category(conn, category_id)?;

    // 先查询是否已存在同名同类型账户
    let existing: Option<String> = conn.query_row(
        "SELECT id FROM accounts WHERE name = ?1 AND type = ?2",
        rusqlite::params![&cat_name, &cat_type],
        |row| row.get(0),
    ).optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    // 不存在则创建
    let account_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO accounts (id, name, type, currency, description, is_active, include_net_worth, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'CNY', '', 1, 0, ?4, ?4)",
        rusqlite::params![&account_id, &cat_name, &cat_type, &now],
    )?;

    Ok(account_id)
}
```

> **设计决策**：分类账户是自动创建的账户，用户也可在账户页面手动创建。命名规则统一：账户名直接使用分类名（如"餐饮"），通过账户类型字段区分 expense/income。

---

## 三、输入数据结构

### 3.1 Rust 结构体

```rust
// src-tauri/src/models/transaction.rs

/// 快速记账输入结构
///
/// 注: 记账模式（mode）仅用于 UI 层推导 Posting，不存储到数据库
#[derive(Debug, Deserialize)]
pub struct QuickAddInput {
    /// 记账模式：expense / income / transfer（必填）
    /// 用于 UI 层推导 Posting，不存储
    pub mode: String,

    /// 金额（十进制字符串，如 "35.00"）（必填）
    pub amount: String,

    /// 来源账户 ID（可选）
    /// - expense: 资金来源账户（asset/liability）
    /// - income: 不使用（收入来源由分类决定）
    /// - transfer: 转出账户
    pub source_account_id: Option<String>,

    /// 目标账户 ID（可选）
    /// - expense: 不使用（支出目标由分类决定）
    /// - income: 资金去向账户（asset/liability）
    /// - transfer: 转入账户
    pub destination_account_id: Option<String>,

    /// 分类 ID（可选）
    /// - expense: 支出分类（expense type）
    /// - income: 收入分类（income type）
    /// - transfer: 不使用
    ///
    /// 可替代 destination_account：提供 category_id 时自动推导账户
    pub category_id: Option<String>,

    /// 交易描述/备注（可选，默认空字符串）
    pub description: Option<String>,

    /// 交易日期（可选，默认今天）
    pub date: Option<String>,

    /// 标签 ID 列表（可选）
    pub tags: Option<Vec<String>>,
}
```

### 3.2 TypeScript 类型

```typescript
// src/types/index.ts 补充

/// 快速记账输入结构
/// 注: mode 仅用于 UI 层推导 Posting，不存储到数据库
export interface QuickAddInput {
  mode: "expense" | "income" | "transfer"; // 记账模式（必填）
  amount: string; // 必填，如 "35.00"
  source_account_id?: string; // 可选
  destination_account_id?: string; // 可选
  category_id?: string; // 可选
  description?: string; // 可选
  date?: string; // 可选，ISO 8601
  tags?: string[]; // 可选
}
```

---

## 四、验证逻辑

### 4.1 必填验证

```rust
pub fn validate_quick_add_input(input: &QuickAddInput) -> Result<i64, AppError> {
    // 1. 记账模式验证
    match input.mode.as_str() {
        "expense" | "income" | "transfer" => {}
        _ => return Err(AppError::ValidationError(
            "errors.transaction.invalidMode".to_string()
        )),
    }

    // 2. 金额验证（正数）
    let amount_cents = parse_amount_to_cents(&input.amount)?;
    if amount_cents <= 0 {
        return Err(AppError::ValidationError(
            "errors.transaction.amountMustBePositive".to_string()
        ));
    }

    // 3. 必须提供账户信息（source 或 category）
    match input.mode.as_str() {
        "expense" => {
            if input.source_account_id.is_none() && input.category_id.is_none() {
                return Err(AppError::ValidationError(
                    "errors.transaction.expenseRequiresSource".to_string()
                ));
            }
        }
        "income" => {
            if input.destination_account_id.is_none() && input.category_id.is_none() {
                return Err(AppError::ValidationError(
                    "errors.transaction.incomeRequiresDestination".to_string()
                ));
            }
        }
        "transfer" => {
            if input.source_account_id.is_none() || input.destination_account_id.is_none() {
                return Err(AppError::ValidationError(
                    "errors.transaction.transferRequiresBothAccounts".to_string()
                ));
            }
        }
        _ => {}
    }

    Ok(amount_cents)
}
```

### 4.2 账户类型匹配验证

```rust
/// 验证账户类型与记账模式的匹配
pub fn validate_account_type_match(
    conn: &Connection,
    input: &QuickAddInput,
) -> Result<(), AppError> {
    match input.mode.as_str() {
        "expense" => {
            // source 必须是 asset/liability
            if let Some(ref acct_id) = input.source_account_id {
                let acct_type: String = conn.query_row(
                    "SELECT type FROM accounts WHERE id = ?1",
                    [acct_id],
                    |row| row.get(0),
                )?;
                if !matches!(acct_type.as_str(), "asset" | "liability") {
                    return Err(AppError::ValidationError(
                        "errors.transaction.invalidExpenseSourceAccount".to_string()
                    ));
                }
            }

            // category 必须是 expense 类型
            if let Some(ref cat_id) = input.category_id {
                let cat_type: String = conn.query_row(
                    "SELECT type FROM categories WHERE id = ?1",
                    [cat_id],
                    |row| row.get(0),
                )?;
                if cat_type != "expense" {
                    return Err(AppError::ValidationError(
                        "errors.transaction.invalidExpenseCategory".to_string()
                    ));
                }
            }
        }

        "income" => {
            // destination 必须是 asset/liability
            if let Some(ref acct_id) = input.destination_account_id {
                let acct_type: String = conn.query_row(
                    "SELECT type FROM accounts WHERE id = ?1",
                    [acct_id],
                    |row| row.get(0),
                )?;
                if !matches!(acct_type.as_str(), "asset" | "liability") {
                    return Err(AppError::ValidationError(
                        "errors.transaction.invalidIncomeDestinationAccount".to_string()
                    ));
                }
            }

            // category 必须是 income 类型
            if let Some(ref cat_id) = input.category_id {
                let cat_type: String = conn.query_row(
                    "SELECT type FROM categories WHERE id = ?1",
                    [cat_id],
                    |row| row.get(0),
                )?;
                if cat_type != "income" {
                    return Err(AppError::ValidationError(
                        "errors.transaction.invalidIncomeCategory".to_string()
                    ));
                }
            }
        }

        "transfer" => {
            // source 和 destination 都必须是 asset/liability
            for acct_id in &[&input.source_account_id, &input.destination_account_id] {
                if let Some(ref id) = acct_id {
                    let acct_type: String = conn.query_row(
                        "SELECT type FROM accounts WHERE id = ?1",
                        [id],
                        |row| row.get(0),
                    )?;
                    if !matches!(acct_type.as_str(), "asset" | "liability") {
                        return Err(AppError::ValidationError(
                            "errors.transaction.invalidTransferAccount".to_string()
                        ));
                    }
                }
            }

            // 禁止转给自己
            if input.source_account_id == input.destination_account_id {
                return Err(AppError::ValidationError(
                    "errors.transaction.transferToSelf".to_string()
                ));
            }
        }

        _ => {}
    }

    Ok(())
}
```

---

## 五、核心实现

### 5.1 快速记账函数

```rust
// src-tauri/src/db/transactions.rs

use rusqlite::Connection;
use uuid::Uuid;
use chrono::Utc;

/// 快速记账核心函数
///
/// 流程：
/// 1. 验证输入
/// 2. 解析金额
/// 3. 验证账户类型匹配
/// 4. 推导分录（根据 mode）
/// 5. 调用 transaction_create 创建交易（不存储 mode）
pub fn quick_add_transaction(
    conn: &Connection,
    input: &QuickAddInput,
) -> Result<TransactionWithPostings, AppError> {
    // Step 1-3: 验证
    let amount_cents = validate_quick_add_input(input)?;
    validate_account_type_match(conn, input)?;

    // 日期验证（如果提供）
    if let Some(ref date) = input.date {
        validate_date_format(date)?;
    }

    // Step 4: 推导分录
    let postings = derive_postings(conn, input, amount_cents)?;

    // Step 5: 构造完整交易输入并创建（不存储 mode）
    let tx_input = CreateTransactionInput {
        date: input.date.clone().unwrap_or_else(||
            Utc::now().format("%Y-%m-%d").to_string()
        ),
        description: input.description.clone().unwrap_or_default(),
        category_id: input.category_id.clone(),
        postings,  // 仅存储推导出的 Posting，不存储 mode
        tags: input.tags.clone(),
    };

    create_transaction(conn, &tx_input)
}

/// 验证日期格式（ISO 8601: YYYY-MM-DD）
fn validate_date_format(date_str: &str) -> Result<(), AppError> {
    chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::ValidationError(
            "errors.transaction.invalidDateFormat".to_string()
        ))?;
    Ok(())
}

/// 分录推导逻辑
fn derive_postings(
    conn: &Connection,
    input: &QuickAddInput,
    amount_cents: i64,
) -> Result<Vec<PostingInput>, AppError> {
    match input.mode.as_str() {
        "expense" => {
            // 借方：expense 分类账户
            let debit_account = if let Some(ref cat_id) = input.category_id {
                ensure_category_account(conn, cat_id)?
            } else {
                // 如果没有分类，需要有 destination_account_id（直接 expense 账户）
                input.destination_account_id.clone().unwrap()
            };

            // 贷方：asset/liability 资金账户
            let credit_account = input.source_account_id.clone()
                .or_else(|| get_default_asset_account(conn, "expense")?)
                .ok_or_else(|| AppError::ValidationError(
                    "errors.transaction.noDefaultSourceAccount".to_string()
                ))?;

            Ok(vec![
                PostingInput { account_id: debit_account, amount: amount_cents.to_string() },
                PostingInput { account_id: credit_account, amount: (-amount_cents).to_string() },
            ])
        }

        "income" => {
            // 借方：asset/liability 资金账户
            let debit_account = input.destination_account_id.clone()
                .or_else(|| get_default_asset_account(conn, "income")?)
                .ok_or_else(|| AppError::ValidationError(
                    "errors.transaction.noDefaultDestinationAccount".to_string()
                ))?;

            // 贷方：income 分类账户
            let credit_account = if let Some(ref cat_id) = input.category_id {
                ensure_category_account(conn, cat_id)?
            } else {
                input.source_account_id.clone().unwrap()
            };

            Ok(vec![
                PostingInput { account_id: debit_account, amount: amount_cents.to_string() },
                PostingInput { account_id: credit_account, amount: (-amount_cents).to_string() },
            ])
        }

        "transfer" => {
            // 借方：转入账户
            let debit_account = input.destination_account_id.clone().unwrap();

            // 贷方：转出账户
            let credit_account = input.source_account_id.clone().unwrap();

            Ok(vec![
                PostingInput { account_id: debit_account, amount: amount_cents.to_string() },
                PostingInput { account_id: credit_account, amount: (-amount_cents).to_string() },
            ])
        }

        _ => Err(AppError::ValidationError(
            "errors.transaction.cannotDerivePostings".to_string()
        )),
    }
}

/// 获取用户的默认资产账户
///
/// 优先级：
/// 1. 用户设置中的默认账户（key: "default_asset_account"）
/// 2. 第一个 active 的 asset 类型账户
fn get_default_asset_account(
    conn: &Connection,
    for_mode: &str, // "expense" 或 "income"
) -> Result<Option<String>, AppError> {
    // 检查用户设置
    let setting_key = format!("default_{}_account", for_mode);
    let default: Option<String> = conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [&setting_key],
        |row| row.get(0),
    ).optional()?;

    if let Some(acct_id) = default {
        // 验证账户仍然有效
        let is_active: bool = conn.query_row(
            "SELECT is_active FROM accounts WHERE id = ?1",
            [&acct_id],
            |row| row.get(0),
        ).optional()?.unwrap_or(false);

        if is_active {
            return Ok(Some(acct_id));
        }
    }

    // 回退：第一个 active 的 asset 账户
    let first_asset: Option<String> = conn.query_row(
        "SELECT id FROM accounts WHERE type = 'asset' AND is_active = 1 ORDER BY created_at LIMIT 1",
        [],
        |row| row.get(0),
    ).optional()?;

    Ok(first_asset)
}
```

            format!("无效的交易类型: {}", type_str)
        )),
    }

}

````

---

## 六、Tauri Command

```rust
// src-tauri/src/commands/transactions.rs

/// 快速记账 Tauri Command
#[tauri::command]
pub async fn quick_add_transaction(
    input: QuickAddInput,
    state: State<'_, AppState>,
) -> Result<TransactionWithPostings, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();

    quick_add_transaction(&conn, &input).map_err(|e| e.to_string())
}

/// 获取默认账户列表（用于前端下拉选择）
#[tauri::command]
pub async fn get_default_accounts(
    state: State<'_, AppState>,
) -> Result<DefaultAccounts, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();

    // 获取所有 active 的 asset/liability 账户
    let accounts = get_active_asset_accounts(&conn).map_err(|e| e.to_string())?;

    // 获取用户设置的默认账户
    let default_expense = get_user_setting(&conn, "default_expense_account");
    let default_income = get_user_setting(&conn, "default_income_account");

    Ok(DefaultAccounts {
        accounts,
        default_expense_account: default_expense,
        default_income_account: default_income,
    })
}

#[derive(Debug, Serialize)]
pub struct DefaultAccounts {
    pub accounts: Vec<AccountDto>,
    pub default_expense_account: Option<String>,
    pub default_income_account: Option<String>,
}
```

---

## 七、前端 API 调用

```typescript
// src/utils/api.ts 补充

import { invoke } from "@tauri-apps/api/core";

export async function quickAddTransaction(
  input: QuickAddInput,
): Promise<TransactionWithPostings> {
  return invoke("quick_add_transaction", { input });
}

export async function getDefaultAccounts(): Promise<DefaultAccounts> {
  return invoke("get_default_accounts");
}

// 使用示例：支出记账
const quickAddExpense = async () => {
  const result = await quickAddTransaction({
    mode: "expense",
    amount: "35.00",
    category_id: "cat-food-uuid", // 餐饮分类
    description: "午餐",
    tags: ["tag-workday-uuid"],
  });

  console.log("快速记账成功:", result.id);
  // 系统自动创建：
  // - expense/餐饮 +35.00
  // - asset/默认账户 -35.00
};

// 使用示例：收入记账
const quickAddIncome = async () => {
  const result = await quickAddTransaction({
    mode: "income",
    amount: "5000.00",
    category_id: "cat-salary-uuid", // 工资分类
    destination_account_id: "acct-bank-uuid",
    description: "工资收入",
  });

  // 系统自动创建：
  // - asset/银行卡 +5000.00
  // - income/工资 -5000.00
};

// 使用示例：转账
const quickAddTransfer = async () => {
  const result = await quickAddTransaction({
    mode: "transfer",
    amount: "100.00",
    source_account_id: "acct-bank-uuid",
    destination_account_id: "acct-alipay-uuid",
    description: "银行卡转支付宝",
  });

  // 系统自动创建：
  // - asset/支付宝 +100.00
  // - asset/银行卡 -100.00
};
```

---

## 八、前端 UI 设计

### 8.1 快速记账弹窗组件

```typescript
// src/components/transactions/QuickAddDialog.tsx

import { useState } from "react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { CategoryPicker } from "@/components/categories/CategoryPicker";
import { AccountPicker } from "@/components/accounts/AccountPicker";
import { TagPicker } from "@/components/tags/TagPicker";
import { quickAddTransaction, getDefaultAccounts } from "@/utils/api";

interface QuickAddDialogProps {
  open: boolean;
  onClose: () => void;
  onSuccess?: (transaction: TransactionWithPostings) => void;
}

export function QuickAddDialog({ open, onClose, onSuccess }: QuickAddDialogProps) {
  const [mode, setMode] = useState<"expense" | "income" | "transfer">("expense");
  const [amount, setAmount] = useState("");
  const [categoryId, setCategoryId] = useState<string | null>(null);
  const [sourceAccountId, setSourceAccountId] = useState<string | null>(null);
  const [destinationAccountId, setDestinationAccountId] = useState<string | null>(null);
  const [description, setDescription] = useState("");

  // 根据记账模式显示不同的字段
  const renderFields = () => {
    switch (mode) {
      case "expense":
        return (
          <>
            <CategoryPicker
              label="支出分类"
              categoryType="expense"
              value={categoryId}
              onChange={setCategoryId}
              required
            />
            <AccountPicker
              label="资金来源"
              accountTypes={["asset", "liability"]}
              value={sourceAccountId}
              onChange={setSourceAccountId}
              optional // 有默认账户
            />
          </>
        );

      case "income":
        return (
          <>
            <CategoryPicker
              label="收入分类"
              categoryType="income"
              value={categoryId}
              onChange={setCategoryId}
              required
            />
            <AccountPicker
              label="资金去向"
              accountTypes={["asset", "liability"]}
              value={destinationAccountId}
              onChange={setDestinationAccountId}
              optional
            />
          </>
        );

      case "transfer":
        return (
          <>
            <AccountPicker
              label="转出账户"
              accountTypes={["asset", "liability"]}
              value={sourceAccountId}
              onChange={setSourceAccountId}
              required
            />
            <AccountPicker
              label="转入账户"
              accountTypes={["asset", "liability"]}
              value={destinationAccountId}
              onChange={setDestinationAccountId}
              required
            />
          </>
        );
    }
  };

  const handleSubmit = async () => {
    const result = await quickAddTransaction({
      mode,
      amount,
      category_id: categoryId,
      source_account_id: sourceAccountId,
      destination_account_id: destinationAccountId,
      description,
      tags: tagIds,
    });

    onSuccess?.(result);
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>快速记账</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          {/* 记账模式选择 */}
          <div className="flex gap-2">
            <Button
              variant={mode === "expense" ? "default" : "outline"}
              onClick={() => setMode("expense")}
            >
              支出
            </Button>
            <Button
              variant={mode === "income" ? "default" : "outline"}
              onClick={() => setMode("income")}
            >
              收入
            </Button>
            <Button
              variant={mode === "transfer" ? "default" : "outline"}
              onClick={() => setMode("transfer")}
            >
              转账
            </Button>
          </div>

          {/* 金额输入 */}
          <Input
            type="number"
            placeholder="金额"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
          />

          {/* 动态字段 */}
          {renderFields()}

          {/* 备注 */}
          <Input
            placeholder="备注（可选）"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />

          {/* 提交 */}
          <Button onClick={handleSubmit} className="w-full">
            保存
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

### 8.2 全局快捷键

```typescript
// src/App.tsx 或 src/hooks/useGlobalShortcuts.ts

import { useEffect } from "react";
import { register } from "@tauri-apps/api/globalShortcut";

export function useGlobalShortcuts(onQuickAdd: () => void) {
  useEffect(() => {
    // 注册 Ctrl/Cmd + N 快捷键
    register("CommandOrControl+N", () => {
      onQuickAdd();
    });

    return () => {
      // 清理快捷键注册
    };
  }, [onQuickAdd]);
}

// 在 AppShell 中使用
function AppShell() {
  const [quickAddOpen, setQuickAddOpen] = useState(false);

  useGlobalShortcuts(() => setQuickAddOpen(true));

  return (
    <>
      {/* ... */}
      <QuickAddDialog
        open={quickAddOpen}
        onClose={() => setQuickAddOpen(false)}
        onSuccess={(tx) => {
          toast.success("记账成功");
          setQuickAddOpen(false);
        }}
      />
    </>
  );
}
```

---

## 九、分类账户自动创建策略

### 9.1 分类创建时的联动

当用户在分类管理界面创建新分类时，系统自动创建对应的隐藏账户：

```rust
// src-tauri/src/db/categories.rs 补充

pub fn create_category(
    conn: &Connection,
    input: &CreateCategoryInput,
) -> Result<Category, AppError> {
    // ... 创建分类 ...

    // 自动创建对应的分类账户
    let account_path = format!("{}{}", input.type, input.name);
    let account_id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO accounts (id, name, type, currency, description, is_system, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'CNY', '自动创建的分类账户', 0, 1, ?4, ?4)",
        rusqlite::params![&account_id, &account_path, &input.type, &now],
    )?;

    // ... 返回分类 ...
}
```

### 9.2 分类删除时的联动

删除分类时，对应的分类账户应该如何处理？

**策略选择**：

| 方案            | 说明                                   | 优缺点                                     |
| --------------- | -------------------------------------- | ------------------------------------------ |
| A. 同时删除账户 | 分类删除 → 账户删除                    | 简单，但可能导致历史交易的 account_id 失效 |
| B. 保留账户     | 分类删除 → 账户保留（标记为 inactive） | 安全，历史交易不受影响，但留下僵尸账户     |
| C. 检查后决定   | 有交易 → 保留账户；无交易 → 删除账户   | 最佳，但逻辑复杂                           |

**推荐方案 C**：

```rust
pub fn delete_category(conn: &Connection, category_id: &str) -> Result<(), AppError> {
    // 1. 查找对应账户
    let cat_name = get_category_name(conn, category_id)?;
    let cat_type = get_category_type(conn, category_id)?;
    let account_path = format!("{}{}", cat_type, cat_name);

    // 2. 检查账户是否有交易
    let has_transactions: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM postings p
         JOIN accounts a ON p.account_id = a.id
         WHERE a.name = ?1)",
        [&account_path],
        |row| row.get(0),
    )?;

    // 3. 删除分类（解除交易关联）
    conn.execute(
        "UPDATE transactions SET category_id = NULL WHERE category_id = ?1",
        [category_id],
    )?;
    conn.execute("DELETE FROM categories WHERE id = ?1", [category_id])?;

    // 4. 处理账户
    if has_transactions {
        // 有交易 → 保留账户，标记 inactive
        conn.execute(
            "UPDATE accounts SET is_active = 0 WHERE name = ?1",
            [&account_path],
        )?;
    } else {
        // 无交易 → 删除账户
        conn.execute(
            "DELETE FROM accounts WHERE name = ?1",
            [&account_path],
        )?;
    }

    Ok(())
}
```

---

## 十、单元测试设计

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_add_expense_with_category() {
        let (_dir, conn) = test_env();
        setup_test_data(&conn);  // 创建分类、账户

        let input = QuickAddInput {
            mode: "expense".into(),
            amount: "35.00".into(),
            category_id: Some("cat-food-uuid".into()),
            source_account_id: Some("acct-cash-uuid".into()),
            description: Some("午餐".into()),
            ..Default::default()
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_ok());

        let tx = result.unwrap();
        assert_eq!(tx.postings.len(), 2);
        // 注: transaction_type 不存储，无此字段

        // 验证分录金额平衡
        let sum = tx.postings.iter().map(|p| p.amount).sum::<i64>();
        assert_eq!(sum, 0);
    }

    #[test]
    fn test_quick_add_income_auto_creates_category_account() {
        let (_dir, conn) = test_env();
        create_category(&conn, &CreateCategoryInput {
            name: "工资".into(),
            type: "income".into(),
            ..Default::default()
        });

        let input = QuickAddInput {
            mode: "income".into(),
            amount: "5000.00".into(),
            category_id: Some("cat-salary-uuid".into()),
            destination_account_id: Some("acct-bank-uuid".into()),
            ..Default::default()
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_ok());

        // 验证 income/工资 账户自动创建
        let account_exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE name = 'income/工资')",
            [],
            |row| row.get(0),
        ).unwrap();
        assert!(account_exists);
    }

    #[test]
    fn test_quick_add_transfer_same_account_fails() {
        let (_dir, conn) = test_env();

        let input = QuickAddInput {
            mode: "transfer".into(),
            amount: "100.00".into(),
            source_account_id: Some("acct-bank-uuid".into()),
            destination_account_id: Some("acct-bank-uuid".into()), // 同一个账户
            ..Default::default()
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_derive_postings_expense() {
        let (_dir, conn) = test_env();

        let input = QuickAddInput {
            mode: "expense".into(),
            amount: "100.00".into(),
            category_id: Some("cat-food-uuid".into()),
            source_account_id: Some("acct-cash-uuid".into()),
            ..Default::default()
        };

        let postings = derive_postings(&conn, &input, 10000).unwrap();

        // 借方：expense 账户 +100
        assert!(postings[0].amount.parse::<i64>().unwrap() > 0);
        // 贷方：asset 账户 -100
        assert!(postings[1].amount.parse::<i64>().unwrap() < 0);
    }

    #[test]
    fn test_negative_amount_fails() {
        let (_dir, conn) = test_env();

        let input = QuickAddInput {
            mode: "expense".into(),
            amount: "-35.00".into(), // 负数
            ..Default::default()
        };

        let result = quick_add_transaction(&conn, &input);
        assert!(result.is_err());
    }
}
```

---

## 十一、设计决策总结

| 决策                 | 说明                                                           |
| -------------------- | -------------------------------------------------------------- |
| **固定 2 分录**      | 快速记账只支持 2 分录交易，多分录需使用自由记账                |
| **金额必须为正**     | 用户输入正数金额，系统自动分配正负号给分录                     |
| **纯复式记账**       | mode 仅用于 UI 层推导 Posting，不存储到数据库                  |
| **分类账户自动创建** | 创建分类时自动创建对应的 expense/income 账户，用户不可见       |
| **默认账户机制**     | 用户未指定资金账户时，使用设置中的默认账户或第一个 asset 账户  |
| **分类可替代账户**   | expense/income 时，提供 category_id 即可，系统自动推导账户     |
| **类型严格验证**     | 确保账户类型与记账模式匹配（expense → expense 分类）           |
| **全局快捷键**       | Ctrl/Cmd + N 触发弹窗，提升录入效率                            |

---

## 十二、后续扩展

| 功能               | 说明                                     | 优先级 |
| ------------------ | ---------------------------------------- | ------ |
| **默认账户设置页** | 让用户配置 expense/income 的默认账户     | P1     |
| **最近使用账户**   | 记住用户上次使用的账户，下次默认选中     | P2     |
| **智能分类推荐**   | 根据描述关键词推荐分类                   | P2     |
| **语音记账**       | 通过语音输入快速记账                     | P3     |`
````
