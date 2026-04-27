# 18 - 交易详情与编辑设计

> 返回 [DESIGN.md](../DESIGN.md) > 返回 [交易模块总览](14-transaction-module.md)

---

## 一、功能需求

根据 `04-requirements.md`：

| ID        | 功能     | 描述                       | 优先级 |
| --------- | -------- | -------------------------- | ------ |
| **TXN-2** | 编辑交易 | 修改已有交易的任意字段     | P0     |
| **TXN-3** | 删除交易 | 删除交易（需确认）         | P0     |
| 详情页    | 交易详情 | 查看完整分录、标签、时间戳 | P1     |

---

## 二、交易详情页设计

### 2.1 数据结构

```rust
// src-tauri/src/models/transaction.rs

/// 完整交易详情（含完整分录信息）
#[derive(Debug, Serialize)]
pub struct TransactionDetail {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,

    /// 完整分录列表（含账户详细信息）
    pub postings: Vec<PostingDetail>,

    /// 标签列表（完整信息）
    pub tags: Vec<TagDetail>,

    /// 元信息
    pub created_at: String,
    pub updated_at: String,

    /// 计算字段
    pub is_balanced: bool,        // 分录是否平衡
    pub posting_count: u32,       // 分录数量
    pub debit_total: i64,         // 借方总额
    pub credit_total: i64,        // 贷方总额
}

> **注**: 本系统不存储 transaction_type，详情对象也无此字段。

/// 分录详情（含账户信息）
#[derive(Debug, Serialize)]
pub struct PostingDetail {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub amount: i64,
    pub amount_display: String,   // 格式化显示，如 "+¥35.00"
    pub is_debit: bool,           // 是否借方（金额 > 0）
    pub created_at: String,
}

/// 标签详情
#[derive(Debug, Serialize)]
pub struct TagDetail {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}
```

### 2.2 TypeScript 类型

```typescript
// src/types/index.ts 补充

export interface TransactionDetail {
  id: string;
  date: string;
  description: string;
  category_id: string | null;
  category_name: string | null;
  postings: PostingDetail[];
  tags: TagDetail[];
  created_at: string;
  updated_at: string;
  is_balanced: boolean;
  posting_count: number;
  debit_total: number;
  credit_total: number;
}

// 注: 本系统不存储 transaction_type，详情对象也无此字段

export interface PostingDetail {
  id: string;
  transaction_id: string;
  account_id: string;
  account_name: string;
  account_type: string;
  amount: number;
  amount_display: string;
  is_debit: boolean;
  created_at: string;
}

export interface TagDetail {
  id: string;
  name: string;
  color: string | null;
}
```

### 2.3 后端查询实现

```rust
// src-tauri/src/db/transactions.rs

/// 获取交易详情
pub fn do_get_transaction_detail(
    conn: &Connection,
    tx_id: &str,
) -> Result<TransactionDetail, AppError> {
    // 查询交易基本信息
    let (date, description, category_id, created_at, updated_at) = conn
        .query_row(
            "SELECT date, description, category_id, created_at, updated_at
             FROM transactions WHERE id = ?1",
            [tx_id],
            |row| Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            )),
        )
.map_err(|_| AppError::NotFound("errors.transaction.notFound".to_string()))?;

    // 查询分类名称
    let category_name = if let Some(ref cat_id) = category_id {
        conn.query_row(
            "SELECT name FROM categories WHERE id = ?1",
            [cat_id],
            |row| row.get::<_, String>(0),
        ).optional()?.flatten()
    } else {
        None
    };

    // 查询完整分录
    let postings: Vec<PostingDetail> = conn
        .prepare(
            "SELECT p.id, p.transaction_id, p.account_id, a.name, a.type, p.amount, p.created_at
             FROM postings p
             JOIN accounts a ON a.id = p.account_id
             WHERE p.transaction_id = ?1
             ORDER BY p.amount DESC"  // 借方在前
        )?
        .query_map([tx_id], |row| {
            let amount: i64 = row.get(5)?;
            let is_debit = amount > 0;
            let amount_display = format_amount_display(amount);

            Ok(PostingDetail {
                id: row.get(0)?,
                transaction_id: row.get(1)?,
                account_id: row.get(2)?,
                account_name: row.get(3)?,
                account_type: row.get(4)?,
                amount,
                amount_display,
                is_debit,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    // 查询标签
    let tags: Vec<TagDetail> = conn
        .prepare(
            "SELECT tg.id, tg.name, tg.color
             FROM transaction_tags tt
             JOIN tags tg ON tg.id = tt.tag_id
             WHERE tt.transaction_id = ?1"
        )?
        .query_map([tx_id], |row| Ok(TagDetail {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        }))?
        .collect::<Result<Vec<_>, _>>()?;

    // 计算聚合字段
    let debit_total: i64 = postings.iter()
        .filter(|p| p.is_debit)
        .map(|p| p.amount)
        .sum();
    let credit_total: i64 = postings.iter()
        .filter(|p| !p.is_debit)
        .map(|p| p.amount)
        .sum();
    let is_balanced = debit_total + credit_total == 0;
    let posting_count = postings.len() as u32;

    Ok(TransactionDetail {
        id: tx_id.to_string(),
        date,
        description,
        category_id,
        category_name,
        postings,
        tags,
        created_at,
        updated_at,
        is_balanced,
        posting_count,
        debit_total,
        credit_total,
    })
}

/// 格式化金额显示
fn format_amount_display(amount: i64) -> String {
    let abs_amount = amount.abs();
    let yuan = abs_amount / 100;
    let fen = abs_amount % 100;

    let prefix = if amount > 0 { "+¥" } else { "-¥" };

    if fen == 0 {
        format!("{}{}.00", prefix, yuan)
    } else {
        format!("{}{}.{:02}", prefix, yuan, fen)
    }
}
```

