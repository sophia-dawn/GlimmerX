# 20 - CSV 数据导入

> 返回 [DESIGN.md](../DESIGN.md)

## 概述

CSV 导入功能支持从外部文件批量导入交易数据，与导出功能形成完整的数据流转闭环。

| 功能 | ID | 优先级 | 说明 |
|---|---|---|---|
| CSV 导入 | DATA-2 | P1 | 从 CSV 文件导入交易，支持账户自动匹配和重复检测 |

---

## 一、设计原则

### 1.1 与导出格式对称

导入功能设计为导出功能的逆向操作：

| 导出字段 | 导入处理 |
|---|---|
| `transaction_id` | 用于分组同一交易的多条分录 |
| `date` | 直接使用 |
| `description` | 直接使用 |
| `currency` | 账户货币校验 |
| `account` | 查询/创建账户（name） |
| `account_type` | 查询账户（type + name） |
| `amount` | 直接使用（整数，分） |
| `category` | 查询分类（name），可选 |
| `reconciled` | 导入后需单独更新 |

### 1.2 业界参考

参考以下开源项目的导入实现：

| 项目 | 关键模式 |
|---|---|
| **Beancount** | 列配置映射（Col enum），支持自定义列角色 |
| **Firefly III** | 运行时 JSON 映射 + SHA-256 重复检测 |
| **Ledger-cli** | SHA-512 哈希链，内容指纹 |

---

## 二、导入流程

### 2.1 用户流程

```
设置页 → "导入交易"按钮 → 文件选择对话框 → 预览导入数据 → 确认导入 → 结果提示
```

### 2.2 处理流程

```
CSV 导入流程
├── Step 1: 解析 CSV 文件（csv crate）
├── Step 2: 验证文件格式（必需列是否存在）
├── Step 3: 按 transaction_id 分组行
├── Step 4: 解析每行数据 → ImportRow 结构
├── Step 5: 账户匹配/创建（按 type + name）
├── Step 6: 分类匹配（按 type + name，可选）
├── Step 7: 验证每笔交易（双分录平衡）
├── Step 8: 重复检测（SHA-256 内容哈希）
├── Step 9: 批量创建交易（事务内）
└── Step 10: 返回导入结果
```

---

## 三、CSV 文件格式

### 3.1 必需格式

导入功能支持与导出功能相同的 CSV 格式：

```csv
transaction_id,date,description,currency,account,account_type,amount,category,reconciled
550e8400-e29b-41d4-a716-446655440001,2024-01-15,超市购物,CNY,餐饮,expense,2000,餐饮,false
550e8400-e29b-41d4-a716-446655440001,2024-01-15,超市购物,CNY,我的资产,asset,-2000,餐饮,false
```

### 3.2 列定义

| 列名 | 必需 | 类型 | 说明 |
|---|---|---|---|
| `transaction_id` | ✅ | String | 用于分组同一交易的多条分录 |
| `date` | ✅ | String | YYYY-MM-DD 格式 |
| `description` | ✅ | String | 交易描述 |
| `currency` | ✅ | String | 货币代码（CNY、USD） |
| `account` | ✅ | String | 账户名称 |
| `account_type` | ✅ | String | 账户类型（asset/liability/income/expense） |
| `amount` | ✅ | Integer | 金额（分），正=借方，负=贷方 |
| `category` | ❌ | String | 分类名称（可选） |
| `reconciled` | ❌ | Boolean | 对账状态（可选，默认 false） |

### 3.3 格式验证规则

1. **Header 验证**：必需列必须存在
2. **行数验证**：至少 2 行数据（一条完整交易）
3. **类型验证**：
   - `date` 必须是有效 YYYY-MM-DD
   - `amount` 必须是有效整数
   - `account_type` 必须是有效类型（asset/liability/income/expense/equity）
4. **交易完整性**：每个 transaction_id 必须有 ≥2 条分录

---

## 四、账户匹配策略

### 4.1 匹配逻辑

采用 **查询优先，创建为辅** 策略：

