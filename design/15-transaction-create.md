# 15 - 交易创建流程详细设计

> 返回 [DESIGN.md](../DESIGN.md) | 返回 [交易模块总览](14-transaction-module.md)

---

## 一、数据结构定义

### 1.1 Rust 结构体

```rust
// src-tauri/src/models/transaction.rs (新建)

use serde::{Deserialize, Serialize};

/// 创建交易的输入结构
#[derive(Debug, Deserialize)]
pub struct CreateTransactionInput {
    /// 交易日期（ISO 8601），必填
    pub date: String,
    /// 交易描述，必填
    pub description: String,
    /// 分类ID，可选
    pub category_id: Option<String>,
    /// 分录列表，至少2条
    pub postings: Vec<PostingInput>,
    /// 标签ID列表，可选
    pub tags: Option<Vec<String>>,
}

/// 分录输入结构
#[derive(Debug, Deserialize)]
pub struct PostingInput {
    /// 账户ID，必填
    pub account_id: String,
    /// 金额（十进制字符串，如 "-100.50"），必填
    /// 前端传入字符串，后端解析为 i64（分）
    pub amount: String,
}

/// 创建交易返回的完整交易对象
#[derive(Debug, Serialize)]
pub struct TransactionWithPostings {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub postings: Vec<Posting>,
    pub tags: Vec<Tag>,
    pub created_at: String,
    pub updated_at: String,
}

/// 分录结构
#[derive(Debug, Serialize)]
pub struct Posting {
    pub id: String,
    pub account_id: String,
    pub amount: i64, // 分，正数借方，负数贷方
}

/// 标签结构（简化）
#[derive(Debug, Serialize)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

> **设计说明**: 本系统采用纯自由复式记账，不存储交易类型标识。
> 所有交易由用户指定 Posting 组合，系统仅验证借贷平衡。
```

### 1.2 TypeScript 类型定义

```typescript
// src/types/index.ts 补充

export interface CreateTransactionInput {
  date: string; // ISO 8601, 必填
  description: string; // 必填
  category_id?: string | null; // 可选
  postings: PostingInput[]; // 至少2条
  tags?: string[]; // 标签ID列表
}

export interface PostingInput {
  account_id: string; // 必填
  amount: string; // 十进制字符串，如 "-100.50"
}

export interface TransactionWithPostings {
  id: string;
  date: string;
  description: string;
  category_id: string | null;
  postings: Posting[];
  tags: Tag[];
  created_at: string;
  updated_at: string;
}

// 注: 本系统不存储交易类型，所有交易均为纯复式记账
```

---

## 二、验证逻辑设计

### 2.1 必须验证（硬性约束）

| 规则          | 说明                       | 失败处理       | 错误键                                |
| ------------- | -------------------------- | -------------- | ------------------------------------- |
| **借贷平衡**  | sum(postings.amount) == 0  | 拒绝创建       | `errors.transaction.unbalanced`       |
| **至少2分录** | postings.length >= 2       | 拒绝创建       | `errors.transaction.minPostings`      |
| **账户存在**  | 所有 account_id 必须有效   | 拒绝创建       | `errors.account.notFound`             |
| **账户激活**  | 所有账户 is_active == true | 拒绝创建或提示 | `errors.account.inactive`             |
| **金额非零**  | 每条分录 amount != 0       | 拒绝创建       | `errors.transaction.zeroAmount`       |
| **账户唯一**  | 同一交易中账户不可重复     | 拒绝创建       | `errors.transaction.duplicateAccount` |

> **错误消息规范**: 所有后端错误消息使用国际化键格式 `"errors.xxx"`，前端负责翻译显示。

### 2.2 验证代码实现

