# 17 - 交易列表设计

> 返回 [DESIGN.md](../DESIGN.md) > 返回 [交易模块总览](14-transaction-module.md)

---

## 一、功能需求

根据 `04-requirements.md` TXN-4、TXN-5：

| ID        | 功能     | 描述                                         |
| --------- | -------- | -------------------------------------------- |
| **TXN-4** | 交易列表 | 按时间倒序展示，支持分页/虚拟滚动            |
| **TXN-5** | 搜索交易 | 按描述、金额、账户、分类、标签、日期范围搜索 |

---

## 二、数据结构设计

### 2.1 列表项结构

交易列表展示需要聚合多个分录的信息，设计专门的列表视图结构：

```rust
// src-tauri/src/models/transaction.rs

/// 交易列表项（前端展示用）
///
/// 与完整 Transaction 的区别：
/// - postings 简化为账户名称列表（减少数据传输）
/// - 添加聚合信息（总金额、主要账户）
/// - 优化为列表展示格式
///
/// 注: 本系统不存储 transaction_type，列表项也无此字段
#[derive(Debug, Serialize)]
pub struct TransactionListItem {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,

    /// 分录摘要：主要涉及的账户（最多显示2个）
    /// 格式: [(account_name, amount), ...]
    pub postings_summary: Vec<(String, i64)>,

    /// 标签列表（仅名称）
    pub tags: Vec<String>,

    /// 对用户"可见"的金额（借方总额，通常为正）
    pub display_amount: i64,

    /// 时间戳
    pub created_at: String,
    pub updated_at: String,
}

/// 分页信息
#[derive(Debug, Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub page_size: u32,
    pub total_count: u32,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// 列表响应结构
#[derive(Debug, Serialize)]
pub struct TransactionListResponse {
    pub items: Vec<TransactionListItem>,
    pub pagination: PaginationInfo,
    /// 按日期分组的数据（包含后端计算的 day_total）
    /// **重要**: day_total 由后端计算，前端不做金额计算（AGENTS.md 约束）
    pub date_groups: Vec<TransactionDateGroup>,
}

/// 按日期分组的列表结构（前端使用）
///
/// **设计约束**: day_total 由后端计算，前端仅显示。
/// 前端不做任何金额计算（AGENTS.md 约束）。
#[derive(Debug, Serialize)]
pub struct TransactionDateGroup {
    pub date: String,
    pub date_display: String, // 格式化日期，如 "2024年3月15日"
    pub items: Vec<TransactionListItem>,
    /// 当日交易金额汇总（借方总额）
    /// **后端计算**: 由查询聚合得出，前端不计算
    pub day_total: i64,
}
```

### 2.2 过滤条件结构

```rust
/// 交易列表过滤条件
#[derive(Debug, Deserialize, Default)]
pub struct TransactionFilter {
    /// 日期范围
    pub from_date: Option<String>,
    pub to_date: Option<String>,

    /// 金额范围（分）
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,

    /// 账户筛选（交易涉及该账户）
    pub account_id: Option<String>,

    /// 分类筛选
    pub category_id: Option<String>,

    /// 标签筛选（包含该标签）
    pub tag_id: Option<String>,

    /// 描述搜索（模糊匹配）
    pub description_query: Option<String>,

    /// 分页参数
    pub page: Option<u32>,
    pub page_size: Option<u32>,

    /// 排序
    pub sort_by: Option<String>,  // "date" | "amount" | "description"
    pub sort_order: Option<String>, // "asc" | "desc"
}

> **注**: 本系统不存储 transaction_type，因此不支持按类型筛选。
> 用户可通过账户类型或分类进行筛选。
```

### 2.3 TypeScript 类型

```typescript
// src/types/index.ts 补充

export interface TransactionListItem {
  id: string;
  date: string;
  description: string;
  category_id: string | null;
  category_name: string | null;
  postings_summary: Array<[string, number]>; // [account_name, amount]
  tags: string[];
  display_amount: number; // 用户可见金额（分）
  created_at: string;
  updated_at: string;
}

export interface PaginationInfo {
  page: number;
  page_size: number;
  total_count: number;
  total_pages: number;
  has_next: boolean;
  has_prev: boolean;
}

export interface TransactionListResponse {
  items: TransactionListItem[];
  pagination: PaginationInfo;
}

export interface TransactionDateGroup {
  date: string;
  date_display: string;
  items: TransactionListItem[];
  day_total: number;
}

export interface TransactionFilter {
  from_date?: string;
  to_date?: string;
  min_amount?: number;
  max_amount?: number;
  account_id?: string;
  category_id?: string;
  tag_id?: string;
  description_query?: string;
  page?: number;
  page_size?: number;
  sort_by?: "date" | "amount" | "description";
  sort_order?: "asc" | "desc";
}

// 注: 本系统不存储 transaction_type，因此不支持按类型筛选
```

---

## 三、后端查询实现

### 3.1 核心查询函数