```rust
// 1. 尝试查询现有账户
let account = find_account_by_type_and_name(conn, account_type, account_name)?;

// 2. 如果不存在，创建新账户（可选，用户配置）
if account.is_none() && create_missing_accounts {
    let new_id = create_account_with_path(
        conn,
        &format!("{}/{}", account_type, account_name),
        currency,
        None,
    )?;
    account = find_account_by_type_and_name(conn, account_type, account_name)?;
}
```

### 4.2 账户创建规则

| 条件 | 行为 |
|---|---|
| 账户已存在 | 使用现有账户 ID |
| 账户不存在 + `create_missing=true` | 自动创建新账户 |
| 账户不存在 + `create_missing=false` | 报错，跳过该交易 |

### 4.3 特殊账户类型

| 类型 | 处理 |
|---|---|
| `equity` | **禁止导入**，报错（仅系统交易可用） |
| `income` | 允许导入，自动创建收入类账户 |
| `expense` | 允许导入，自动创建支出类账户 |

---

## 五、交易分组与验证

### 5.1 分组逻辑

CSV 文件中同一 `transaction_id` 的多行属于同一交易：

```rust
// 按 transaction_id 分组
let mut transactions: HashMap<String, Vec<ImportRow>> = HashMap::new();
for row in csv_rows {
    transactions.entry(row.transaction_id).or_default().push(row);
}
```

### 5.2 双分录验证

每笔交易必须满足复式记账规则：

| 规则 | 验证 |
|---|---|
| **最少分录** | ≥2 条 posting |
| **借贷平衡** | `sum(postings.amount) == 0` |
| **账户唯一** | 同一交易中账户不可重复 |
| **金额非零** | 每条分录 `amount != 0` |

### 5.3 验证代码

```rust
fn validate_imported_transaction(rows: &[ImportRow]) -> Result<Vec<PostingInput>, String> {
    // 1. 最少分录
    if rows.len() < 2 {
        return Err("errors.transactionMinPostings");
    }

    // 2. 金额非零
    for row in rows {
        if row.amount == 0 {
            return Err("errors.transaction.zeroAmount");
        }
    }

    // 3. 借贷平衡
    let sum: i64 = rows.iter().map(|r| r.amount).sum();
    if sum != 0 {
        return Err("errors.transactionUnbalanced");
    }

    // 4. 账户唯一
    let accounts: HashSet<(String, String)> = rows.iter()
        .map(|r| (r.account_type.clone(), r.account.clone()))
        .collect();
    if accounts.len() != rows.len() {
        return Err("errors.transaction.duplicateAccount");
    }

    Ok(rows.iter().map(|r| PostingInput {
        account_id: resolve_account_id(r),
        amount: r.amount,
    }).collect())
}
```

---

## 六、重复检测

### 6.1 内容哈希法

采用 Firefly III 的 SHA-256 内容哈希法：

```rust
fn compute_transaction_hash(date: &str, description: &str, postings: &[PostingInput]) -> String {
    let canonical = format!(
        "{}|{}|{}",
        date,
        description,
        postings.iter()
            .map(|p| format!("{}:{}", p.account_id, p.amount))
            .sorted()
            .join("|")
    );
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}
```

### 6.2 重复检测流程

```rust
// 1. 计算导入交易的哈希
let import_hash = compute_transaction_hash(&date, &description, &postings);

// 2. 查询数据库中相同哈希的交易
let existing = find_transaction_by_hash(conn, &import_hash)?;

// 3. 处理重复
if existing.is_some() && skip_duplicates {
    continue; // 跳过重复交易
}
```

### 6.3 用户配置

| 配置项 | 选项 | 默认 |
|---|---|---|
| `skip_duplicates` | true / false | true（跳过重复） |
| `overwrite_duplicates` | true / false | false（不覆盖） |

### 6.4 哈希存储方案

**方案选择：添加 transaction_hashes 表**

不修改 transactions 表（避免破坏现有结构），新增独立映射表：

