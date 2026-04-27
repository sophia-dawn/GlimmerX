# 19 - 数据导出与备份

> 返回 [DESIGN.md](../DESIGN.md)

## 概述

数据管理模块提供数据库备份和交易数据导出功能，支持 CSV 和 Beancount 格式。

| 功能 | ID | 优先级 | 说明 |
|---|---|---|---|
| 数据库备份 | DATA-4 | P0 | 一键复制加密数据库文件 |
| CSV 导出 | DATA-1 | P0 | 交易数据导出为 CSV（多行格式） |
| Beancount 导出 | DATA-3 | P1 | 交易数据导出为 Beancount 纯文本格式 |

> **注意**: 数据库恢复无需单独功能，用户可直接通过"打开已有账本"打开备份文件。

---

## 一、数据库备份（DATA-4）

### 用户流程

```
设置页 → "备份数据库"按钮 → 文件保存对话框 → 备份完成提示
```

### API 定义

```rust
#[tauri::command]
pub async fn db_backup(
    backup_path: String,
    state: State<'_, AppState>,
) -> Result<(), String>;
```

### 实现逻辑

1. **WAL checkpoint**：确保所有数据写入主数据库文件
2. **文件复制**：`std::fs::copy(current_db_path, backup_path)`
3. **密码保持**：备份文件使用原密码加密，无需额外处理

### 错误处理

| 错误场景 | 用户提示 |
|---|---|
| 磁盘空间不足 | "备份失败：磁盘空间不足" |
| 文件路径无效 | "备份失败：无法创建文件" |
| 数据库未解锁 | "请先解锁账本后再备份" |
| 权限不足 | "备份失败：无写入权限" |

---

## 二、CSV 导出（DATA-1）

### CSV 文件格式

采用**多行格式**（每条 posting 一行），同一交易的多分录通过 `transaction_id` 关联：

```csv
transaction_id,date,description,currency,account,amount,category,reconciled
550e8400-e29b-41d4-a716-446655440001,2024-01-15,超市购物,CNY,asset/银行卡,-128,食品,0
550e8400-e29b-41d4-a716-446655440001,2024-01-15,超市购物,CNY,expense/食品,128,食品,0
550e8400-e29b-41d4-a716-446655440002,2024-02-01,工资收入,CNY,asset/银行卡,8000,工资,1
550e8400-e29b-41d4-a716-446655440002,2024-02-01,工资收入,CNY,income/工资,-8000,工资,1
```

### 字段说明

| 字段 | 类型 | 说明 |
|---|---|---|
| `transaction_id` | UUID | 交易唯一标识（同交易多行相同） |
| `date` | YYYY-MM-DD | 交易日期 |
| `description` | String | 交易描述 |
| `currency` | String | 货币代码（CNY、USD 等） |
| `account` | String | 账户完整路径（如 `asset/银行卡`） |
| `amount` | Integer | 金额（分），正=借方，负=贷方 |
| `category` | String | 分类名称（空表示无分类） |
| `reconciled` | 0/1 | 对账状态：0=未核对，1=已核对 |

### 导出范围

- **默认**：所有未删除交易（`deleted_at IS NULL`）
- **可选筛选**：日期范围（start_date、end_date）

### API 定义

```rust
#[tauri::command]
pub async fn export_transactions_csv(
    output_path: String,
    start_date: Option<String>,    // YYYY-MM-DD
    end_date: Option<String>,      // YYYY-MM-DD
    state: State<'_, AppState>,
) -> Result<ExportResult, String>;

#[derive(Serialize)]
pub struct ExportResult {
    pub transaction_count: u32,    // 导出的交易数量
    pub posting_count: u32,        // 导出的分录数量
}
```

### 实现要点

1. **数据查询**：JOIN transactions + postings + accounts + categories
2. **排序规则**：按 `transactions.date` ASC，同交易内按 `postings.sequence` ASC
3. **CSV 写入**：使用 Rust `csv` crate + `BufWriter`（流式处理，避免内存溢出）
4. **金额保持**：整数（分），不做格式转换
5. **性能优化**：流式导出（边读边写），利用 SQL ORDER BY 避免冗余排序