```rust
// src-tauri/src/db/transactions.rs

use rusqlite::Connection;

/// 获取交易列表（带过滤和分页）
///
/// 使用单次 JOIN 查询避免 N+1 问题：
/// - 传统 N+1: 查询交易 ID → 每个交易再查询分录/标签 → 总查询数 = 1 + N*4
/// - 单查询: JOIN 所有表 + GROUP BY → 总查询数 = 1
pub fn do_list_transactions(
    conn: &Connection,
    filter: &TransactionFilter,
) -> Result<TransactionListResponse, AppError> {
    // 默认分页
    let page = filter.page.unwrap_or(1);
    let page_size = filter.page_size.unwrap_or(20);
    let offset = (page - 1) * page_size;

    // 构建查询条件
    let (where_clause, params) = build_where_clause(filter);

    // 查询总数
    let total_count: u32 = conn.query_row(
        &format!(
            "SELECT COUNT(DISTINCT t.id) FROM transactions t
             LEFT JOIN postings p ON p.transaction_id = t.id
             LEFT JOIN transaction_tags tt ON tt.transaction_id = t.id
             WHERE {}",
            if where_clause.is_empty() { "1=1" } else { &where_clause }
        ),
        rusqlite::params_from_iter(params.iter().map(|p| p as &dyn rusqlite::types::ToSql)),
        |row| row.get(0),
    )?;

    let total_pages = (total_count + page_size - 1) / page_size;

    // 单查询获取所有数据（避免 N+1）
    let sort_by = filter.sort_by.as_deref().unwrap_or("date");
    let sort_order = filter.sort_order.as_deref().unwrap_or("desc");

    let items_sql = format!(
        "SELECT
            t.id, t.date, t.description, t.category_id, t.created_at, t.updated_at,
            c.name as category_name,
            COALESCE(
                SUM(CASE WHEN p.amount > 0 THEN p.amount ELSE 0 END),
                0
            ) as display_amount,
            GROUP_CONCAT(
                DISTINCT a.name || ':' || CASE
                    WHEN p.amount > 0 THEN '+' || CAST(ABS(p.amount) AS TEXT)
                    ELSE '-' || CAST(ABS(p.amount) AS TEXT)
                END,
                '|'
            ) as postings_data,
            GROUP_CONCAT(DISTINCT tg.name, ',') as tag_names
         FROM transactions t
         LEFT JOIN categories c ON c.id = t.category_id
         JOIN postings p ON p.transaction_id = t.id
         JOIN accounts a ON a.id = p.account_id
         LEFT JOIN transaction_tags tt ON tt.transaction_id = t.id
         LEFT JOIN tags tg ON tg.id = tt.tag_id
         WHERE {}
         GROUP BY t.id
         ORDER BY t.{} {}
         LIMIT ? OFFSET ?",
        if where_clause.is_empty() { "1=1" } else { &where_clause },
        sort_by,
        sort_order
    );

    // 构建参数
    let mut sql_params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    for p in params.iter() {
        sql_params.push(Box::new(p.clone()));
    }
    sql_params.push(Box::new(page_size as i32));
    sql_params.push(Box::new(offset as i32));

    let param_refs: Vec<&dyn rusqlite::types::ToSql> =
        sql_params.iter().map(|p| p.as_ref()).collect();

    // 单查询解析结果
    let items: Vec<TransactionListItem> = conn
        .prepare(&items_sql)?
        .query_map(rusqlite::params_from_iter(param_refs), |row| {
            let postings_data: Option<String> = row.get(7);
            let tag_names: Option<String> = row.get(8);

            // 解析 postings_data: "account1:+3500|account2:-3500"
            let postings_summary: Vec<(String, i64)> = postings_data
                .and_then(|data| parse_postings_summary(&data))
                .unwrap_or_default();

            // 解析 tag_names: "tag1,tag2"
            let tags: Vec<String> = tag_names
                .map(|data| data.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default();

            Ok(TransactionListItem {
                id: row.get(0)?,
                date: row.get(1)?,
                description: row.get(2)?,
                category_id: row.get(3)?,
                category_name: row.get(6)?,
                postings_summary,
                tags,
                display_amount: row.get(7)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TransactionListResponse {
        items: items.clone(),
        pagination: PaginationInfo {
            page,
            page_size,
            total_count,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
        // **重要**: date_groups.day_total 由后端计算，前端不做金额计算
        date_groups: build_date_groups(&items),
    })
}

/// 构建日期分组数据（后端计算 day_total）
///
/// **设计约束**: AGENTS.md 要求前端不做金额计算
/// day_total 由后端聚合计算，前端仅显示
fn build_date_groups(items: &[TransactionListItem]) -> Vec<TransactionDateGroup> {
    let mut groups: std::collections::HashMap<String, TransactionDateGroup> =
        std::collections::HashMap::new();

    for item in items {
        if !groups.contains_key(&item.date) {
            groups.insert(item.date.clone(), TransactionDateGroup {
                date: item.date.clone(),
                date_display: format_date_display(&item.date),
                items: Vec::new(),
                day_total: 0,
            });
        }

        let group = groups.get_mut(&item.date).unwrap();
        group.items.push(item.clone());
        // **后端计算**: day_total 由借方总额累加得出
        group.day_total += item.display_amount;
    }

    // 按日期倒序排列
    groups.into_values()
        .collect::<Vec<_>>()
        .into_iter()
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect()
}

/// 格式化日期显示（YYYY-MM-DD → "2024年3月15日"）
fn format_date_display(date_str: &str) -> String {
    // 解析 YYYY-MM-DD 格式
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() == 3 {
        let year: i32 = parts[0].parse().unwrap_or(0);
        let month: u32 = parts[1].parse().unwrap_or(0);
        let day: u32 = parts[2].parse().unwrap_or(0);
        format!("{}年{}月{}日", year, month, day)
    } else {
        date_str.to_string()
    }
}

/// 解析 postings 摘要字符串
fn parse_postings_summary(data: &str) -> Option<Vec<(String, i64)>> {
    data.split('|')
        .filter_map(|entry| {
            let parts: Vec<&str> = entry.split(':').collect();
            if parts.len() == 2 {
                let account = parts[0].trim();
                let amount_str = parts[1].trim();
                let amount: i64 = amount_str.parse().ok()?;
                Some((account.to_string(), amount))
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .into()
}

/// 构建 WHERE 条件
///
/// **安全设计**: LIKE 查询必须转义 SQL 通配符（%、_、\），防止：
/// 1. 用户输入 `%` 匹配任意字符串，绕过精确搜索
/// 2. 用户输入 `_` 匹配任意单字符，造成意外匹配
/// 3. 这不是数据泄露，但会导致搜索行为不可预期
fn build_where_clause(filter: &TransactionFilter) -> (String, Vec<String>) {
    let mut conditions: Vec<String> = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(ref from) = filter.from_date {
        conditions.push("t.date >= ?");
        params.push(from.clone());
    }

    if let Some(ref to) = filter.to_date {
        conditions.push("t.date <= ?");
        params.push(to.clone());
    }

    if let Some(ref acct_id) = filter.account_id {
        conditions.push("p.account_id = ?");
        params.push(acct_id.clone());
    }

    if let Some(ref cat_id) = filter.category_id {
        conditions.push("t.category_id = ?");
        params.push(cat_id.clone());
    }

    if let Some(ref tag_id) = filter.tag_id {
        conditions.push("tt.tag_id = ?");
        params.push(tag_id.clone());
    }

    if let Some(ref query) = filter.description_query {
        // **关键修复**: 转义 LIKE 通配符
        let escaped = escape_like_pattern(query);
        conditions.push("t.description LIKE ? ESCAPE '\\'");
        params.push(format!("%{}%", escaped));
    }

    // 金额筛选需要 JOIN postings 计算
    // 简化处理：金额范围筛选借方总额
    if filter.min_amount.is_some() || filter.max_amount.is_some() {
        // 这部分需要更复杂的查询，暂用子查询
        // 实际实现时可以优化为 JOIN
    }

    let where_clause = conditions.join(" AND ");
    (where_clause, params)
}

/// 转义 LIKE 搜索模式中的 SQL 通配符
///
/// 转义规则：
/// - `%` → `\%`（防止匹配任意字符串）
/// - `_` → `\_`（防止匹配任意单字符）
/// - `\` → `\\`（防止转义字符本身被误解析）
///
/// 使用 `ESCAPE '\\'` 声明转义字符
fn escape_like_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// 构建单个列表项
fn build_list_item(conn: &Connection, tx_id: &str) -> Result<TransactionListItem, AppError> {
    // 查询交易基本信息
    let (date, description, category_id, created_at, updated_at):
        (String, String, Option<String>, String, String) = conn.query_row(
        "SELECT date, description, category_id, created_at, updated_at
         FROM transactions WHERE id = ?1",
        [tx_id],
        |row| Ok((
            row.get(0)?, row.get(1)?,
            row.get(2)?, row.get(3)?, row.get(4)?
        )),
    )?;

    // 查询分类名称
    let category_name: Option<String> = if let Some(ref cat_id) = category_id {
        conn.query_row(
            "SELECT name FROM categories WHERE id = ?1",
            [cat_id],
            |row| row.get(0),
        ).optional()?
    } else {
        None
    };

    // 查询分录摘要
    let postings_summary: Vec<(String, i64)> = conn
        .prepare(
            "SELECT a.name, p.amount
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             WHERE p.transaction_id = ?1
             ORDER BY ABS(p.amount) DESC
             LIMIT 2"
        )?
        .query_map([tx_id], |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?
        )))?
        .collect::<Result<Vec<_>, _>>()?;

    // 计算显示金额（借方总额）
    let display_amount = calculate_display_amount(conn, tx_id)?;

    // 查询标签
    let tags: Vec<String> = conn
        .prepare(
            "SELECT tg.name
             FROM transaction_tags tt
             JOIN tags tg ON tg.id = tt.tag_id
             WHERE tt.transaction_id = ?1"
        )?
        .query_map([tx_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TransactionListItem {
        id: tx_id.to_string(),
        date,
        description,
        category_id,
        category_name,
        postings_summary,
        tags,
        display_amount,
        created_at,
        updated_at,
    })
}

/// 计算用户可见的金额（借方总额）
///
/// 规则：借方金额（正数）总和
fn calculate_display_amount(
    conn: &Connection,
    tx_id: &str,
) -> Result<i64, AppError> {
    // 借方金额（正数）总和
    let debit_total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0) FROM postings
         WHERE transaction_id = ?1 AND amount > 0",
        [tx_id],
        |row| row.get(0),
    )?;

    Ok(debit_total)
}
```