### 2.4 Tauri Command

```rust
// src-tauri/src/commands/transactions.rs

/// 获取交易详情
#[tauri::command]
pub async fn get_transaction_detail(
    id: String,
    state: State<'_, AppState>,
) -> Result<TransactionDetail, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    do_get_transaction_detail(&conn, &id).map_err(|e| e.to_string())
}
```

### 2.5 前端详情页

```typescript
// src/pages/TransactionDetailPage.tsx

import { useParams, useNavigate } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { TransactionDetail } from "@/types";
import { getTransactionDetail } from "@/utils/api";
import { formatAmount, formatDate } from "@/utils/format";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import {
  ArrowLeft, Edit, Trash2, CheckCircle2, AlertCircle,
  ArrowUpRight, ArrowDownRight
} from "lucide-react";

export function TransactionDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();

  const { data: transaction, isLoading, error } = useQuery({
    queryKey: ["transaction-detail", id],
    queryFn: () => getTransactionDetail(id!),
    enabled: !!id,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  if (isLoading) {
    return <div className="p-6">{t("common.loading")}</div>;
  }

  if (error || !transaction) {
    return (
      <div className="p-6 text-center">
        <p className="text-muted-foreground">{t("transactions.notFound")}</p>
        <Button variant="outline" onClick={() => navigate("/transactions")}>
          {t("common.back")}
        </Button>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* 顶部导航 */}
      <div className="flex items-center justify-between px-6 py-4 border-b">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => navigate("/transactions")}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h1 className="text-lg font-semibold">{t("transactions.detail.title")}</h1>
        </div>

        <div className="flex gap-2">
          <Button variant="outline" onClick={() => navigate(`/transactions/${id}/edit`)}>
            <Edit className="h-4 w-4 mr-1" />
            {t("common.edit")}
          </Button>
          <Button variant="destructive" onClick={() => {/* 打开删除确认 */}}>
            <Trash2 className="h-4 w-4 mr-1" />
            {t("common.delete")}
          </Button>
        </div>
      </div>

      {/* 主内容 */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto space-y-6">

          {/* 基本信息 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t("transactions.detail.basicInfo")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* 日期 */}
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">{t("transactions.detail.date")}</span>
                <span className="text-sm font-medium">{formatDate(transaction.date)}</span>
              </div>

              {/* 描述 */}
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">{t("transactions.detail.description")}</span>
                <span className="text-sm font-medium">{transaction.description}</span>
              </div>

              {/* 分类 */}
              {transaction.category_name && (
                <div className="flex items-center justify-between">
                  <span className="text-sm text-muted-foreground">{t("transactions.detail.category")}</span>
                  <Badge variant="secondary">{transaction.category_name}</Badge>
                </div>
              )}

              {/* 标签 */}
              {transaction.tags.length > 0 && (
                <div className="flex items-start justify-between">
                  <span className="text-sm text-muted-foreground">{t("transactions.detail.tags")}</span>
                  <div className="flex gap-1 flex-wrap">
                    {transaction.tags.map((tag) => (
                      <Badge
                        key={tag.id}
                        variant="outline"
                        style={tag.color ? { borderColor: tag.color } : undefined}
                      >
                        {tag.name}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}
            </CardContent>
          </Card>

          {/* 分录列表 */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-base">{t("transactions.detail.postings")}</CardTitle>
                <div className="flex items-center gap-2">
                  {transaction.is_balanced ? (
                    <Badge variant="success" className="gap-1">
                      <CheckCircle2 className="h-3 w-3" />
                      {t("transactions.detail.balanced")}
                    </Badge>
                  ) : (
                    <Badge variant="destructive" className="gap-1">
                      <AlertCircle className="h-3 w-3" />
                      {t("transactions.detail.unbalanced")}
                    </Badge>
                  )}
                  <span className="text-xs text-muted-foreground">
                    {transaction.posting_count} {t("transactions.detail.entries")}
                  </span>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              <div className="space-y-2">
                {transaction.postings.map((posting) => (
                  <PostingRow key={posting.id} posting={posting} />
                ))}
              </div>

              <Separator className="my-4" />

              {/* 合计 */}
              <div className="space-y-2">
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">{t("transactions.detail.debitTotal")}</span>
                  <span className="font-medium text-green-600 tabular-nums">
                    +{formatAmount(transaction.debit_total)}
                  </span>
                </div>
                <div className="flex items-center justify-between text-sm">
                  <span className="text-muted-foreground">{t("transactions.detail.creditTotal")}</span>
                  <span className="font-medium text-red-600 tabular-nums">
                    {formatAmount(transaction.credit_total)}
                  </span>
                </div>
                <Separator />
                <div className="flex items-center justify-between text-sm font-medium">
                  <span>{t("transactions.detail.balance")}</span>
                  <span className={`tabular-nums ${
                    transaction.debit_total + transaction.credit_total === 0
                      ? "text-green-600"
                      : "text-red-600"
                  }`}>
                    {formatAmount(transaction.debit_total + transaction.credit_total)}
                  </span>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* 元信息 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t("transactions.detail.metadata")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{t("transactions.detail.id")}</span>
                <span className="font-mono">{transaction.id}</span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{t("transactions.detail.createdAt")}</span>
                <span>{formatTimestamp(transaction.created_at)}</span>
              </div>
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground">{t("transactions.detail.updatedAt")}</span>
                <span>{formatTimestamp(transaction.updated_at)}</span>
              </div>
            </CardContent>
          </Card>

        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 分录行组件
// ---------------------------------------------------------------------------

function PostingRow({ posting }: { posting: PostingDetail }) {
  return (
    <div className="flex items-center justify-between py-2 px-3 rounded-md bg-muted/30">
      <div className="flex items-center gap-3">
        {/* 借贷标识 */}
        {posting.is_debit ? (
          <ArrowUpRight className="h-4 w-4 text-green-600" />
        ) : (
          <ArrowDownRight className="h-4 w-4 text-red-600" />
        )}

        {/* 账户信息 */}
        <div>
          <span className="text-sm font-medium">{posting.account_name}</span>
          <span className="text-xs text-muted-foreground ml-2">
            ({posting.account_type})
          </span>
        </div>
      </div>

      {/* 金额 */}
      <span className={`text-sm font-medium tabular-nums ${
        posting.is_debit ? "text-green-600" : "text-red-600"
      }`}>
        {posting.amount_display}
      </span>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

function formatTimestamp(ts: string): string {
  return new Date(ts).toLocaleString();
}

function getTypeConfig(txType: string | null) {
  switch (txType) {
    case "withdrawal":
      return { badgeVariant: "destructive" };
    case "deposit":
      return { badgeVariant: "success" };
    case "transfer":
      return { badgeVariant: "secondary" };
    default:
      return { badgeVariant: "outline" };
  }
}
```