---

## 三、Beancount 导出（DATA-3）

### Beancount 文件格式

遵循 Beancount 标准语法，包含账户 open 指令和交易记录：

```beancount
; GlimmerX Export - Generated at 2026-04-26T10:30:00+08:00
option "operating_currency" "CNY"

; === Accounts ===
2024-01-01 open Assets:银行卡
2024-01-01 open Expenses:食品
2024-01-01 open Income:工资

; === Transactions ===
2024-01-15 * "超市购物"
  Assets:银行卡      -128.00 CNY
  Expenses:食品       128.00 CNY

2024-02-01 * "工资收入"
  Assets:银行卡      8000.00 CNY
  Income:工资       -8000.00 CNY
```

### 转换规则

| GlimmerX | Beancount | 说明 |
|---|---|---|
| `asset/银行卡` | `Assets:银行卡` | 冒号分隔，首字母大写 |
| `expense/食品` | `Expenses:食品` | 同上 |
| `income/工资` | `Income:工资` | 同上 |
| `liability/信用卡` | `Liabilities:信用卡` | 同上 |
| `equity/...` | `Equity:...` | 同上 |
| `amount=-128`（分） | `-128.00 CNY` | 分→元转换，加货币符号 |
| `amount=8000`（分） | `8000.00 CNY` | 同上 |
| `is_reconciled=1` | flag=`*` | 已核对=完成标志 |
| `is_reconciled=0` | flag=`!` | 未核对=待处理标志 |

### 账户命名规范（Beancount）

Beancount 要求账户以五大类型开头，且使用冒号分隔层级：

| 根账户 | 类型符号 | 典型用途 |
|---|---|---|
| `Assets` | +（正余额） | 银行、现金、投资 |
| `Liabilities` | -（负余额） | 信用卡、贷款 |
| `Income` | -（收入减少净资产） | 工资、利息收入 |
| `Expenses` | +（支出增加） | 餐饮、交通 |
| `Equity` | -（权益调整） | 期初余额、结转 |

### API 定义

```rust
#[tauri::command]
pub async fn export_transactions_beancount(
    output_path: String,
    start_date: Option<String>,
    end_date: Option<String>,
    state: State<'_, AppState>,
) -> Result<ExportResult, String>;
```

### 实现要点

1. **账户收集**：提取所有涉及的账户，生成 `open` 指令
2. **账户日期**：取第一条交易日期或年初（YYYY-01-01）
3. **金额转换**：`amount / 100.0`，格式化为 `{amount:.2} {currency}`
4. **账户名转换**：`{type}/{name}` → `{Type}:{name}`（首字母大写）
5. **文件写入**：`BufWriter` 流式写入（减少 syscall，避免内存溢出）
6. **性能优化**：利用 SQL ORDER BY，边读边写，无冗余排序

---

## 四、前端界面

### 设置页面新增区块

```text
┌───────────────────────────────────────────────┐
│  数据管理                                      │
│  ┌───────────────────────────────────────────┐│
│  │ [备份数据库]                               ││
│  │ 导出当前账本到指定位置                     ││
│  │                                           ││
│  │ 日期范围: [本月 ▼]                        ││
│  │ 开始日期: [2024-01-01] 结束日期: [今日]   ││
│  │                                           ││
│  │ [导出交易(CSV)]    [导出交易(Beancount)]  ││
│  └───────────────────────────────────────────┘│
└───────────────────────────────────────────────┘
```

### 日期范围预设

| 预设值 | 说明 |
|---|---|
| 本月 | 当前月份（1日-月末） |
| 本年 | 当前年份（1月1日-12月31日） |
| 全部 | 不限制日期范围 |
| 自定义 | 用户选择开始和结束日期 |

### 交互流程