```sql
CREATE TABLE IF NOT EXISTS transaction_hashes (
    transaction_id TEXT PRIMARY KEY REFERENCES transactions(id) ON DELETE CASCADE,
    content_hash TEXT NOT NULL UNIQUE,    -- SHA-256 内容哈希
    import_source TEXT,                   -- 导入来源文件名（可选）
    imported_at TEXT NOT NULL,            -- 导入时间 RFC3339
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_transaction_hashes_hash ON transaction_hashes(content_hash);
```

**设计理由：**
- 解耦设计：哈希数据与交易数据分离，便于清理和扩展
- 快速查询：通过 content_hash 索引快速检测重复
- 来源追踪：记录导入来源，便于审计
- **不破坏现有 transactions 表结构**

### 6.5 现有交易迁移方案

**问题：现有交易没有 transaction_hashes 记录**

**解决方案：导入功能首次使用时自动迁移（分批执行）**

```rust
/// 检查并初始化现有交易的哈希记录（分批迁移）
fn ensure_existing_hashes(conn: &Connection) -> Result<u32, String> {
    // 1. 检查是否已有哈希记录（首次使用检测）
    let hash_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transaction_hashes",
        [],
        |row| row.get(0)
    )?;
    
    if hash_count > 0 {
        return Ok(0); // 已迁移，跳过
    }
    
    // 2. 统计需要迁移的交易总数
    let total_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM transactions WHERE deleted_at IS NULL",
        [],
        |row| row.get(0)
    )?;
    
    // 3. 分批迁移（每批 500 笔）
    const BATCH_SIZE: i32 = 500;
    let mut migrated = 0;
    
    for offset in (0..total_count as i32).step_by(BATCH_SIZE as usize) {
        let batch_txns = query_transactions_batch(conn, offset, BATCH_SIZE)?;
        
        let tx = conn.transaction()?;
        for (txn_id, date, description, postings) in batch_txns {
            let hash = compute_transaction_hash(&date, &description, &postings);
            let now = now_rfc3339();
            
            tx.execute(
                "INSERT INTO transaction_hashes (transaction_id, content_hash, import_source, imported_at, created_at)
                 VALUES (?1, ?2, NULL, ?3, ?3)",
                params![txn_id, hash, now]
            )?;
            migrated += 1;
        }
        tx.commit()?;
    }
    
    Ok(migrated)
}

/// 分批查询交易 + postings
fn query_transactions_batch(
    conn: &Connection,
    offset: i32,
    limit: i32,
) -> Result<Vec<(String, String, String, Vec<PostingRecord>)>, String> {
    // 查询分页交易
    let txns = conn.prepare("
        SELECT id, date, description FROM transactions 
        WHERE deleted_at IS NULL 
        ORDER BY created_at 
        LIMIT ?1 OFFSET ?2
    ")?;
    
    // 对每笔交易查询 postings
    // ... 
}
```

**迁移时机：**
- **首次调用导入功能时**自动执行
- 用户无感知，不影响现有功能
- 一次性迁移，后续调用直接跳过
- **分批执行**，避免内存溢出和超时

**性能考虑：**
- 每 500 笔交易一批，控制内存使用
- 每批独立事务，失败可重试
- 进度可显示给用户（可选）
    }
    
    // 2. 查询所有现有交易及其分录
    let transactions = query_all_transactions_with_postings(conn)?;
    
    // 3. 批量计算哈希并插入
    let tx = conn.transaction()?;
    let mut count = 0;
    
    for (txn_id, date, description, postings) in transactions {
        let hash = compute_transaction_hash(&date, &description, &postings);
        let now = now_rfc3339();
        
        tx.execute(
            "INSERT INTO transaction_hashes (transaction_id, content_hash, import_source, imported_at, created_at)
             VALUES (?1, ?2, NULL, ?3, ?3)",
            params![txn_id, hash, now]
        )?;
        count += 1;
    }
    
    tx.commit()?;
    Ok(count)
}
```

**迁移时机：**
- **首次调用导入功能时**自动执行
- 用户无感知，不影响现有功能
- 一次性迁移，后续调用直接跳过

**迁移 SQL（备选方案）：**

```sql
-- 单次执行的迁移脚本
INSERT INTO transaction_hashes (transaction_id, content_hash, imported_at, created_at)
SELECT 
    t.id,
    -- 哈希计算需要应用层完成，SQLite 无法直接计算 SHA-256
    'pending_migration' as content_hash,  -- 临时标记
    datetime('now', 'localtime') as imported_at,
    datetime('now', 'localtime') as created_at