---

## 三、交易编辑设计

### 3.1 编辑输入结构

```rust
// src-tauri/src/models/transaction.rs

/// 编辑交易输入（所有字段可选）
#[derive(Debug, Deserialize)]
pub struct UpdateTransactionInput {
    /// 交易日期
    pub date: Option<String>,

    /// 交易描述
    pub description: Option<String>,

    /// 分类ID
    pub category_id: Option<String>,

    /// 分录修改（可选）
    /// 如果提供，会完全替换现有分录
    pub postings: Option<Vec<PostingInput>>,

    /// 标签修改（可选）
    /// 如果提供，会完全替换现有标签
    pub tags: Option<Vec<String>>,
}

> **注**: 本系统不存储 transaction_type，编辑输入也无此字段。
```

### 3.2 编辑验证规则

| 规则                 | 说明                       |
| -------------------- | -------------------------- |
| **至少一个字段修改** | 所有字段为空时拒绝更新     |
| **分录平衡**         | 如果修改分录，必须借贷平衡 |
| **账户存在**         | 新分录的账户必须有效且激活 |

### 3.3 后端编辑实现

```rust
// src-tauri/src/db/transactions.rs

/// 更新交易
pub fn update_transaction(
    conn: &Connection,
    tx_id: &str,
    input: &UpdateTransactionInput,
) -> Result<TransactionDetail, AppError> {
    // 1. 检查交易存在
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
        [tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound("errors.transaction.notFound".to_string()));
    }

    // 2. 检查是否有修改
    if input.date.is_none()
        && input.description.is_none()
        && input.category_id.is_none()
        && input.postings.is_none()
        && input.tags.is_none() {
        return Err(AppError::ValidationError("errors.transaction.noChanges".to_string()));
    }

    // 3. 分录验证（如果修改）
    let parsed_postings: Option<Vec<(String, i64)>> = if let Some(ref postings) = input.postings {
        let parsed = validate_and_parse_postings(postings)?;

        // 验证借贷平衡
        let total = parsed.iter().map(|(_, amt)| amt).sum::<i64>();
        if total != 0 {
            return Err(AppError::ValidationError(
                "errors.transaction.unbalanced".to_string()
            ));
        }

        // 验证账户存在且激活
        for (account_id, _) in &parsed {
            validate_account_active(conn, account_id)?;
        }

        Some(parsed)
    } else {
        None
    };

    // 4. 执行更新（事务）
    let tx = conn.transaction()?;
    let now = chrono::Utc::now().to_rfc3339();

    // 更新交易基本信息
    if let Some(ref date) = input.date {
        tx.execute(
            "UPDATE transactions SET date = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![date, &now, tx_id],
        )?;
    }

    if let Some(ref desc) = input.description {
        tx.execute(
            "UPDATE transactions SET description = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![desc, &now, tx_id],
        )?;
    }

    if input.category_id.is_some() {
        tx.execute(
            "UPDATE transactions SET category_id = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![&input.category_id, &now, tx_id],
        )?;
    }

    // 更新分录（如果提供）
    if let Some(ref parsed) = parsed_postings {
        // 删除旧分录
        tx.execute("DELETE FROM postings WHERE transaction_id = ?1", [tx_id])?;

        // 插入新分录
        for (account_id, amount) in parsed {
            let posting_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO postings (id, transaction_id, account_id, amount, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![&posting_id, tx_id, &account_id, amount, &now],
            )?;
        }
    }

    // 更新标签（如果提供）
    if let Some(ref tag_ids) = input.tags {
        // 删除旧标签关联
        tx.execute("DELETE FROM transaction_tags WHERE transaction_id = ?1", [tx_id])?;

        // 插入新标签关联
        for tag_id in tag_ids {
            // 验证标签存在
            let tag_exists: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
                [tag_id],
                |row| row.get(0),
            )?;

            if tag_exists {
                tx.execute(
                    "INSERT INTO transaction_tags (transaction_id, tag_id)
                     VALUES (?1, ?2)",
                    rusqlite::params![tx_id, tag_id],
                )?;
            }
        }
    }

    // 更新时间戳（如果没有任何其他更新，至少更新这个）
    tx.execute(
        "UPDATE transactions SET updated_at = ?1 WHERE id = ?2",
        rusqlite::params![&now, tx_id],
    )?;

    tx.commit()?;

    // 6. 返回更新后的详情
    get_transaction_detail(conn, tx_id)
}

/// 验证账户存在且激活
///
/// **国际化设计**: 错误消息使用国际化键，前端负责翻译：
/// - `"errors.account.notFound"` - 账户不存在
/// - `"errors.account.inactive"` - 账户已关闭
fn validate_account_active(conn: &Connection, account_id: &str) -> Result<(), AppError> {
    let (exists, is_active): (bool, bool) = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = ?1),
               COALESCE((SELECT is_active FROM accounts WHERE id = ?1), 0)",
        [account_id],
        |row| Ok((row.get::<_, bool>(0)?, row.get::<_, i32>(1)? == 1)),
    )?;

    if !exists {
        return Err(AppError::NotFound("errors.account.notFound".to_string()));
    }

    if !is_active {
        return Err(AppError::ValidationError(
            "errors.account.inactive".to_string()
        ));
    }

    Ok(())
}
```

