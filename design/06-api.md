# 06 - Tauri Commands 接口

> 返回 [DESIGN.md](../DESIGN.md)

## 数据库管理

```rust
#[tauri::command]
async fn db_create(password: String, path: String) -> Result<DbInfo, String>;
#[tauri::command]
async fn db_unlock(password: String, path: String) -> Result<DbInfo, String>;
#[tauri::command]
async fn db_is_unlocked() -> Result<bool, String>;
#[tauri::command]
async fn db_change_password(old_password: String, new_password: String) -> Result<(), String>;
#[tauri::command]
async fn db_check_exists(path: String) -> Result<bool, String>;
#[tauri::command]
async fn db_check_any_exists() -> Result<bool, String>;
#[tauri::command]
async fn db_list_recent() -> Result<Vec<RecentDbEntry>, String>;
#[tauri::command]
async fn db_remove_recent(path: String) -> Result<(), String>;
#[tauri::command]
async fn db_lock() -> Result<(), String>;
#[tauri::command]
async fn db_backup(backup_path: String) -> Result<(), String>;
#[tauri::command]
async fn db_restore(backup_path: String, password: String) -> Result<(), String>;
```

> **`db_is_unlocked`**: 检查数据库是否已解锁（连接存在于 AppState）。用于页面刷新后恢复解锁状态。

## 账户

### 创建账户

```rust
#[tauri::command]
async fn account_create(input: CreateAccountInput) -> Result<AccountDto, String>;
```

**`CreateAccountInput`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | `String` | 路径格式：`{type}/{账户名}`，如 `asset/招商银行` |
| `currency` | `Option<String>` | 货币，默认 CNY |
| `initial_balance` | `Option<String>` | 期初余额（十进制字符串，如 "1000.50"） |
| `initial_balance_date` | `Option<String>` | 期初日期（ISO 8601） |
| `description` | `Option<String>` | 备注 |
| `account_number` | `Option<String>` | 账户号 |
| `iban` | `Option<String>` | 国际银行账户号码 |
| `is_active` | `Option<bool>` | 是否激活 |
| `include_net_worth` | `Option<bool>` | 是否计入净资产 |
| `meta` | `Option<HashMap<String, String>>` | 扩展字段（见下方） |
| `equity_account_name` | `Option<String>` | 权益根账户名（默认 "Equity"） |
| `opening_balance_name` | `Option<String>` | 期初余额账户名（默认 "Opening Balances"） |

> **路径格式说明**: `name` 必须以账户类型开头，格式为 `{type}/{账户名}`。后端会从路径首段提取类型。
> **Equity 拦截**: 路径首段为 `equity` 或 `equities`（不区分大小写）则返回错误——权益账户由系统管理。
> **期初余额**: 若 `initial_balance` 非 0，自动创建一笔"Opening Balance"交易，包含双分录（账户借方 + 权益贷方）。

### 元数据键（meta 字段）

| meta 键                | 适用类型        | 值说明                                                                                 |
| ---------------------- | --------------- | -------------------------------------------------------------------------------------- |
| `account_role`         | asset           | 子类型：`defaultAsset` / `sharedAsset` / `savingAsset` / `ccAsset` / `cashWalletAsset` |
| `liability_type`       | liability       | 子类型：`loan` / `debt` / `mortgage`                                                   |
| `credit_card_type`     | asset (ccAsset) | 信用卡类型：`monthlyFull`                                                              |
| `monthly_payment_date` | asset (ccAsset) | 还款日（日期字符串）                                                                   |
| `interest`             | liability       | 利率（百分比字符串）                                                                   |
| `interest_period`      | liability       | 利息周期：`monthly` / `yearly`                                                         |

### 更新账户

```rust
#[tauri::command]
async fn account_update(id: String, input: UpdateAccountInput) -> Result<AccountDto, String>;
```

**`UpdateAccountInput`** 字段（全部可选）：
| 字段 | 说明 |
|------|------|
| `name` | 账户名 |
| `description` | 备注 |
| `account_number` | 账户号 |
| `initial_balance` | 期初余额（十进制字符串） |
| `initial_balance_date` | 期初日期 |
| `iban` | IBAN |
| `is_active` | 激活状态 |
| `include_net_worth` | 是否计入净资产 |
| `meta` | 扩展字段（提供时会替换全部现有 meta） |

> **动态更新**: 仅更新提供的字段，未提供的保持不变。
> **期初余额更新**: 若 asset/liability 类型提供 `initial_balance`，自动更新或创建 Opening Balance 交易。