FROM transactions t
WHERE t.deleted_at IS NULL
AND NOT EXISTS (SELECT 1 FROM transaction_hashes WHERE transaction_id = t.id);
```

> **注意：** SQLite 不支持内置 SHA-256 函数，必须通过 Rust 应用层计算哈希后批量插入。

### 6.6 备选方案：FITID 式匹配（借鉴 KMyMoney）

KMyMoney 使用 OFX FITID（金融机构交易 ID）而非内容哈希：

| 方法 | 优点 | 缺点 |
|---|---|---|
| **SHA-256 内容哈希** | 数据修改也能检测 | 金额微调后哈希变化 |
| **FITID 匹配** | 银行原始 ID | 需导入源提供唯一 ID |

**GlimmerX 选择：SHA-256 内容哈希**，因为：
- CSV 导出包含 transaction_id，可作为 FITID
- 但用户可能修改 CSV（如调整金额），内容哈希更可靠
- 支持从无 transaction_id 的外部 CSV 导入

---

## 七、API 定义

### 7.1 Tauri Command

```rust
#[tauri::command]
pub async fn import_transactions_csv(
    input_path: String,
    create_missing_accounts: bool,   // 是否自动创建缺失账户
    skip_duplicates: bool,           // 是否跳过重复交易
    state: State<'_, AppState>,
) -> Result<ImportResult, String>;
```

### 7.2 返回结构

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub imported_count: u32,       // 成功导入的交易数
    pub skipped_count: u32,        // 跳过的交易数（重复）
    pub error_count: u32,          // 错误的交易数
    pub created_accounts: Vec<String>, // 新创建的账户名称列表
    pub errors: Vec<ImportError>,  // 错误详情列表
}

#[derive(Serialize)]
pub struct ImportError {
    pub row_number: u32,           // CSV 行号
    pub transaction_id: String,    // 相关 transaction_id
    pub message: String,           // 错误消息
}
```

### 7.3 导入行结构

```rust
#[derive(Debug, Deserialize)]
pub struct ImportRow {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub currency: String,
    pub account: String,           // 账户名称
    pub account_type: String,      // 账户类型
    pub amount: i64,
    pub category: Option<String>,
    pub reconciled: Option<bool>,
}
```

### 7.4 reconciled 字段处理

**问题：** `create_transaction()` API 不接受 `is_reconciled` 参数，交易默认创建为未核对状态。

**解决方案：导入后批量更新**

```rust
// 导入完成后，批量更新 reconciled 状态
fn update_reconciled_status(conn: &Connection, updates: &[(&str, bool)]) -> Result<(), String> {
    let tx = conn.transaction()?;
    for (transaction_id, is_reconciled) in updates {
        tx.execute(
            "UPDATE transactions SET is_reconciled = ?1, updated_at = ?2 WHERE id = ?3",
            params![is_reconciled as i32, now_rfc3339(), transaction_id],
        )?;
    }
    tx.commit()?;
    Ok(())
}
```

**流程调整：**
1. 先调用 `create_transaction()` 创建交易（默认 reconciled=false）
2. 收集所有需要更新的 `(transaction_id, reconciled)` 对
3. 导入完成后，单次批量 UPDATE

---

## 八、前端界面

### 8.1 设置页面新增

```text
┌───────────────────────────────────────────────┐
│  数据管理                                      │
│  ┌───────────────────────────────────────────┐│
│  │ [备份数据库]                               ││
│  │                                           ││
│  │ [导入交易(CSV)]                            ││
│  │   □ 自动创建缺失账户                       ││
│  │   □ 跳过重复交易                           ││
│  │                                           ││
│  │ 日期范围: [本月 ▼]                        ││
│  │                                           ││
│  │ [导出交易(CSV)]    [导出交易(Beancount)]  ││
│  └───────────────────────────────────────────┘│
└───────────────────────────────────────────────┘
```