### 3.4 Tauri Command

```rust
/// 更新交易
#[tauri::command]
pub async fn update_transaction(
    id: String,
    input: UpdateTransactionInput,
    state: State<'_, AppState>,
) -> Result<TransactionDetail, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    do_update_transaction(&conn, &id, &input).map_err(|e| e.to_string())
}
```

### 3.5 前端编辑页

```typescript
// src/pages/TransactionEditPage.tsx

import { useParams, useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useState, useEffect } from "react";
import type { TransactionDetail, UpdateTransactionInput, PostingInput } from "@/types";
import { getTransactionDetail, updateTransaction, listAccounts, listCategories, listTags } from "@/utils/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { PostingEditor } from "@/components/transactions/PostingEditor";
import { TagPicker } from "@/components/tags/TagPicker";
import { toast } from "sonner";
import { ArrowLeft, Save, Plus, Trash2 } from "lucide-react";

export function TransactionEditPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const isNew = !id; // 新建模式

  // 加载现有交易（编辑模式）
  const { data: existingTx } = useQuery({
    queryKey: ["transaction-detail", id],
    queryFn: () => getTransactionDetail(id!),
    enabled: !!id,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 加载账户、分类、标签选项
  const { data: accounts = [] } = useQuery({
    queryKey: ["accounts"],
    queryFn: listAccounts,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  const { data: categories = [] } = useQuery({
    queryKey: ["categories"],
    queryFn: () => listCategories(),
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  const { data: tags = [] } = useQuery({
    queryKey: ["tags"],
    queryFn: listTags,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 表单状态
  const [date, setDate] = useState("");
  const [description, setDescription] = useState("");
  const [categoryId, setCategoryId] = useState<string | null>(null);
  const [postings, setPostings] = useState<PostingInput[]>([]);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);

  // 初始化表单（编辑模式）
  useEffect(() => {
    if (existingTx) {
      setDate(existingTx.date);
      setDescription(existingTx.description);
      setCategoryId(existingTx.category_id);
      setSelectedTags(existingTx.tags.map((t) => t.id));

      // 转换分录为输入格式（金额由后端格式化，前端接收字符串）
      setPostings(existingTx.postings.map((p) => ({
        account_id: p.account_id,
        amount: p.amount_display.replace(/[¥+-]/g, ""), // 移除格式化符号
      })));
    } else if (isNew) {
      // 新建默认值
      setDate(new Date().toISOString().split("T")[0]);
      setPostings([
        { account_id: "", amount: "" },
        { account_id: "", amount: "" },
      ]);
    }
  }, [existingTx, isNew]);

  // 更新 Mutation
  const updateMutation = useMutation({
    mutationFn: (input: UpdateTransactionInput) =>
      updateTransaction(id!, input),
    onSuccess: () => {
      toast.success(t("transactions.edit.success"));
      queryClient.invalidateQueries({ queryKey: ["transaction-detail", id] });
      queryClient.invalidateQueries({ queryKey: ["transaction-list"] });
      navigate(`/transactions/${id}`);
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  // 保存
  const handleSave = () => {
    // 验证
    if (!date) {
      toast.error(t("transactions.edit.dateRequired"));
      return;
    }
    if (!description.trim()) {
      toast.error(t("transactions.edit.descriptionRequired"));
      return;
    }
    if (postings.length < 2) {
      toast.error(t("transactions.edit.minPostings"));
      return;
    }

    // 借贷平衡验证由后端执行，前端不做金额计算（AGENTS.md 约束）

    // 构造输入
    const input: UpdateTransactionInput = {
      date,
      description,
      category_id: categoryId,
      postings: postings.map((p) => ({
        account_id: p.account_id,
        amount: p.amount,
      })),
      tags: selectedTags,
    };

    updateMutation.mutate(input);
  };

  return (
    <div className="flex flex-col h-full">
      {/* 顶部导航 */}
      <div className="flex items-center justify-between px-6 py-4 border-b">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <h1 className="text-lg font-semibold">
            {isNew ? t("transactions.new.title") : t("transactions.edit.title")}
          </h1>
        </div>

        <Button onClick={handleSave} disabled={updateMutation.isPending}>
          <Save className="h-4 w-4 mr-1" />
          {t("common.save")}
        </Button>
      </div>

      {/* 编辑表单 */}
      <div className="flex-1 overflow-y-auto p-6">
        <div className="max-w-2xl mx-auto space-y-6">

          {/* 基本信息 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t("transactions.edit.basicInfo")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              {/* 日期 */}
              <div className="space-y-2">
                <Label>{t("transactions.edit.date")}</Label>
                <DatePicker value={date} onChange={setDate} />
              </div>

              {/* 描述 */}
              <div className="space-y-2">
                <Label>{t("transactions.edit.description")}</Label>
                <Input
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder={t("transactions.edit.descriptionPlaceholder")}
                />
              </div>

              {/* 交易类型 */}
              <div className="space-y-2">
                <Label>{t("transactions.edit.type")}</Label>
                <Select
                  value={transactionType ?? ""}
                  onChange={(v) => setTransactionType(v || null)}
                  options={[
                    { value: "", label: t("transactions.types.freeEntry") },
                    { value: "withdrawal", label: t("transactions.types.withdrawal") },
                    { value: "deposit", label: t("transactions.types.deposit") },
                    { value: "transfer", label: t("transactions.types.transfer") },
                  ]}
                />
              </div>

              {/* 分类 */}
              <div className="space-y-2">
                <Label>{t("transactions.edit.category")}</Label>
                <Select
                  value={categoryId ?? ""}
                  onChange={(v) => setCategoryId(v || null)}
                  options={[
                    { value: "", label: t("common.none") },
                    ...categories.map((c) => ({ value: c.id, label: c.name })),
                  ]}
                />
              </div>

              {/* 标签 */}
              <div className="space-y-2">
                <Label>{t("transactions.edit.tags")}</Label>
                <TagPicker value={selectedTags} onChange={setSelectedTags} />
              </div>
            </CardContent>
          </Card>

          {/* 分录编辑 */}
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-base">{t("transactions.edit.postings")}</CardTitle>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setPostings([...postings, { account_id: "", amount: "" }])}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  {t("transactions.edit.addPosting")}
                </Button>
              </div>
            </CardHeader>
            <CardContent>
              <PostingEditor
                postings={postings}
                accounts={accounts}
                onChange={setPostings}
              />

              {/* 平衡提示 */}
              <PostingBalanceIndicator postings={postings} />
            </CardContent>
          </Card>

        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// 分录平衡指示器
// ---------------------------------------------------------------------------
// 注: 借贷平衡验证由后端执行，前端仅显示提示信息

function PostingBalanceIndicator({ postings }: { postings: PostingInput[] }) {
  const { t } = useTranslation();

  // 前端不做金额计算（AGENTS.md 约束）
  // 平衡验证由后端 transaction_update 执行

  return (
    <div className="mt-4 p-3 rounded-md bg-muted/30">
      <div className="flex items-center justify-between">
        <span className="text-sm">{t("transactions.edit.balanceStatus")}</span>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">
            {t("transactions.edit.balanceVerifiedOnSave")}
          </span>
        </div>
      </div>
    </div>
  );
}
```