1. 点击按钮 → `dialog.save()` 获取保存路径
2. 调用对应 API → 显示 loading 状态
3. 导出完成 → 显示成功提示："已导出 XX 条交易，XX 条分录"
4. 导出失败 → 显示错误提示（后端返回的错误信息）

---

## 五、后端实现

### 新增文件结构

```
src-tauri/src/
├── commands/
│   ├── data.rs       # 新增：db_backup, export_transactions_csv, export_transactions_beancount
│   └── mod.rs        # 注册 data 模块
├── db/
│   ├── export.rs     # 新增：导出逻辑实现
│   └── mod.rs        # 注册 export 模块
```

### 新增依赖

```toml
# Cargo.toml
[dependencies]
csv = "1.3"  # CSV 读写
```

### 数据查询 SQL

```sql
-- 导出交易数据（CSV/Beancount 共用）
SELECT
    t.id as transaction_id,
    t.date,
    t.description,
    t.is_reconciled,
    a.currency,
    a.name as account_name,
    p.amount,
    c.name as category_name
FROM transactions t
JOIN postings p ON p.transaction_id = t.id
JOIN accounts a ON a.id = p.account_id
LEFT JOIN categories c ON c.id = t.category_id
WHERE t.deleted_at IS NULL
  AND (?1 IS NULL OR t.date >= ?1)
  AND (?2 IS NULL OR t.date <= ?2)
ORDER BY t.date ASC, t.created_at ASC, p.sequence ASC;
```

---

## 六、测试要点

### 单元测试

| 测试项 | 验证内容 |
|---|---|
| `db_backup` | 文件复制成功、WAL checkpoint 执行 |
| CSV 格式生成 | 字段顺序正确、金额为整数、多行关联正确 |
| Beancount 格式生成 | 账户名转换正确、金额元格式正确、open 指令生成 |

### 边界测试

| 场景 | 验证 |
|---|---|
| 无交易数据 | 导出空文件（仅 header） |
| 大量交易（10000+） | 导出不超时、内存稳定（流式处理） |
| 多币种交易 | currency 字段正确、Beancount 正确标注 |
| 无分类交易 | category 字段为空 |

---

## 七、错误处理

### 错误码定义

| 错误码 | 说明 | 用户提示 |
|---|---|---|
| `errors.backupFailed` | 备份失败 | "备份失败：{原因}" |
| `errors.exportFailed` | 导出失败 | "导出失败：{原因}" |
| `errors.databaseLocked` | 数据库未解锁 | "请先解锁账本后再操作" |
| `errors.invalidDateRange` | 日期范围无效 | "日期范围无效：开始日期不能晚于结束日期" |

### 前端错误展示

遵循项目规则：**后端返回友好消息，前端直接显示**。

```typescript
// 前端调用示例
try {
  const result = await invoke<ExportResult>("export_transactions_csv", {
    outputPath: path,
    startDate: startDate || null,
    endDate: endDate || null,
  });
  toast.success(`已导出 ${result.transactionCount} 条交易，${result.postingCount} 条分录`);
} catch (error) {
  toast.error(error as string);  // 直接显示后端返回的错误消息
}
```

---

## 八、国际化

### 新增翻译键

```json
// src/i18n/locales/zh.json
{
  "settings": {
    "dataManagement": "数据管理",
    "backupDatabase": "备份数据库",
    "backupDatabaseDesc": "导出当前账本到指定位置",
    "exportTransactions": "导出交易",
    "exportCsv": "导出交易(CSV)",
    "exportBeancount": "导出交易(Beancount)",
    "dateRange": "日期范围",
    "thisMonth": "本月",
    "thisYear": "本年",
    "allTime": "全部",
    "custom": "自定义",
    "startDate": "开始日期",
    "endDate": "结束日期",
    "exportSuccess": "已导出 {{transactionCount}} 条交易，{{postingCount}} 条分录"
  },
  "errors": {
    "backupFailed": "备份失败",
    "exportFailed": "导出失败",
    "databaseLocked": "请先解锁账本后再操作"
  }
}
```