### 8.2 导入预览对话框

导入前显示预览，让用户确认：

```text
┌───────────────────────────────────────────────┐
│  导入预览                              [取消] │
├───────────────────────────────────────────────┤
│  文件: transactions_2024.csv                  │
│  共 28 行，14 笔交易                          │
│                                               │
│  ✓ 可导入: 12 笔交易                          │
│  ⚠ 重复跳过: 2 笔交易                         │
│  ❌ 错误: 0 笔交易                             │
│                                               │
│  新账户将创建:                                │
│  • asset/支付宝                               │
│  • expense/交通                               │
│                                               │
│              [确认导入]    [取消]              │
└───────────────────────────────────────────────┘
```

### 8.3 导入结果提示

```text
导入完成：
✓ 成功导入 12 笔交易（24 条分录）
⚠ 跳过 2 笔重复交易
• 新建账户：支付宝、交通
```

---

## 九、后端实现

### 9.1 新增文件

```
src-tauri/src/
├── commands/
│   ├── data.rs       # 新增：import_transactions_csv 命令
│   └── mod.rs        # 注册 data 模块
├── db/
│   ├── import.rs     # 新增：导入逻辑实现
│   └── mod.rs        # 注册 import 模块
```

### 9.2 新增依赖

```toml
# Cargo.toml
[dependencies]
sha2 = "0.10"     # SHA-256 哈希
hex = "0.4"       # 哈希编码
```

### 9.3 核心实现代码（已应用性能优化）

```rust
// src-tauri/src/db/import.rs

use csv::ReaderBuilder;
use rusqlite::Connection;
use sha2::{Sha256, Digest};
use std::collections::{HashMap, HashSet};

/// 导入 CSV 文件中的交易（性能优化版）
pub fn import_csv(
    conn: &Connection,
    input_path: &Path,
    create_missing_accounts: bool,
    skip_duplicates: bool,
) -> Result<ImportResult, String> {
    // 0. 确保现有交易已迁移哈希（首次使用时）
    ensure_existing_hashes(conn)?;
    
    // 1. 解析 CSV（流式读取）
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .buffer_capacity(8192)
        .from_path(input_path)
        .map_err(|e| format!("无法读取文件: {}", e))?;

    // 2. 收集所有行（分组后处理）
    let rows: Vec<ImportRow> = rdr.deserialize()
        .collect::<Result<Vec<_>, csv::Error>>()
        .map_err(|e| format!("CSV 解析错误: {}", e))?;

    // 3. 验证必需列
    validate_csv_headers(&rows)?;

    // 4. 按 transaction_id 分组
    let mut transactions: HashMap<String, Vec<ImportRow>> = HashMap::new();
    for row in &rows {
        transactions.entry(row.transaction_id.clone()).or_default().push(row);
    }

    // 5. === 性能优化：批量预查询 ===
    
    // 5a. 批量收集所有需要的账户
    let needed_accounts: HashSet<(String, String)> = rows.iter()
        .map(|r| (r.account_type.clone(), r.account.clone()))
        .collect();
    
    // 5b. 批量查询账户（避免 N+1）
    let account_map = batch_query_accounts(conn, &needed_accounts)?;
    
    // 5c. 批量计算所有哈希（内存操作）
    let mut import_hashes: HashMap<String, String> = HashMap::new();
    for (txn_id, txn_rows) in &transactions {
        // 先验证双分录
        if validate_imported_transaction(txn_rows).is_ok() {
            let postings = resolve_postings_from_rows(txn_rows, &account_map);
            let hash = compute_transaction_hash(
                &txn_rows[0].date,
                &txn_rows[0].description,
                &postings,
            );
            import_hashes.insert(txn_id.clone(), hash);
        }
    }
    
    // 5d. 批量查询已存在的哈希（避免 N+1）
    let all_hashes: HashSet<String> = import_hashes.values().cloned().collect();
    let existing_hashes = if skip_duplicates {
        batch_query_existing_hashes(conn, &all_hashes)?
    } else {
        HashSet::new()
    };

    // 6. 处理每笔交易（分批提交）
    let mut result = ImportResult::default();
    let mut pending_reconciled: Vec<(String, bool)> = Vec::new();
    
    const BATCH_SIZE: usize = 100;
    let mut batch_count = 0;
    let mut tx = conn.transaction().map_err(|e| e.to_string())?;

    for (txn_id, txn_rows) in transactions {
        // 验证双分录
        if let Err(e) = validate_imported_transaction(&txn_rows) {
            result.errors.push(ImportError {
                row_number: 0,
                transaction_id: txn_id,
                message: e,
            });
            result.error_count += 1;
            continue;
        }

        // 解析账户和创建缺失账户
        let postings = resolve_and_create_postings(
            &tx, &txn_rows, create_missing_accounts, &account_map, &mut result.created_accounts
        )?;

        // 重复检测（内存比对，无数据库查询）
        let hash = import_hashes.get(&txn_id).unwrap();
        if existing_hashes.contains(hash) {
            result.skipped_count += 1;
            continue;
        }

        // 创建交易
        let first_row = &txn_rows[0];
        let created_txn_id = create_imported_transaction(
            &tx,
            &first_row.date,
            &first_row.description,
            first_row.category.as_deref(),
            &postings,
            hash,
        )?;
        
        // 收集 reconciled 状态
        if first_row.reconciled == Some(true) {
            pending_reconciled.push((created_txn_id, true));
        }
        
        result.imported_count += 1;
        batch_count += 1;

        // 每 100 笔交易提交一次
        if batch_count >= BATCH_SIZE {
            tx.commit().map_err(|e| e.to_string())?;
            update_reconciled_status(conn, &pending_reconciled)?;
            insert_transaction_hashes(conn, &pending_reconciled.iter()
                .filter_map(|(id, _)| import_hashes.get(id)).collect::<HashSet<_>>())?;
            
            pending_reconciled.clear();
            tx = conn.transaction().map_err(|e| e.to_string())?;
            batch_count = 0;
        }
    }

    // 提交剩余交易
    if batch_count > 0 {
        tx.commit().map_err(|e| e.to_string())?;
        update_reconciled_status(conn, &pending_reconciled)?;
    }
    
    Ok(result)
}
```