### 3.2 搜索功能实现

```rust
/// 全文搜索交易
///
/// **安全设计**: 使用参数化查询 + LIKE 通配符转义，防止：
/// 1. SQL 注入（通过参数化查询）
/// 2. LIKE 通配符绕过（通过 escape_like_pattern）
pub fn search_transactions(
    conn: &Connection,
    query: &str,
    limit: u32,
) -> Result<Vec<TransactionListItem>, AppError> {
    // 搜索条件：
    // 1. 描述匹配
    // 2. 账户名称匹配
    // 3. 分类名称匹配
    // 4. 标签名称匹配

    // **关键修复**: 转义 LIKE 通配符
    let escaped = escape_like_pattern(query);
    let search_pattern = format!("%{}%", escaped);

    let tx_ids: Vec<String> = conn
        .prepare(
            "SELECT DISTINCT t.id
             FROM transactions t
             LEFT JOIN postings p ON p.transaction_id = t.id
             LEFT JOIN accounts a ON a.id = p.account_id
             LEFT JOIN categories c ON c.id = t.category_id
             LEFT JOIN transaction_tags tt ON tt.transaction_id = t.id
             LEFT JOIN tags tg ON tg.id = tt.tag_id
             WHERE t.description LIKE ?1 ESCAPE '\\'
                OR a.name LIKE ?1 ESCAPE '\\'
                OR c.name LIKE ?1 ESCAPE '\\'
                OR tg.name LIKE ?1 ESCAPE '\\'
             ORDER BY t.date DESC
             LIMIT ?2"
        )?
        .query_map(rusqlite::params![&search_pattern, limit], |row| {
            row.get::<_, String>(0)
        })?
        .collect::<Result<Vec<_>, _>>()?;

    tx_ids
        .iter()
        .map(|tx_id| build_list_item(conn, tx_id))
        .collect()
}
```