```rust
// src-tauri/src/db/transactions.rs (新建)

use crate::db::AppError;

/// 验证交易输入的基础约束
///
/// **安全设计**: 包含以下验证防止数据损坏：
/// 1. 分录数量 >= 2
/// 2. 借贷平衡
/// 3. 每条分录金额非零
/// 4. 同一交易中账户唯一（防止同一账户出现多次）
pub fn validate_transaction_input(input: &CreateTransactionInput) -> Result<Vec<(String, i64)>, AppError> {
    // 1. 分录数量验证
    if input.postings.len() < 2 {
        return Err(AppError::ValidationError(
            "errors.transaction.minPostings".to_string()
        ));
    }

    // 2. 日期格式验证（ISO 8601）
    validate_date_format(&input.date)?;

    // 3. 解析并计算金额总和
    let mut total_amount: i64 = 0;
    let mut parsed_postings: Vec<(String, i64)> = Vec::new();
    let mut seen_accounts: std::collections::HashSet<String> = std::collections::HashSet::new();

    for posting in &input.postings {
        // 4. 金额解析（十进制字符串 → 分）
        let amount_cents = parse_amount_to_cents(&posting.amount)?;

        // 5. 单条分录金额非零验证
        if amount_cents == 0 {
            return Err(AppError::ValidationError(
                "errors.transaction.zeroAmount".to_string()
            ));
        }

        // 6. **账户唯一性验证**: 同一交易中账户不可重复
        // 防止：同一账户同时出现在借方和贷方（逻辑错误）
        if seen_accounts.contains(&posting.account_id) {
            return Err(AppError::ValidationError(
                "errors.transaction.duplicateAccount".to_string()
            ));
        }
        seen_accounts.insert(posting.account_id.clone());

        total_amount += amount_cents;
        parsed_postings.push((posting.account_id.clone(), amount_cents));
    }

    // 7. 借贷平衡验证（核心复式记账约束）
    if total_amount != 0 {
        return Err(AppError::ValidationError(
            "errors.transaction.unbalanced".to_string()
        ));
    }

    Ok(parsed_postings)
}

/// 验证日期格式（ISO 8601: YYYY-MM-DD）
///
/// **验证规则**:
/// 1. 格式必须为 YYYY-MM-DD
/// 2. 日期不应超过当前日期（防止误操作）
///
/// **设计决策**: 未来日期报错而非警告，因为：
/// - 记账软件记录历史交易，而非未来计划
/// - 未来日期通常是用户输入错误（年份多输一位）
/// - 用户如有特殊需求（如预付账款），可在当天创建交易并备注
fn validate_date_format(date_str: &str) -> Result<(), AppError> {
    let date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
        .map_err(|_| AppError::ValidationError(
            "errors.transaction.invalidDateFormat".to_string()
        ))?;

    // 未来日期验证
    let today = chrono::Utc::now().date_naive();
    if date > today {
        return Err(AppError::ValidationError(
            "errors.transaction.futureDateNotAllowed".to_string()
        ));
    }

    Ok(())
}

/// 十进制字符串金额转换为分（整数）
/// 支持 "100.50"、"100"、"100.00"、"−100.50" 等格式
///
/// **安全设计**: 使用字符串解析而非浮点运算，避免：
/// 1. 大金额溢出（如 "99999999999999.99" 使用浮点会溢出）
/// 2. 浮点精度误差（如 0.1 * 100.0 可能产生 9.99999999999998）
/// 3. 科学计数法攻击（如 "1e100"）
fn parse_amount_to_cents(amount_str: &str) -> Result<i64, AppError> {
    let trimmed = amount_str.trim();

    // 拒绝科学计数法（防止 "1e100" 等攻击）
    if trimmed.contains('e') || trimmed.contains('E') {
        return Err(AppError::ValidationError(
            "errors.transaction.invalidAmountFormat".to_string()
        ));
    }

    // 处理负号（支持 ASCII 和 Unicode 负号）
    let (negative, numeric) = if trimmed.starts_with('-') || trimmed.starts_with('−') {
        (true, &trimmed[1..])
    } else {
        (false, trimmed)
    };

    // 分割整数和小数部分
    let parts: Vec<&str> = numeric.split('.').collect();
    if parts.len() > 2 {
        return Err(AppError::ValidationError(
            "errors.transaction.invalidAmountFormat".to_string()
        ));
    }

    // 解析整数部分
    let integer_part: i64 = parts[0].parse().map_err(|_| {
        AppError::ValidationError("errors.transaction.invalidAmountFormat".to_string())
    })?;

    // 解析小数部分（最多2位）
    let decimal_cents: i64 = if parts.len() == 2 {
        let decimal_str = parts[1];
        if decimal_str.len() > 2 {
            return Err(AppError::ValidationError(
                "errors.transaction.tooManyDecimals".to_string()
            ));
        }
        // 补齐到2位（如 ".5" → "50"）
        let padded = if decimal_str.len() == 1 {
            format!("{}0", decimal_str)
        } else {
            decimal_str.to_string()
        };
        padded.parse().map_err(|_| {
            AppError::ValidationError("errors.transaction.invalidAmountFormat".to_string())
        })?
    } else {
        0
    };

    // 边界检查：i64::MAX / 100 = 92233720368547758 (约 92万亿)
    // 超过此值的整数部分乘100会溢出
    const MAX_SAFE_INTEGER_PART: i64 = i64::MAX / 100;
    if integer_part.abs() > MAX_SAFE_INTEGER_PART {
        return Err(AppError::ValidationError(
            "errors.transaction.amountTooLarge".to_string()
        ));
    }

    // 计算总金额（整数部分 * 100 + 小数部分）
    let total_cents = integer_part * 100 + decimal_cents;

    Ok(if negative { -total_cents } else { total_cents })
}
```