### 9.4 性能优化设计

**问题：大文件导入（10000+ 行）可能超时或内存溢出**

**优化策略：**

| 优化点 | 方案 |
|---|---|
| **CSV 解析** | 流式读取，不一次性加载所有行 |
| **账户查询** | 批量预查询，减少 N+1 问题 |
| **数据库提交** | 分批提交（每 100 笔交易） |
| **哈希存储** | 延迟写入，批量 INSERT |

#### 9.4.1 流式 CSV 解析

```rust
// 使用 csv::Reader 的迭代器模式，不一次性加载
let mut rdr = ReaderBuilder::new()
    .has_headers(true)
    .buffer_capacity(8192)  // 8KB 缓冲区
    .from_path(input_path)?;

// 迭代处理，不收集所有行
for result in rdr.deserialize() {
    let row: ImportRow = result?;
    // 处理单行...
}
```

#### 9.4.2 批量账户预查询

```rust
/// 批量查询所有需要的账户，避免 N+1 问题
fn batch_query_accounts(
    conn: &Connection,
    needed_accounts: &HashSet<(String, String)>,  // (type, name)
) -> Result<HashMap<(String, String), String>, String> {  // (type, name) -> id
    let mut account_map = HashMap::new();
    
    // 单次查询所有匹配的账户
    for (account_type, account_name) in needed_accounts {
        let result: Option<String> = conn.query_row(
            "SELECT id FROM accounts WHERE type = ?1 AND name = ?2",
            params![account_type, account_name],
            |row| row.get(0)
        ).optional().map_err(|e| e.to_string())?;
        
        if let Some(id) = result {
            account_map.insert((account_type.clone(), account_name.clone()), id);
        }
    }
    
    Ok(account_map)
}

// 导入流程中使用：
// 1. 收集所有需要的账户
let needed_accounts: HashSet<(String, String)> = rows.iter()
    .map(|r| (r.account_type.clone(), r.account.clone()))
    .collect();

// 2. 批量查询（单次）
let account_map = batch_query_accounts(conn, &needed_accounts)?;

// 3. 导入时直接使用 map
for row in &rows {
    let account_id = account_map.get(&(row.account_type, &row.account));
}
```