### 3.6 分录编辑组件

```typescript
// src/components/transactions/PostingEditor.tsx

import { useTranslation } from "react-i18next";
import type { PostingInput, AccountDto } from "@/types";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Trash2, ArrowUpRight, ArrowDownRight } from "lucide-react";

interface PostingEditorProps {
  postings: PostingInput[];
  accounts: AccountDto[];
  onChange: (postings: PostingInput[]) => void;
}

export function PostingEditor({ postings, accounts, onChange }: PostingEditorProps) {
  const { t } = useTranslation();

  const handleAccountChange = (index: number, accountId: string) => {
    const updated = [...postings];
    updated[index] = { ...updated[index], account_id: accountId };
    onChange(updated);
  };

  const handleAmountChange = (index: number, amount: string) => {
    const updated = [...postings];
    updated[index] = { ...updated[index], amount };
    onChange(updated);
  };

  const handleRemove = (index: number) => {
    if (postings.length > 2) {
      onChange(postings.filter((_, i) => i !== index));
    }
  };

  return (
    <div className="space-y-2">
      {postings.map((posting, index) => {
        // 注: 前端不做金额计算（AGENTS.md 约束），借贷标识仅根据字符串第一个字符判断

        return (
          <div key={index} className="flex items-center gap-3">
            {/* 借贷标识 */}
            <div className="w-6 flex justify-center">
              {/* 简化的借贷标识，仅基于输入字符串 */}
            </div>

            {/* 账户选择 */}
            <Select
              value={posting.account_id}
              onChange={(v) => handleAccountChange(index, v)}
              options={[
                { value: "", label: t("transactions.edit.selectAccount") },
                ...accounts.map((a) => ({ value: a.id, label: a.name })),
              ]}
              className="flex-1"
            />

            {/* 金额输入 */}
            <Input
              type="number"
              step="0.01"
              value={posting.amount}
              onChange={(e) => handleAmountChange(index, e.target.value)}
              placeholder="0.00"
              className="w-24"
            />

            {/* 删除按钮 */}
            {postings.length > 2 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => handleRemove(index)}
              >
                <Trash2 className="h-4 w-4" />
              </Button>
            )}
          </div>
        );
      })}
    </div>
  );
}
```