### 其他账户命令

```rust
#[tauri::command]
async fn account_list() -> Result<Vec<AccountDto>, String>;
#[tauri::command]
async fn account_delete(id: String) -> Result<(), String>;
#[tauri::command]
async fn account_balance(id: String) -> Result<i64, String>;
#[tauri::command]
async fn account_transfer(from_id: String, to_id: String, amount: String, description: String) -> Result<String, String>;
#[tauri::command]
async fn account_batch_create(inputs: Vec<CreateAccountInput>) -> Result<Vec<AccountDto>, String>;
#[tauri::command]
async fn account_transactions(id: String, from_date: Option<String>, to_date: Option<String>) -> Result<Vec<AccountTransaction>, String>;
#[tauri::command]
async fn account_meta_get(account_id: String) -> Result<Vec<AccountMeta>, String>;
#[tauri::command]
async fn account_meta_set(account_id: String, key: String, value: String) -> Result<(), String>;
#[tauri::command]
async fn account_meta_batch_set(account_id: String, metas: Vec<(String, String)>) -> Result<(), String>;
#[tauri::command]
async fn account_meta_schema() -> Result<AccountMetaSchema, String>;
```

### 返回类型

**`AccountDto`**: 账户响应对象，包含 `id`, `name`, `account_type`, `currency`, `description`, `account_number`, `is_system`, `iban`, `is_active`, `include_net_worth`, `created_at`, `updated_at`, `meta: Vec<AccountMeta>`, `initial_balance`, `initial_balance_date`。

**`AccountTransaction`**: 账户交易记录，包含 `id`, `date`, `description`, `category_id`, `amount`（分，正=借方，负=贷方）, `created_at`, `updated_at`。

**`AccountMetaSchema`**: 元数据验证选项，包含 `valid_account_roles: Vec<String>`, `valid_liability_types: Vec<String>`。

> **余额计算**: `account_balance` 返回该账户所有 postings.amount 的原始 SUM（分），不按账户类型翻转符号。
> **删除保护**: 若账户有用户交易（非 Opening Balance），删除会被阻止。

## 交易

### 交易类型与验证矩阵

GlimmerX 支持可选的 `transaction_type` 字段，用于「快速记账」场景和报表统计。

**交易类型定义**：
| 类型 | 含义 | 典型场景 |
|------|------|----------|
| `withdrawal` | 支出 | 从资产账户花钱到支出分类 |
| `deposit` | 收入 | 从收入分类收款到资产账户 |
| `transfer` | 转账 | 两个资产/负债账户间转移 |
| `opening_balance` | 期初余额 | 系统生成，设置账户初始值 |
| `reconciliation` | 对账调整 | 系统生成，银行对账差异 |

**验证矩阵**：定义 `TransactionType × AccountType` 的合法组合，用于前端账户过滤和后端可选验证。

```typescript
// source: 借方账户（金额增加）允许的类型
// destination: 贷方账户（金额减少）允许的类型
const VALID_COMBINATIONS = {
  withdrawal: {
    source: ["asset", "liability"], // 资金来源
    destination: ["expense"], // 支出目标
  },
  deposit: {
    source: ["income"], // 收入来源
    destination: ["asset", "liability"], // 资金去向
  },
  transfer: {
    source: ["asset", "liability"], // 转出账户
    destination: ["asset", "liability"], // 转入账户
  },
  opening_balance: {
    source: ["equity"],
    destination: ["asset", "liability"],
  },
  reconciliation: {
    source: ["equity"],
    destination: ["asset"],
  },
};
```

> **设计说明**: `transaction_type` 为可选字段。`NULL` 表示自由复式记账，用户可手动指定任意 Posting 组合；非 `NULL` 时后端可验证 Posting 账户类型是否符合矩阵，但不强制。

### 交易命令

```rust
#[tauri::command]
async fn transaction_create(input: CreateTransactionInput) -> Result<Transaction, String>;
#[tauri::command]
async fn transaction_get(id: String) -> Result<TransactionWithPostings, String>;
#[tauri::command]
async fn transaction_list(filter: TransactionFilter) -> Result<Vec<TransactionListItem>, String>;
#[tauri::command]
async fn transaction_update(id: String, input: UpdateTransactionInput) -> Result<Transaction, String>;
#[tauri::command]
async fn transaction_delete(id: String) -> Result<(), String>;
#[tauri::command]
async fn transaction_search(query: String) -> Result<Vec<TransactionListItem>, String>;
#[tauri::command]
async fn transaction_quick_add(input: QuickAddInput) -> Result<Transaction, String>;
#[tauri::command]
async fn transaction_type_validation() -> Result<TransactionTypeValidation, String>;  // 获取验证矩阵
```