### 2.3 账户验证

```rust
/// 验证所有涉及账户
pub fn validate_accounts(
    conn: &Connection,
    parsed_postings: &[(String, i64)],
    allow_equity: bool,  // 是否允许 equity 账户（仅系统交易）
) -> Result<(), AppError> {
    for (account_id, _) in parsed_postings {
        // 1. 账户存在性
        let exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM accounts WHERE id = ?1",
            [account_id],
            |row| row.get(0),
        )?;

        if !exists {
            return Err(AppError::NotFound("errors.account.notFound".to_string()));
        }

        // 2. 账户激活状态
        let is_active: bool = conn.query_row(
            "SELECT is_active FROM accounts WHERE id = ?1",
            [account_id],
            |row| row.get(0),
        )?;

        if !is_active {
            return Err(AppError::ValidationError(
                "errors.account.inactive".to_string()
            ));
        }

        // 3. 权益账户限制（仅系统交易可用）
        if !allow_equity {
            let account_type: String = conn.query_row(
                "SELECT type FROM accounts WHERE id = ?1",
                [account_id],
                |row| row.get(0),
            )?;

            if account_type == "equity" {
                return Err(AppError::ValidationError(
                    "errors.transaction.equityAccountRestricted".to_string()
                ));
            }
        }
    }

    Ok(())
}
```

---

## 三、核心创建流程

### 3.1 流程步骤

```
创建交易流程
├── Step 1: 验证输入结构（字段完整性）
├── Step 2: 解析金额为整数（分）
├── Step 3: 验证借贷平衡
├── Step 4: 验证账户存在性和状态
├── Step 5: 可选：验证分类存在性
├── Step 6: 可选：验证标签存在性
├── Step 7: 创建交易记录（INSERT transactions）
├── Step 8: 创建分录记录（INSERT postings）
├── Step 9: 关联标签（INSERT transaction_tags）
├── Step 10: 提交事务（COMMIT）
└── Step 11: 返回完整交易对象
```

### 3.2 完整实现代码