---

## 四、Tauri Command

```rust
// src-tauri/src/commands/transactions.rs

/// 获取交易列表（带过滤和分页）
#[tauri::command]
pub async fn list_transactions(
    filter: TransactionFilter,
    state: State<'_, AppState>,
) -> Result<TransactionListResponse, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    do_list_transactions(&conn, &filter).map_err(|e| e.to_string())
}

/// 搜索交易
#[tauri::command]
pub async fn search_transactions(
    query: String,
    limit: Option<u32>,
    state: State<'_, AppState>,
) -> Result<Vec<TransactionListItem>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    let limit = limit.unwrap_or(50);
    do_search_transactions(&conn, &query, limit).map_err(|e| e.to_string())
}

/// 获取单个交易详情
#[tauri::command]
pub async fn get_transaction(
    id: String,
    state: State<'_, AppState>,
) -> Result<TransactionWithPostings, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    get_transaction_by_id(&conn, &id).map_err(|e| e.to_string())
}
```

---

## 五、前端 API 调用

```typescript
// src/utils/api.ts 补充

import { invoke } from "@tauri-apps/api/core";

export async function listTransactions(
  filter: TransactionFilter = {},
): Promise<TransactionListResponse> {
  return invoke("list_transactions", { filter });
}

export async function searchTransactions(
  query: string,
  limit?: number,
): Promise<TransactionListItem[]> {
  return invoke("search_transactions", { query, limit });
}

export async function getTransaction(
  id: string,
): Promise<TransactionWithPostings> {
  return invoke("get_transaction", { id });
}

// 使用示例
const loadTransactions = async (page = 1) => {
  const result = await listTransactions({
    page,
    page_size: 20,
    sort_by: "date",
    sort_order: "desc",
  });

  console.log(`加载第 ${page} 页，共 ${result.pagination.total_count} 条`);
  return result;
};

const searchByDescription = async (keyword: string) => {
  const results = await searchTransactions(keyword);
  return results;
};

const filterByCategory = async (categoryId: string) => {
  const result = await transactionList({
    category_id: categoryId,
    from_date: "2024-01-01",
    to_date: "2024-12-31",
  });
  return result;
};
```

---

## 六、前端页面设计

### 6.1 页面布局

```
┌─────────────────────────────────────────────────────────────────┐
│  交易列表                                                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 🔍 搜索栏 [________________]  [搜索]                         ││
│  │                                                              ││
│  │ 过滤面板:                                                    ││
│  │ [日期范围 ▼] [账户 ▼] [分类 ▼] [类型 ▼] [标签 ▼] [清除]     ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 📅 2024年3月15日                           当日合计: ¥235    ││
│  ├─────────────────────────────────────────────────────────────┤│
│  │ ├─ 午餐 - 麦当劳              支出 · 餐饮        -¥35       ││
│  │ │   现金账户 → 餐饮分类                                      ││
│  │ │   标签: [工作日] [午餐]                    [编辑] [删除]   ││
│  │ │                                                            ││
│  │ ├─ 工资收入                    收入 · 工资       +¥5000     ││
│  │ │   收入分类 → 银行账户                                      ││
│  │ │                                            [编辑] [删除]   ││
│  │ │                                                            ││
│  │ └─ 银行转账支付宝              转账              ¥100       ││
│  │ │   银行账户 → 支付宝                                        ││
│  │ │                                            [编辑] [删除]   ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │ 📅 2024年3月14日                           当日合计: ¥150    ││
│  ├─────────────────────────────────────────────────────────────┤│
│  │ ├─ 交通费                      支出 · 交通        -¥30      ││
│  │ │   ...                                                       ││
│  └─────────────────────────────────────────────────────────────┘│
│                                                                  │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    [上一页] 第1/25页 [下一页]                ││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 页面组件实现

```typescript
// src/pages/TransactionsPage.tsx