### 创建交易输入

**`CreateTransactionInput`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `date` | `String` | 交易日期（ISO 8601） |
| `description` | `String` | 交易描述 |
| `category_id` | `Option<String>` | 分类 ID |
| `transaction_type` | `Option<String>` | 交易类型（可选） |
| `postings` | `Vec<PostingInput>` | 分录列表 |

**`PostingInput`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `account_id` | `String` | 账户 ID |
| `amount` | `String` | 金额（十进制字符串，如 "-100.50"） |

### 快速记账

**`QuickAddInput`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `transaction_type` | `String` | 交易类型：`withdrawal` / `deposit` / `transfer` |
| `amount` | `String` | 金额（十进制字符串） |
| `source_account_id` | `Option<String>` | 来源账户（withdrawal 时为资产账户） |
| `destination_account_id` | `Option<String>` | 目标账户（withdrawal 时为支出分类） |
| `category_id` | `Option<String>` | 分类 ID（可替代 destination_account） |
| `description` | `Option<String>` | 备注 |
| `date` | `Option<String>` | 日期（默认今天） |

> **自动推导**: 快速记账根据 `transaction_type` 自动生成 Posting：
>
> - `withdrawal`: source账户-金额 + expense账户+金额
> - `deposit`: income账户-金额 + destination账户+金额
> - `transfer`: source账户-金额 + destination账户+金额

### 返回类型

**`TransactionTypeValidation`**: 验证矩阵响应，包含 `valid_combinations: HashMap<String, ValidCombination>`，每个组合定义 `source_types` 和 `destination_types`。

## 分类

### 分类删除约束

- **有预算绑定时** → 阻止删除，提示先删除预算
- **有交易关联时** → 解除关联，关联交易的 `category_id` 将置为 NULL，交易保留
- **删除预览**：可通过 `category_delete_preview` 命令查看关联数量

### 分类命令

```rust
#[tauri::command]
async fn category_list(category_type: Option<String>) -> Result<Vec<Category>, String>;
#[tauri::command]
async fn category_create(input: CreateCategoryInput) -> Result<Category, String>;
#[tauri::command]
async fn category_update(id: String, input: UpdateCategoryInput) -> Result<Category, String>;
#[tauri::command]
async fn category_delete_preview(id: String) -> Result<DeletePreview, String>;
#[tauri::command]
async fn category_delete(id: String, cascade: bool) -> Result<(), String>;
```

### 创建分类输入

**`CreateCategoryInput`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | `String` | 分类名称（同一 type 下唯一） |
| `type` | `String` | 分类类型：`income`/`expense`（必填） |
| `icon` | `Option<String>` | 图标（可选，可设为 `null` 清除） |

### 更新分类输入

**`UpdateCategoryInput`** 字段（全部可选）：
| 字段 | 说明 |
|------|------|
| `name` | 分类名称 |
| `icon` | 图标（`null` 表示清除，字段缺失表示不修改） |

> **类型不可修改**：分类创建后 `type` 不可变更，避免破坏预算和统计一致性。
> **图标三态**：`Some(Some(v))` = 设置图标，`Some(None)` = 清除图标，`None` = 不修改。

### 删除预览返回类型

**`DeletePreview`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `budgetCount` | `i64` | 关联的预算数量 |
| `transactionCount` | `i64` | 关联的交易数量 |
| `canDelete` | `bool` | 是否可删除（budgetCount > 0 时为 false） |

### 删除参数

**`category_delete` 参数**：
| 参数 | 类型 | 说明 |
|------|------|------|
| `id` | `String` | 分类 ID |
| `cascade` | `bool` | 是否级联删除（默认 false） |

> **cascade 行为**：
>
> - `cascade=false`：有预算时阻止删除，有交易时解除关联
> - `cascade=true`：删除预算，解除交易关联，删除分类

## 预算

```rust
#[tauri::command]
async fn budget_list(period: String) -> Result<Vec<BudgetStatus>, String>;
#[tauri::command]
async fn budget_set(input: SetBudgetInput) -> Result<Budget, String>;
#[tauri::command]
async fn budget_delete(id: String) -> Result<(), String>;
```

## Dashboard（概览）

Dashboard 模块提供首页仪表盘所需的所有聚合数据。