---

## 四、交易删除设计

### 4.1 删除预览

```rust
/// 删除预览信息
#[derive(Debug, Serialize)]
pub struct DeletePreview {
    pub transaction_id: String,
    pub description: String,
    pub date: String,
    pub posting_count: u32,
    pub has_tags: bool,
    pub can_delete: bool,
    pub warning_message: Option<String>,
}

/// 获取删除预览
pub fn get_delete_preview(
    conn: &Connection,
    tx_id: &str,
) -> Result<DeletePreview, AppError> {
    // 查询交易基本信息
    let (description, date): (String, String) = conn.query_row(
        "SELECT description, date FROM transactions WHERE id = ?1",
        [tx_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).map_err(|_| AppError::NotFound("errors.transaction.notFound".to_string()))?;

    // 查询分录数量
    let posting_count: u32 = conn.query_row(
        "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
        [tx_id],
        |row| row.get(0),
    )?;

    // 查询是否有标签
    let has_tags: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transaction_tags WHERE transaction_id = ?1)",
        [tx_id],
        |row| row.get(0),
    )?;

    // 检查是否可以删除
    // 注: 本系统不存储 transaction_type，无法判断系统交易类型
    // 简化规则：所有用户交易都可以删除
    // 系统交易（期初余额）由账户删除时联动处理，不单独警告

    Ok(DeletePreview {
        transaction_id: tx_id.to_string(),
        description,
        date,
        posting_count,
        has_tags,
        can_delete: true, // 用户交易都可以删除
        warning_message: None, // 无类型判断，无警告
    })
}
```

### 4.2 删除实现