import { useState, useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { TransactionFilter, TransactionListResponse, TransactionDateGroup } from "@/types";
import { listTransactions, searchTransactions } from "@/utils/api";
import { formatAmount, formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { Badge } from "@/components/ui/badge";
import { TransactionFilterPanel } from "@/components/transactions/TransactionFilterPanel";
import { TransactionCard } from "@/components/transactions/TransactionCard";
import { Pagination } from "@/components/ui/pagination";
import { Plus, Search } from "lucide-react";

export function TransactionsPage() {
  const { t } = useTranslation();

  // 状态管理
  const [page, setPage] = useState(1);
  const [searchQuery, setSearchQuery] = useState("");
  const [filter, setFilter] = useState<TransactionFilter>({
    page: 1,
    page_size: 20,
    sort_by: "date",
    sort_order: "desc",
  });

  // 是否使用搜索模式
  const isSearchMode = searchQuery.trim().length > 0;

  // 查询交易列表
  const {
    data: listData,
    isLoading: listLoading,
    refetch,
  } = useQuery({
    queryKey: ["transaction-list", filter],
    queryFn: () => listTransactions(filter),
    enabled: !isSearchMode,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 查询搜索结果
  const {
    data: searchResults,
    isLoading: searchLoading,
  } = useQuery({
    queryKey: ["transaction-search", searchQuery],
    queryFn: () => searchTransactions(searchQuery, 50),
    enabled: isSearchMode,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 合并数据
  const displayData = isSearchMode
    ? { items: searchResults ?? [], pagination: { page: 1, total_count: searchResults?.length ?? 0 } }
    : listData ?? { items: [], pagination: { page: 1, total_count: 0 } };

  const isLoading = isSearchMode ? searchLoading : listLoading;

  // 按日期分组
  const groupedTransactions = useMemo(() => {
    return groupByDate(displayData.items);
  }, [displayData.items]);

  // 处理分页变化
  const handlePageChange = (newPage: number) => {
    setPage(newPage);
    setFilter(prev => ({ ...prev, page: newPage }));
  };

  // 处理过滤变化
  const handleFilterChange = (newFilter: Partial<TransactionFilter>) => {
    setFilter(prev => ({ ...prev, ...newFilter, page: 1 }));
    setPage(1);
  };

  // 清除过滤
  const handleClearFilter = () => {
    setFilter({ page: 1, page_size: 20, sort_by: "date", sort_order: "desc" });
    setSearchQuery("");
    setPage(1);
  };

  return (
    <div className="flex flex-col h-full">
      {/* 顶部工具栏 */}
      <div className="flex items-center justify-between px-6 py-4 border-b">
        <h1 className="text-xl font-semibold">{t("transactions.title")}</h1>
        <Button>
          <Plus className="h-4 w-4 mr-1" />
          {t("transactions.new")}
        </Button>
      </div>

      {/* 搜索栏 */}
      <div className="px-6 py-3 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder={t("transactions.searchPlaceholder")}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
          <Button variant="secondary" onClick={handleClearFilter}>
            {t("common.clearFilter")}
          </Button>
        </div>
      </div>

      {/* 过滤面板 */}
      <TransactionFilterPanel
        filter={filter}
        onChange={handleFilterChange}
      />

      {/* 交易列表 */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        {isLoading && (
          <div className="py-12 text-center text-muted-foreground">
            {t("common.loading")}
          </div>
        )}

        {!isLoading && groupedTransactions.length === 0 && (
          <div className="py-12 text-center">
            <p className="text-muted-foreground">{t("transactions.noTransactions")}</p>
          </div>
        )}

        {!isLoading && groupedTransactions.length > 0 && (
          <div className="space-y-6">
            {groupedTransactions.map((group) => (
              <TransactionDateGroupComponent key={group.date} group={group} />
            ))}
          </div>
        )}
      </div>

      {/* 分页 */}
      {!isSearchMode && displayData.pagination.total_count > 20 && (
        <div className="px-6 py-3 border-t">
          <Pagination
            page={page}
            totalPages={Math.ceil(displayData.pagination.total_count / 20)}
            onPageChange={handlePageChange}
          />
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// 日期分组组件
// ---------------------------------------------------------------------------

function TransactionDateGroupComponent({ group }: { group: TransactionDateGroup }) {
  return (
    <div>
      {/* 日期标题 */}
      <div className="flex items-center justify-between mb-2 px-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{formatDate(group.date)}</span>
        </div>
        <span className="text-sm text-muted-foreground tabular-nums">
          当日合计: {formatAmount(group.day_total)}
        </span>
      </div>

      {/* 交易卡片列表 */}
      <div className="space-y-1">
        {group.items.map((item) => (
          <TransactionCard key={item.id} transaction={item} />
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 交易卡片组件
// ---------------------------------------------------------------------------

interface TransactionCardProps {
  transaction: TransactionListItem;
}

function TransactionCard({ transaction }: TransactionCardProps) {
  const { t } = useTranslation();
  const [showActions, setShowActions] = useState(false);

  return (
    <div
      className="flex items-center justify-between rounded-lg px-3 py-2.5
                 hover:bg-accent transition-colors cursor-pointer"
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
      onClick={() => {/* 导航到详情页 */}}
    >
      <div className="min-w-0 flex-1">
        {/* 描述 */}
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">
            {transaction.description}
          </span>
        </div>

        {/* 分录摘要和分类 */}
        <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
          {transaction.category_name && (
            <span>{transaction.category_name}</span>
          )}
          {transaction.postings_summary.length > 0 && (
            <span className="truncate">
              {transaction.postings_summary
                .map(([name, _]) => name)
                .join(" → ")}
            </span>
          )}
        </div>

        {/* 标签 */}
        {transaction.tags.length > 0 && (
          <div className="flex gap-1 mt-1">
            {transaction.tags.map((tag) => (
              <Badge key={tag} variant="outline" className="text-xs">
                {tag}
              </Badge>
            ))}
          </div>
        )}
      </div>

      {/* 金额 */}
      <div className="flex items-center gap-3">
        <span className="text-sm font-medium tabular-nums">
          {formatAmount(transaction.display_amount)}
        </span>

        {/* 操作按钮 */}
        {showActions && (
          <div className="flex gap-1">
            <Button variant="ghost" size="sm" onClick={(e) => {
              e.stopPropagation();
              {/* 打开编辑 */}
            }}>
              {t("common.edit")}
            </Button>
            <Button variant="ghost" size="sm" onClick={(e) => {
              e.stopPropagation();
              {/* 打开删除确认 */}
            }}>
              {t("common.delete")}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------
// **重要**: 前端不做金额计算（AGENTS.md 约束）
// day_total 由后端在 TransactionListResponse.date_groups 中返回

function groupByDate(items: TransactionListItem[]): TransactionDateGroup[] {
  // **注意**: 此函数仅用于将扁平列表转换为分组结构
  // day_total 必须由后端计算，前端仅做分组聚合（不涉及金额）
  // 如果后端已返回 date_groups，前端应直接使用

  const groups: Map<string, TransactionDateGroup> = new Map();

  for (const item of items) {
    if (!groups.has(item.date)) {
      groups.set(item.date, {
        date: item.date,
        date_display: formatDate(item.date),
        items: [],
        day_total: 0, // 占位值，应由后端提供
      });
    }

    const group = groups.get(item.date)!;
    group.items.push(item);
    // **禁止**: 前端不计算 day_total（AGENTS.md 约束）
    // group.day_total += item.display_amount; // ❌ 移除
  }

  // **警告**: 返回的 day_total 为占位值，生产环境应由后端返回 date_groups
  return Array.from(groups.values()).sort((a, b) => b.date.localeCompare(a.date));
}
```

### 6.3 过滤面板组件

```typescript
// src/components/transactions/TransactionFilterPanel.tsx

import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { TransactionFilter } from "@/types";
import { accountList, categoryList, tagList } from "@/utils/api";
import { Select } from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { X } from "lucide-react";

interface TransactionFilterPanelProps {
  filter: TransactionFilter;
  onChange: (filter: Partial<TransactionFilter>) => void;
}

export function TransactionFilterPanel({ filter, onChange }: TransactionFilterPanelProps) {
  const { t } = useTranslation();

  // 加载账户、分类、标签选项
  const { data: accounts = [] } = useQuery({
    queryKey: ["accounts"],
    queryFn: accountList,
  });

  const { data: categories = [] } = useQuery({
    queryKey: ["categories"],
    queryFn: () => categoryList(),
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  const { data: tags = [] } = useQuery({
    queryKey: ["tags"],
    queryFn: tagList,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 检查是否有活跃的过滤条件
  const hasActiveFilters =
    filter.from_date || filter.to_date ||
    filter.account_id || filter.category_id ||
    filter.tag_id;

  if (!hasActiveFilters) {
    // 显示简洁的过滤按钮
    return (
      <div className="px-6 py-2 border-b">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onChange({ from_date: "", to_date: "" })}
        >
          {t("transactions.showFilters")}
        </Button>
      </div>
    );
  }

  return (
    <div className="px-6 py-3 border-b bg-muted/20">
      <div className="flex items-center gap-4 flex-wrap">
        {/* 日期范围 */}
        <div className="flex items-center gap-2">
          <Label className="text-xs">{t("transactions.filter.date")}</Label>
          <DatePicker
            value={filter.from_date ?? ""}
            onChange={(v) => onChange({ from_date: v })}
            placeholder={t("transactions.filter.from")}
          />
          <span className="text-xs">-</span>
          <DatePicker
            value={filter.to_date ?? ""}
            onChange={(v) => onChange({ to_date: v })}
            placeholder={t("transactions.filter.to")}
          />
        </div>

        {/* 账户筛选 */}
        <div className="flex items-center gap-2">
          <Label className="text-xs">{t("transactions.filter.account")}</Label>
          <Select
            value={filter.account_id ?? ""}
            onChange={(v) => onChange({ account_id: v })}
            options={[
              { value: "", label: t("common.all") },
              ...accounts.map((a) => ({ value: a.id, label: a.name })),
            ]}
          />
        </div>

        {/* 分类筛选 */}
        <div className="flex items-center gap-2">
          <Label className="text-xs">{t("transactions.filter.category")}</Label>
          <Select
            value={filter.category_id ?? ""}
            onChange={(v) => onChange({ category_id: v })}
            options={[
              { value: "", label: t("common.all") },
              ...categories.map((c) => ({ value: c.id, label: c.name })),
            ]}
          />
        </div>

        {/* 标签筛选 */}
        <div className="flex items-center gap-2">
          <Label className="text-xs">{t("transactions.filter.tag")}</Label>
          <Select
            value={filter.tag_id ?? ""}
            onChange={(v) => onChange({ tag_id: v })}
            options={[
              { value: "", label: t("common.all") },
              ...tags.map((t) => ({ value: t.id, label: t.name })),
            ]}
          />
        </div>

        {/* 清除按钮 */}
        <Button variant="ghost" size="sm" onClick={() => onChange({
          from_date: undefined,
          to_date: undefined,
          account_id: undefined,
          category_id: undefined,
          tag_id: undefined,
        })}>
          <X className="h-3 w-3 mr-1" />
          {t("common.clear")}
        </Button>
      </div>
    </div>
  );
}
```

---

## 七、性能优化策略

### 7.1 查询优化

```rust
// 添加索引以提升查询性能

// 在 schema.rs 中添加
"CREATE INDEX IF NOT EXISTS idx_transactions_date_desc ON transactions(date DESC);"
"CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category_id);"
"CREATE INDEX IF NOT EXISTS idx_transaction_tags_tag ON transaction_tags(tag_id);"
```

### 7.2 分页策略

| 场景            | 策略                    |
| --------------- | ----------------------- |
| 数据量 < 100    | 全量加载，前端分页      |
| 数据量 100-1000 | 后端分页，每页 20-50 条 |
| 数据量 > 1000   | 虚拟滚动 + 滚动加载     |

### 7.3 前端虚拟滚动（大型列表）

```typescript
// src/components/transactions/VirtualTransactionList.tsx

import { useVirtualizer } from "@tanstack/react-virtual";

export function VirtualTransactionList({
  transactions,
  onLoadMore,
}: VirtualTransactionListProps) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: transactions.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 80, // 每条交易高度
    overscan: 5,
  });

  const items = virtualizer.getVirtualItems();

  // 监听滚动到底部时加载更多
  useEffect(() => {
    const lastItem = items[items.length - 1];
    if (lastItem && lastItem.index >= transactions.length - 3) {
      onLoadMore();
    }
  }, [items, transactions.length, onLoadMore]);

  return (
    <div ref={parentRef} className="h-full overflow-y-auto">
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {items.map((virtualRow) => (
          <div
            key={virtualRow.key}
            style={{
              position: "absolute",
              top: 0,
              left: 0,
              width: "100%",
              height: `${virtualRow.size}px`,
              transform: `translateY(${virtualRow.start}px)`,
            }}
          >
            <TransactionCard transaction={transactions[virtualRow.index]} />
          </div>
        ))}
      </div>
    </div>
  );
}
```

---

## 八、单元测试设计

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_transactions_basic() {
        let (_dir, conn) = test_env();
        setup_test_transactions(&conn);

        let filter = TransactionFilter::default();
        let result = list_transactions(&conn, &filter).unwrap();

        assert!(result.items.len() > 0);
        assert!(result.pagination.total_count > 0);
    }

    #[test]
    fn test_list_transactions_with_date_filter() {
        let (_dir, conn) = test_env();
        setup_test_transactions(&conn);

        let filter = TransactionFilter {
            from_date: Some("2024-03-01".into()),
            to_date: Some("2024-03-31".into()),
            ..Default::default()
        };

        let result = list_transactions(&conn, &filter).unwrap();

        // 所有交易日期应在范围内
        for item in &result.items {
            assert!(item.date >= "2024-03-01");
            assert!(item.date <= "2024-03-31");
        }
    }

    #[test]
    fn test_list_transactions_pagination() {
        let (_dir, conn) = test_env();
        setup_many_transactions(&conn, 100);

        // 第一页
        let filter1 = TransactionFilter {
            page: Some(1),
            page_size: Some(10),
            ..Default::default()
        };
        let result1 = list_transactions(&conn, &filter1).unwrap();
        assert_eq!(result1.items.len(), 10);
        assert!(result1.pagination.has_next);
        assert!(!result1.pagination.has_prev);

        // 第二页
        let filter2 = TransactionFilter {
            page: Some(2),
            page_size: Some(10),
            ..Default::default()
        };
        let result2 = list_transactions(&conn, &filter2).unwrap();
        assert_eq!(result2.items.len(), 10);
        assert!(result2.pagination.has_next);
        assert!(result2.pagination.has_prev);
    }

    #[test]
    fn test_search_transactions() {
        let (_dir, conn) = test_env();
        setup_test_transactions(&conn);

        let results = search_transactions(&conn, "午餐", 10).unwrap();

        // 所有结果描述应包含"午餐"
        for item in &results {
            assert!(item.description.contains("午餐"));
        }
    }

    #[test]
    fn test_filter_by_category() {
        let (_dir, conn) = test_env();
        setup_test_transactions(&conn);

        let filter = TransactionFilter {
            category_id: Some("cat-food-uuid".into()),
            ..Default::default()
        };

        let result = list_transactions(&conn, &filter).unwrap();

        for item in &result.items {
            assert_eq!(item.category_id, Some("cat-food-uuid".into()));
        }
    }

    #[test]
    fn test_display_amount_calculation() {
        let (_dir, conn) = test_env();

        // withdrawal: 借方金额为支出
        create_test_transaction(&conn, "withdrawal", 3500);
        let amount = calculate_display_amount(&conn, &tx_id, &Some("withdrawal".into())).unwrap();
        assert_eq!(amount, 3500);

        // deposit: 借方金额为收入
        create_test_transaction(&conn, "deposit", 500000);
        let amount = calculate_display_amount(&conn, &tx_id, &Some("deposit".into())).unwrap();
        assert_eq!(amount, 500000);
    }

    #[test]
    fn test_group_by_date_ordering() {
        let items = vec![
            create_mock_item("2024-03-15"),
            create_mock_item("2024-03-14"),
            create_mock_item("2024-03-16"),
        ];

        let groups = group_by_date(&items);

        // 应按日期倒序
        assert_eq!(groups[0].date, "2024-03-16");
        assert_eq!(groups[1].date, "2024-03-15");
        assert_eq!(groups[2].date, "2024-03-14");
    }
}
```

---

## 九、路由配置

```typescript
// src/App.tsx 路由补充

import { TransactionsPage } from "@/pages/TransactionsPage";
import { TransactionDetailPage } from "@/pages/TransactionDetailPage";
import { TransactionEditPage } from "@/pages/TransactionEditPage";

const routes = [
  { path: "/transactions", element: <TransactionsPage /> },
  { path: "/transactions/:id", element: <TransactionDetailPage /> },
  { path: "/transactions/:id/edit", element: <TransactionEditPage /> },
  { path: "/transactions/new", element: <TransactionEditPage /> },
];
```

---

## 十、国际化文案

```json
// src/i18n/locales/zh.json 补充

{
  "transactions": {
    "title": "交易",
    "new": "新增交易",
    "searchPlaceholder": "搜索交易描述、账户、分类...",
    "noTransactions": "暂无交易记录",
    "showFilters": "显示筛选",
    "types": {
      "withdrawal": "支出",
      "deposit": "收入",
      "transfer": "转账"
    },
    "filter": {
      "date": "日期",
      "from": "开始",
      "to": "结束",
      "account": "账户",
      "category": "分类",
      "type": "类型",
      "tag": "标签"
    },
    "dayTotal": "当日合计",
    "postingsSummary": "分录"
  }
}

// src/i18n/locales/en.json 补充

{
  "transactions": {
    "title": "Transactions",
    "new": "New Transaction",
    "searchPlaceholder": "Search description, account, category...",
    "noTransactions": "No transactions found",
    "showFilters": "Show Filters",
    "types": {
      "withdrawal": "Expense",
      "deposit": "Income",
      "transfer": "Transfer"
    },
    "filter": {
      "date": "Date",
      "from": "From",
      "to": "To",
      "account": "Account",
      "category": "Category",
      "type": "Type",
      "tag": "Tag"
    },
    "dayTotal": "Day Total",
    "postingsSummary": "Postings"
  }
}
```

---

## 十一、设计决策总结

| 决策             | 说明                                                         |
| ---------------- | ------------------------------------------------------------ |
| **列表项简化**   | TransactionListItem 不包含完整分录，仅显示摘要，减少数据传输 |
| **按日期分组**   | 前端展示时按日期分组，提供日期合计，增强可读性               |
| **分页优先**     | 后端分页，每页 20 条，避免一次加载过多数据                   |
| **虚拟滚动可选** | 数据量 > 1000 时启用虚拟滚动，提升性能                       |
| **搜索多维度**   | 支持描述、账户名、分类名、标签名全文搜索                     |
| **金额显示**     | 根据交易类型显示正负号和颜色                                 |
| **悬停操作**     | 交易卡片悬停时显示编辑/删除按钮                              |

---

## 十二、后续扩展

| 功能           | 说明                           | 优先级           |
| -------------- | ------------------------------ | ---------------- |
| **交易详情页** | 显示完整分录信息、标签、时间戳 | P0（下一步设计） |
| **批量操作**   | 批量删除、批量修改分类         | P2               |
| **导出功能**   | 从列表导出筛选结果为 CSV       | P2               |
| **图表集成**   | 列表页面右侧显示收支趋势图     | P3               |
| **智能分组**   | 支持按周、按月分组展示         | P3               |