```rust
use rusqlite::Connection;
use uuid::Uuid;
use chrono::Utc;

/// 创建交易（核心函数）
pub fn create_transaction(
    conn: &Connection,
    input: &CreateTransactionInput,
) -> Result<TransactionWithPostings, AppError> {
    // Step 1-3: 基础验证
    let parsed_postings = validate_transaction_input(input)?;

    // Step 4: 账户验证（普通用户交易不允许使用 equity 账户）
    validate_accounts(conn, &parsed_postings, false)?;

    // Step 5: 分类验证（如果提供）
    if let Some(ref cat_id) = input.category_id {
        let cat_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM categories WHERE id = ?1",
            [cat_id],
            |row| row.get(0),
        )?;
        if !cat_exists {
            return Err(AppError::NotFound("errors.category.notFound".to_string()));
        }
    }

    // Step 6-10: 数据插入（使用事务保证原子性）
    let tx = conn.transaction()?;

    let tx_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    // 插入交易记录（无 transaction_type 字段）
    tx.execute(
        "INSERT INTO transactions (id, date, description, category_id, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            &tx_id,
            &input.date,
            &input.description,
            &input.category_id,
            &now,
            &now,
        ],
    )?;

    // 插入分录记录
    let mut created_postings: Vec<Posting> = Vec::new();
    for (account_id, amount) in parsed_postings {
        let posting_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&posting_id, &tx_id, &account_id, amount, &now],
        )?;
        created_postings.push(Posting {
            id: posting_id,
            account_id,
            amount,
        });
    }

    // 关联标签（如果有）
    let mut created_tags: Vec<Tag> = Vec::new();
    if let Some(ref tag_ids) = input.tags {
        for tag_id in tag_ids {
            let tag: Option<(String, String, Option<String>)> = tx.query_row(
                "SELECT id, name, color FROM tags WHERE id = ?1",
                [tag_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            ).optional()?;

            if let Some((id, name, color)) = tag {
                tx.execute(
                    "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
                    rusqlite::params![&tx_id, &id],
                )?;
                created_tags.push(Tag { id, name, color });
            }
        }
    }

    tx.commit()?;

    // Step 11: 返回完整交易对象
    Ok(TransactionWithPostings {
        id: tx_id,
        date: input.date.clone(),
        description: input.description.clone(),
        category_id: input.category_id.clone(),
        postings: created_postings,
        tags: created_tags,
        created_at: now.clone(),
        updated_at: now,
    })
}
```

---

## 四、Tauri Command 暴露

```rust
// src-tauri/src/commands/transactions.rs (新建)

use tauri::State;
use crate::db::transactions::{create_transaction, CreateTransactionInput, TransactionWithPostings};
use crate::AppState;

/// 创建交易 Tauri Command
#[tauri::command]
pub async fn create_transaction(
    input: CreateTransactionInput,
    state: State<'_, AppState>,
) -> Result<TransactionWithPostings, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    create_transaction(&conn, &input).map_err(|e| e.to_string())
}
```

---

## 五、前端调用示例

```typescript
// src/utils/api.ts 补充

import { invoke } from "@tauri-apps/api/core";
import type { CreateTransactionInput, TransactionWithPostings } from "@/types";

export async function createTransaction(
  input: CreateTransactionInput,
): Promise<TransactionWithPostings> {
  return invoke("create_transaction", { input });
}

// 使用示例：创建支出交易（用户手动指定 Posting）
const createExpenseTransaction = async () => {
  const result = await createTransaction({
    date: "2024-03-15",
    description: "午餐 - 麦当劳",
    category_id: "cat-food-uuid",
    postings: [
      { account_id: "expense-food-uuid", amount: "35.00" }, // 借方 +35
      { account_id: "asset-cash-uuid", amount: "-35.00" }, // 贷方 -35
    ],
  });
  console.log("Created transaction:", result.id);
};

// 使用示例：多分录交易（拆分支出）
const createSplitTransaction = async () => {
  const result = await createTransaction({
    date: "2024-03-15",
    description: "周末消费",
    postings: [
      { account_id: "expense-food-uuid", amount: "50.00" }, // 借方
      { account_id: "expense-transit-uuid", amount: "20.00" }, // 借方
      { account_id: "asset-cash-uuid", amount: "-70.00" }, // 贷方
    ],
  });
};
```