#### 9.4.3 批量哈希查询（关键优化）

**问题：每笔交易单独查询哈希 = N 次数据库查询**

**优化：批量计算 + 批量查询**

```rust
/// 批量查询已存在的哈希，避免 N+1
fn batch_query_existing_hashes(
    conn: &Connection,
    hashes: &HashSet<String>,
) -> Result<HashSet<String>, String> {
    // 构建 IN 查询
    let placeholders: Vec<String> = hashes.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT content_hash FROM transaction_hashes WHERE content_hash IN ({})",
        placeholders.join(",")
    );
    
    let mut stmt = conn.prepare(&sql)?;
    let hash_refs: Vec<&String> = hashes.iter().collect();
    
    let existing: HashSet<String> = stmt
        .query_map(rusqlite::params_from_iter(hash_refs), |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(|e| e.to_string())?;
    
    Ok(existing)
}

// 导入流程中使用：
// 1. 先计算所有导入交易的哈希（内存操作）
let import_hashes: HashMap<String, String> = transactions.iter()
    .map(|(txn_id, rows)| {
        let postings = resolve_postings(rows);
        (txn_id.clone(), compute_transaction_hash(&rows[0].date, &rows[0].description, &postings))
    })
    .collect();

// 2. 批量查询已存在的哈希（单次数据库查询）
let all_hashes: HashSet<String> = import_hashes.values().cloned().collect();
let existing_hashes = batch_query_existing_hashes(conn, &all_hashes)?;

// 3. 导入时直接比对（无数据库查询）
for (txn_id, rows) in transactions {
    let hash = import_hashes.get(&txn_id).unwrap();
    if existing_hashes.contains(hash) {
        result.skipped_count += 1;
        continue;  // 跳过重复
    }
    // 创建交易...
}
```

#### 9.4.4 分批提交策略

```rust
const BATCH_SIZE: usize = 100;

let mut batch_count = 0;
let mut pending_reconciled: Vec<(String, bool)> = Vec::new();

for (txn_id, txn_rows) in transactions {
    // 创建交易...
    
    // 收集 reconciled 状态
    if txn_rows[0].reconciled == Some(true) {
        pending_reconciled.push((created_txn_id.clone(), true));
    }
    
    batch_count += 1;
    
    // 每 100 笔交易提交一次
    if batch_count >= BATCH_SIZE {
        tx.commit()?;
        update_reconciled_status(conn, &pending_reconciled)?;
        pending_reconciled.clear();
        tx = conn.transaction()?;
        batch_count = 0;
    }
}

// 提交剩余交易
if batch_count > 0 {
    tx.commit()?;
    update_reconciled_status(conn, &pending_reconciled)?;
}
```

---

## 十、错误处理

### 10.1 错误类型

| 错误场景 | 用户提示（中文） |
|---|---|
| 文件读取失败 | "无法读取文件：{原因}" |
| CSV 格式错误 | "CSV 格式错误：第 {N} 行" |
| 必需列缺失 | "CSV 缺少必需列：{列名}" |
| 账户不存在 | "账户不存在：{账户名}" |
| 借贷不平衡 | "交易不平衡：{transaction_id}" |
| 权益账户禁止 | "权益账户禁止导入" |
| 日期格式错误 | "日期格式错误：{日期}" |

### 10.2 错误收集策略

采用 **批量收集 + 单次报告** 策略：

1. 不中断导入流程，收集所有错误
2. 最终返回完整错误列表
3. 用户可查看具体哪行/哪笔交易出错

---

## 十一、测试要点

### 11.1 单元测试

| 测试项 | 验证内容 |
|---|---|
| CSV 解析 | 正确解析各列，处理空值 |
| 分组逻辑 | transaction_id 正确分组 |
| 账户匹配 | 查询/创建逻辑正确 |
| 双分录验证 | 平衡校验、账户唯一校验 |
| 重复检测 | SHA-256 哈希正确计算 |