```rust
/// 删除交易
pub fn delete_transaction(
    conn: &Connection,
    tx_id: &str,
) -> Result<(), AppError> {
    // 1. 检查交易存在
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
        [tx_id],
        |row| row.get(0),
    )?;

    if !exists {
        return Err(AppError::NotFound("errors.transaction.notFound".to_string()));
    }

    // 2. 执行删除（事务）
    let tx = conn.transaction()?;

    // 删除标签关联（CASCADE 会自动处理，但显式删除更清晰）
    tx.execute(
        "DELETE FROM transaction_tags WHERE transaction_id = ?1",
        [tx_id],
    )?;

    // 删除分录（CASCADE 会自动处理）
    tx.execute(
        "DELETE FROM postings WHERE transaction_id = ?1",
        [tx_id],
    )?;

    // 删除交易
    tx.execute(
        "DELETE FROM transactions WHERE id = ?1",
        [tx_id],
    )?;

    tx.commit()?;

    Ok(())
}
```

### 4.3 Tauri Commands

```rust
/// 获取删除预览
#[tauri::command]
pub async fn preview_delete_transaction(
    id: String,
    state: State<'_, AppState>,
) -> Result<DeletePreview, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    do_get_delete_preview(&conn, &id).map_err(|e| e.to_string())
}

/// 删除交易
#[tauri::command]
pub async fn delete_transaction(
    id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("errors.database.locked".to_string())?;

    let conn = db.get_conn();
    do_delete_transaction(&conn, &id).map_err(|e| e.to_string())
}
```

### 4.4 前端删除确认对话框

```typescript
// src/components/transactions/TransactionDeleteDialog.tsx

import { useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import type { DeletePreview } from "@/types";
import { previewDeleteTransaction, deleteTransaction } from "@/utils/api";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Badge } from "@/components/ui/badge";
import { AlertCircle, Info } from "lucide-react";
import { toast } from "sonner";

interface TransactionDeleteDialogProps {
  transactionId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function TransactionDeleteDialog({
  transactionId,
  open,
  onOpenChange,
  onSuccess,
}: TransactionDeleteDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  // 加载删除预览
  const { data: preview } = useQuery({
    queryKey: ["transaction-delete-preview", transactionId],
    queryFn: () => previewDeleteTransaction(transactionId),
    enabled: open,
    staleTime: 0, // 禁用缓存（AGENTS.md 约束）
    gcTime: 0,
  });

  // 删除 Mutation
  const deleteMutation = useMutation({
    mutationFn: () => deleteTransaction(transactionId),
    onSuccess: () => {
      toast.success(t("transactions.delete.success"));
      queryClient.invalidateQueries({ queryKey: ["transaction-list"] });
      queryClient.invalidateQueries({ queryKey: ["transaction-detail", transactionId] });
      queryClient.invalidateQueries({ queryKey: ["account-transactions"] });
      onOpenChange(false);
      onSuccess?.();
    },
    onError: (err: Error) => {
      toast.error(err.message);
    },
  });

  const handleConfirm = () => {
    deleteMutation.mutate();
  };

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("transactions.delete.title")}</AlertDialogTitle>
          <AlertDialogDescription>
            {preview ? (
              <div className="space-y-3">
                {/* 交易信息 */}
                <div className="p-3 rounded-md bg-muted/30">
                  <div className="font-medium">{preview.description}</div>
                  <div className="text-sm text-muted-foreground">
                    {preview.date} · {preview.posting_count} {t("transactions.delete.entries")}
                  </div>
                </div>

                {/* 警告信息 */}
                {preview.warning_message && (
                  <div className="flex items-start gap-2 p-3 rounded-md bg-yellow-50 border border-yellow-200">
                    <AlertCircle className="h-4 w-4 text-yellow-600 mt-0.5" />
                    <span className="text-sm text-yellow-800">
                      {preview.warning_message}
                    </span>
                  </div>
                )}

                {/* 确认提示 */}
                <p className="text-sm">
                  {t("transactions.delete.confirm")}
                </p>
              </div>
            ) : (
              <p>{t("common.loading")}</p>
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={deleteMutation.isPending}>
            {t("common.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={handleConfirm}
            disabled={deleteMutation.isPending || !preview?.can_delete}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {deleteMutation.isPending
              ? t("common.deleting")
              : t("transactions.delete.confirmButton")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
```

---

## 五、前端 API 补充

```typescript
// src/utils/api.ts 补充

export async function getTransactionDetail(
  id: string,
): Promise<TransactionDetail> {
  return invoke("get_transaction_detail", { id });
}

export async function updateTransaction(
  id: string,
  input: UpdateTransactionInput,
): Promise<TransactionDetail> {
  return invoke("update_transaction", { id, input });
}

export async function previewDeleteTransaction(
  id: string,
): Promise<DeletePreview> {
  return invoke("preview_delete_transaction", { id });
}

export async function deleteTransaction(id: string): Promise<void> {
  return invoke("delete_transaction", { id });
}
```