### Dashboard 命令

```rust
#[tauri::command]
async fn dashboard_summary(
    from_date: Option<String>,
    to_date: Option<String>
) -> Result<DashboardSummary, String>;

#[tauri::command]
async fn dashboard_monthly_chart(
    year: Option<i32>,
    month: Option<i32>
) -> Result<MonthlyChartData, String>;

#[tauri::command]
async fn dashboard_category_breakdown(
    year: Option<i32>,
    month: Option<i32>,
    category_type: Option<String>
) -> Result<CategoryBreakdownData, String>;

#[tauri::command]
async fn dashboard_top_expenses(
    year: Option<i32>,
    month: Option<i32>,
    limit: Option<i32>
) -> Result<TopExpensesData, String>;
```

### Dashboard 返回类型

**`DashboardSummary`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `month_income` | `i64` | 当月收入总额（cents） |
| `month_expense` | `i64` | 当月支出总额（cents） |
| `month_start` | `String` | 当月起始日期 |
| `month_end` | `String` | 当月结束日期 |
| `year_income` | `i64` | 当年收入总额（cents） |
| `year_expense` | `i64` | 当年支出总额（cents） |
| `year_start` | `String` | 当年起始日期 |
| `year_end` | `String` | 当年结束日期 |
| `total_assets` | `i64` | 资产总额（cents） |
| `total_liabilities` | `i64` | 债务总额（cents） |
| `net_worth` | `i64` | 净资产（cents） |
| `calculated_at` | `String` | 计算时间戳（RFC3339） |

> **年度范围**: 固定使用当前年份（1月1日 - 12月31日），而非基于传入日期推导。
> **净资产计算**: 仅统计 `include_net_worth=1` 且 `is_active=1` 的账户。

**`MonthlyChartData`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `year` | `i32` | 年份 |
| `month` | `i32` | 月份（1-12） |
| `days` | `Vec<DailyIncomeExpense>` | 每日收支数据 |
| `month_total_income` | `i64` | 月度收入合计（cents） |
| `month_total_expense` | `i64` | 月度支出合计（cents） |

**`DailyIncomeExpense`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `date` | `String` | 日期（YYYY-MM-DD） |
| `income` | `i64` | 当日收入（cents） |
| `expense` | `i64` | 当日支出（cents） |
| `has_transactions` | `bool` | 当日是否有交易 |

**`CategoryBreakdownData`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `year` | `i32` | 年份 |
| `month` | `i32` | 月份 |
| `category_type` | `String` | 分类类型（income/expense） |
| `categories` | `Vec<CategoryAmount>` | 分类金额列表 |
| `total_amount` | `i64` | 总金额（cents） |

**`CategoryAmount`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `category_id` | `String` | 分类 ID |
| `category_name` | `String` | 分类名称 |
| `icon` | `Option<String>` | 分类图标 |
| `amount` | `i64` | 金额（cents） |
| `percentage` | `f64` | 占比百分比 |

**`TopExpensesData`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `year` | `i32` | 年份 |
| `month` | `i32` | 月份 |
| `expenses` | `Vec<TopExpenseItem>` | Top 支出列表 |

**`TopExpenseItem`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `transaction_id` | `String` | 交易 ID |
| `date` | `String` | 交易日期 |
| `description` | `String` | 交易描述 |
| `amount` | `i64` | 金额（cents） |
| `category_id` | `Option<String>` | 分类 ID |
| `category_name` | `Option<String>` | 分类名称 |
| `category_icon` | `Option<String>` | 分类图标 |

> **默认参数**:
> - 未传入 year/month 时，使用当前年月
> - 未传入 limit 时，默认返回 Top 10
> - category_type 默认为 expense

## 报表

报表模块提供 9 个 Tauri 命令，所有报表共用 `ReportFilter` 参数结构。

### ReportFilter 结构

```rust
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFilter {
    pub date_range_preset: DateRangePreset,   // CurrentMonth, LastMonth, CurrentYear, LastYear, Last3Months, Last6Months, Last12Months, Custom
    pub start_date: Option<String>,           // Required when preset = Custom (YYYY-MM-DD)
    pub end_date: Option<String>,             // Required when preset = Custom (YYYY-MM-DD)
    pub period_granularity: PeriodGranularity, // Daily, Weekly, Monthly, Yearly
    pub account_ids: Option<Vec<String>>,     // 篩选账户
    pub category_ids: Option<Vec<String>>,    // 篩选分类
}
```