---

## 六、单元测试设计

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_env() -> (TempDir, Connection) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        init_schema(&conn).unwrap();
        // 插入测试账户和分类
        insert_test_accounts(&conn);
        insert_test_categories(&conn);
        (dir, conn)
    }

    #[test]
    fn test_create_transaction_balanced() {
        let (_dir, conn) = test_env();

        let input = CreateTransactionInput {
            date: "2024-03-15".into(),
            description: "Test transaction".into(),
            category_id: None,
            postings: vec![
                PostingInput { account_id: "acct-asset".into(), amount: "100.00".into() },
                PostingInput { account_id: "acct-expense".into(), amount: "-100.00".into() },
            ],
            tags: None,
        };

        let result = create_transaction(&conn, &input);
        assert!(result.is_ok());
        let tx = result.unwrap();
        assert_eq!(tx.postings.len(), 2);
    }

    #[test]
    fn test_create_transaction_unbalanced_fails() {
        let (_dir, conn) = test_env();

        let input = CreateTransactionInput {
            date: "2024-03-15".into(),
            description: "Unbalanced".into(),
            category_id: None,
            postings: vec![
                PostingInput { account_id: "acct-1".into(), amount: "100.00".into() },
                PostingInput { account_id: "acct-2".into(), amount: "-50.00".into() },
            ],
            tags: None,
        };

        let result = create_transaction(&conn, &input);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::ValidationError(_)));
    }

    #[test]
    fn test_create_transaction_multi_postings() {
        let (_dir, conn) = test_env();

        // 多分录交易（拆分支出）
        let input = CreateTransactionInput {
            date: "2024-03-15".into(),
            description: "周末消费".into(),
            postings: vec![
                PostingInput { account_id: "expense-food".into(), amount: "50.00".into() },
                PostingInput { account_id: "expense-transit".into(), amount: "20.00".into() },
                PostingInput { account_id: "asset-cash".into(), amount: "-70.00".into() },
            ],
            tags: None,
            category_id: None,
        };

        let result = create_transaction(&conn, &input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().postings.len(), 3);
    }

    #[test]
    fn test_parse_amount_decimal() {
        assert_eq!(parse_amount_to_cents("100.50").unwrap(), 10050);
        assert_eq!(parse_amount_to_cents("-100.50").unwrap(), -10050);
        assert_eq!(parse_amount_to_cents("100").unwrap(), 10000);
        assert_eq!(parse_amount_to_cents("0.01").unwrap(), 1);
        assert_eq!(parse_amount_to_cents("−50").unwrap(), -5000);  // Unicode负号
    }

    #[test]
    fn test_single_posting_fails() {
        let (_dir, conn) = test_env();

        let input = CreateTransactionInput {
            date: "2024-03-15".into(),
            description: "Single posting".into(),
            postings: vec![
                PostingInput { account_id: "acct-1".into(), amount: "100.00".into() },
            ],
            tags: None,
            category_id: None,
        };

        assert!(create_transaction(&conn, &input).is_err());
    }
}
```

---

## 七、关键设计决策

| 决策                 | 说明                                                     |
| -------------------- | -------------------------------------------------------- |
| **金额存储为整数**   | 避免浮点精度问题，前端传字符串，后端解析为分             |
| **借贷平衡硬性约束** | `sum(postings.amount) == 0`，否则拒绝创建                |
| **纯复式记账**       | 不存储交易类型，用户完全控制 Posting 组合                |
| **事务原子性**       | 使用 SQLite transaction 保证交易+分录一起成功或失败 |
| **分类可选**         | category_id 可选，用于支出/收入交易的分类归属            |
| **权益账户限制**     | 禁止用户交易使用 equity 账户，仅系统交易可用             |

> **注**: 本系统不需要添加 `transaction_type` 列，所有交易均为纯复式记账。