---

## 六、单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_transaction_detail() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let detail = get_transaction_detail(&conn, &tx_id).unwrap();

        assert_eq!(detail.id, tx_id);
        assert_eq!(detail.postings.len(), 2);
        assert!(detail.is_balanced);
        assert_eq!(detail.debit_total + detail.credit_total, 0);
    }

    #[test]
    fn test_update_transaction_description() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let input = UpdateTransactionInput {
            description: Some("新的描述".into()),
            ..Default::default()
        };

        let result = update_transaction(&conn, &tx_id, &input).unwrap();
        assert_eq!(result.description, "新的描述");
    }

    #[test]
    fn test_update_transaction_postings() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let input = UpdateTransactionInput {
            postings: Some(vec![
                PostingInput { account_id: "acct-1".into(), amount: "50.00".into() },
                PostingInput { account_id: "acct-2".into(), amount: "-50.00".into() },
            ]),
            ..Default::default()
        };

        let result = update_transaction(&conn, &tx_id, &input).unwrap();
        assert_eq!(result.postings.len(), 2);
        assert_eq!(result.debit_total, 5000);
    }

    #[test]
    fn test_update_unbalanced_postings_fails() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let input = UpdateTransactionInput {
            postings: Some(vec![
                PostingInput { account_id: "acct-1".into(), amount: "100.00".into() },
                PostingInput { account_id: "acct-2".into(), amount: "-50.00".into() }, // 不平衡
            ]),
            ..Default::default()
        };

        let result = update_transaction(&conn, &tx_id, &input);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_transaction() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        // 删除
        delete_transaction(&conn, &tx_id).unwrap();

        // 验证已删除
        let exists: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM transactions WHERE id = ?1)",
            [&tx_id],
            |row| row.get(0),
        ).unwrap();
        assert!(!exists);

        // 验证分录也已删除
        let posting_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM postings WHERE transaction_id = ?1",
            [&tx_id],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(posting_count, 0);
    }

    #[test]
    fn test_delete_preview() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let preview = get_delete_preview(&conn, &tx_id).unwrap();

        assert!(preview.can_delete);
        assert_eq!(preview.posting_count, 2);
    }

    #[test]
    fn test_delete_nonexistent_fails() {
        let (_dir, conn) = test_env();

        let result = delete_transaction(&conn, "nonexistent-id");
        assert!(result.is_err());
    }

    #[test]
    fn test_update_empty_input_fails() {
        let (_dir, conn) = test_env();
        let tx_id = create_test_transaction(&conn);

        let input = UpdateTransactionInput::default();

        let result = update_transaction(&conn, &tx_id, &input);
        assert!(result.is_err());
    }
}
```

---

## 七、设计决策总结

| 决策                     | 说明                                                     |
| ------------------------ | -------------------------------------------------------- |
| **详情页显示完整分录**   | 与列表页不同，详情页显示完整账户信息、借贷标识、金额合计 |
| **编辑支持部分更新**     | 只修改提供的字段，其他保持不变                           |
| **分录替换而非追加**     | 编辑分录时，完全替换而非追加修改                         |
| **删除需预览确认**       | 显示交易信息和警告，防止误删                             |
| **系统交易可删除但警告** | opening_balance、reconciliation 可删除，但显示警告       |
| **CASCADE 自动清理**     | 分录和标签关联通过外键 CASCADE 自动删除                  |

---

## 八、国际化文案

```json
// src/i18n/locales/zh.json 补充

{
  "transactions": {
    "detail": {
      "title": "交易详情",
      "basicInfo": "基本信息",
      "postings": "分录",
      "metadata": "元信息",
      "date": "日期",
      "description": "描述",
      "type": "交易类型",
      "category": "分类",
      "tags": "标签",
      "balanced": "已平衡",
      "unbalanced": "未平衡",
      "entries": "条分录",
      "debitTotal": "借方合计",
      "creditTotal": "贷方合计",
      "balance": "差额"
    },
    "edit": {
      "title": "编辑交易",
      "basicInfo": "基本信息",
      "postings": "分录编辑",
      "date": "日期",
      "description": "描述",
      "descriptionPlaceholder": "输入交易描述",
      "type": "交易类型",
      "category": "分类",
      "tags": "标签",
      "dateRequired": "请选择日期",
      "descriptionRequired": "请输入描述",
      "minPostings": "至少需要2条分录",
      "unbalanced": "分录不平衡",
      "balanced": "已平衡",
      "balanceStatus": "平衡状态",
      "success": "交易已更新",
      "addPosting": "添加分录",
      "selectAccount": "选择账户"
    },
    "delete": {
      "title": "删除交易",
      "confirm": "确定要删除此交易吗？此操作不可撤销。",
      "confirmButton": "确认删除",
      "entries": "条分录",
      "success": "交易已删除"
    },
    "notFound": "交易不存在"
  }
}
```