### 11.2 边界测试

| 场景 | 验证 |
|---|---|
| 空 CSV | 返回错误 |
| 单行 CSV | 返回错误（最少2分录） |
| 不平衡交易 | 返回错误 |
| 10000+ 行 | 性能测试，不超时 |
| UTF-8 中文账户名 | 正确处理 |

### 11.3 集成测试

| 测试 | 验证 |
|---|---|
| 导出→导入循环 | 数据一致性 |
| 重复导入 | 正确跳过 |
| 新账户创建 | 账户余额正确 |

---

## 十二、国际化

### 12.1 新增翻译键

```json
// src/i18n/locales/zh.json
{
  "settings": {
    "importCsv": "导入交易(CSV)",
    "importPreview": "导入预览",
    "importSuccess": "导入成功：{{count}} 笔交易",
    "importSkipped": "跳过 {{count}} 笔重复交易",
    "importErrors": "{{count}} 笔交易导入失败",
    "createMissingAccounts": "自动创建缺失账户",
    "skipDuplicates": "跳过重复交易",
    "newAccountsCreated": "新建账户：{{accounts}}"
  },
  "errors": {
    "importFailed": "导入失败：{原因}",
    "csvMissingColumn": "CSV 缺少必需列：{列名}",
    "csvParseError": "CSV 解析错误：第 {row} 行",
    "accountNotFoundForImport": "账户不存在：{账户名}（类型：{类型})"
  }
}
```

---

## 十三、与导出功能的关系

### 13.1 数据流闭环

```
导出 → CSV 文件 → 导入 → 数据库
```

设计确保：

- 导出的 CSV 可以被导入功能完整解析
- 导入后数据与原数据一致（除 transaction_id 可能变化）

### 13.2 transaction_id 处理

| 场景 | 处理 |
|---|---|
| 导入到同一账本 | 保持原 transaction_id（用于重复检测） |
| 导入到新账本 | 生成新 transaction_id |

---

## 十四、未来扩展

| 功能 | 说明 |
|---|---|
| Beancount 导入 | 解析 Beancount 格式导入 |
| 导入规则 | 自动应用分类规则 |
| 多文件导入 | 批量导入多个 CSV 文件 |
| 导入模板 | 保存导入配置供复用 |

---

## 十五、性能对比总结

### 15.1 查询次数对比

假设导入 **1000 笔交易，每笔 2 条分录，涉及 20 个账户**：

| 操作 | 优化前 | 优化后 | 减少 |
|---|---|---|---|
| 账户查询 | 2000 次 (每分录) | 20 次 (批量) | **99%** |
| 哈希查询 | 1000 次 (每交易) | 1 次 (批量) | **99.9%** |
| 迁移哈希 | 1 次 (全部) | 分批（每 500 笔） | 内存安全 |
| 数据库提交 | 1 次 | 分批（每 100 笔） | 超时安全 |

### 15.2 性能瓶颈分析

| 阶段 | 主要耗时 | 优化后耗时 |
|---|---|---|
| CSV 解析 | O(n) 读取 | O(n) 流式 |
| 账户查询 | O(n) × 数据库延迟 | O(m) 其中 m = 账户数 |
| 哈希计算 | O(n) SHA-256 | O(n) 内存操作 |
| 哈希查询 | O(n) × 数据库延迟 | O(1) 批量 IN 查询 |
| 交易插入 | O(n) 事务开销 | 分批提交 |

### 15.3 预期性能

| 数据规模 | 优化前预估 | 优化后预估 |
|---|---|---|
| 100 笔交易 | ~2 秒 | <1 秒 |
| 1000 笔交易 | ~20 秒 | ~3 秒 |
| 10000 笔交易 | 超时失败 | ~30 秒 |

> **注意：** 优化后性能主要取决于：
> 1. SHA-256 计算（内存操作，快速）
> 2. 磁盘 I/O（CSV 读取 + 数据库写入）
> 3. SQLite 事务开销（分批后显著降低） |