### 报表命令列表

```rust
// P0 核心报表
#[tauri::command]
async fn report_standard(filter: ReportFilter) -> Result<StandardReportDto, String>;
#[tauri::command]
async fn report_category_breakdown(filter: ReportFilter, income_or_expense: String) -> Result<CategoryBreakdownReportDto, String>;
#[tauri::command]
async fn report_balance_sheet(snapshot_date: String) -> Result<BalanceSheetReportDto, String>;
#[tauri::command]
async fn report_trend(filter: ReportFilter) -> Result<TrendReportDto, String>;

// P1 重要报表
#[tauri::command]
async fn report_month_comparison(month1: String, month2: String) -> Result<MonthComparisonReportDto, String>;
#[tauri::command]
async fn report_year_summary(year: i32) -> Result<YearSummaryReportDto, String>;
#[tauri::command]
async fn report_account_transactions(account_id: String, filter: ReportFilter, page: u32, page_size: u32) -> Result<AccountTransactionsReportDto, String>;
#[tauri::command]
async fn report_account_balance_trend(account_id: String, filter: ReportFilter) -> Result<AccountBalanceTrendReportDto, String>;

// P2 扩展报表
#[tauri::command]
async fn report_audit() -> Result<AuditReportDto, String>;
```

### 返回类型概览

| 命令 | 返回类型 | 主要字段 |
|------|----------|----------|
| `report_standard` | `StandardReportDto` | `period_income`, `period_expense`, `prev_income`, `prev_expense`, `income_change_pct`, `expense_change_pct`, `net_worth_trend`, `account_changes` |
| `report_category_breakdown` | `CategoryBreakdownReportDto` | `total_amount`, `categories` (含 `category_id`, `category_name`, `amount`, `percentage`, `transaction_count`) |
| `report_balance_sheet` | `BalanceSheetReportDto` | `snapshot_date`, `assets`, `liabilities`, `total_assets`, `total_liabilities`, `net_worth` |
| `report_trend` | `TrendReportDto` | `granularity`, `data_points` (含 `period`, `income`, `expense`, `net`), `total_income`, `total_expense`, `total_net` |
| `report_month_comparison` | `MonthComparisonReportDto` | `month1`, `month2`, `month1_income`, `month1_expense`, `month2_income`, `month2_expense`, `income_diff`, `expense_diff`, `category_comparison` |
| `report_year_summary` | `YearSummaryReportDto` | `year`, `total_income`, `total_expense`, `net`, `monthly_breakdown`, `top_income_categories`, `top_expense_categories` |
| `report_account_transactions` | `AccountTransactionsReportDto` | `account_id`, `account_name`, `transactions`, `total_inflow`, `total_outflow`, `net_change`, `total_count`, `total_pages` |
| `report_account_balance_trend` | `AccountBalanceTrendReportDto` | `account_id`, `account_name`, `granularity`, `data_points` (含 `period`, `balance`) |
| `report_audit` | `AuditReportDto` | `balance_check`, `category_check`, `account_usage`, `generated_at` |

> **金额单位**: 所有金额字段均为 `i64` 类型，单位为 cents（分），前端需使用 `formatAmount()` 转换显示。
> **预算执行报告**: 属于预算模块，使用 `report_budget_execution` 命令（见预算章节）。

## 设置与数据管理

### 设置命令

```rust
#[tauri::command]
async fn setting_get(key: String) -> Result<String, String>;
#[tauri::command]
async fn setting_set(key: String, value: String) -> Result<(), String>;
```

### 数据备份与导出

```rust
// 数据库备份（DATA-4）
#[tauri::command]
async fn db_backup(backup_path: String) -> Result<(), String>;

// CSV 导出（DATA-1）
#[tauri::command]
async fn export_transactions_csv(
    output_path: String,
    start_date: Option<String>,    // YYYY-MM-DD
    end_date: Option<String>       // YYYY-MM-DD
) -> Result<ExportResult, String>;

// Beancount 导出（DATA-3）
#[tauri::command]
async fn export_transactions_beancount(
    output_path: String,
    start_date: Option<String>,
    end_date: Option<String>
) -> Result<ExportResult, String>;
```

### 导出返回类型

**`ExportResult`** 字段：
| 字段 | 类型 | 说明 |
|------|------|------|
| `transaction_count` | `u32` | 导出的交易数量 |
| `posting_count` | `u32` | 导出的分录数量 |

> **设计详情**: 见 [19-data-export](19-data-export.md)。
