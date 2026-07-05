# Transaction Form Enter 提交 & 列表性能优化 执行计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** TransactionForm/QuickAddDialog 支持 Enter 提交 + 交易列表页滚动性能优化

**Architecture:** 5 个前端文件，不改后端、不改 SQL、不改缓存策略。Enter 提交通过 `<form>` 元素复用原生行为；性能优化通过增大 pageSize + React.memo + useCallback/useRef 减少 IPC 往返和重渲染。

**Tech Stack:** React 19, TypeScript 5.8, @tanstack/react-virtual, @tanstack/react-query

---

### Task 1: TransactionForm.tsx — 回车提交

**文件：**
- Modify: `src/components/transactions/TransactionForm.tsx`

- [ ] **Step 1: handleSubmit 加 FormEvent 参数**

在 `handleSubmit` 函数签名末尾加 `(e: React.FormEvent)`，首行加 `e.preventDefault();`

```typescript
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;

    const data: CreateTransactionInput = {
      date: date.trim(),
      description: description.trim(),
      categoryId:
        categoryId === NO_CATEGORY_VALUE
          ? isEditing
            ? CLEAR_CATEGORY_MARKER
            : null
          : categoryId,
      postings: postings.map((p) => ({
        accountId: p.accountId,
        amount: decimalToCents(p.amount) ?? 0,
      })),
    };

    onSubmit(data);
  };
```

- [ ] **Step 2: JSX 包 `<form>` + 按钮改 type**

用 `<form onSubmit={handleSubmit}>` 包裹 `<div className="space-y-4">` 和 `<DialogFooter>`。取消按钮加 `type="button"`，保存按钮加 `type="submit"`。

```tsx
        <DialogContent className="sm:max-w-[40%] sm:min-w-[420px] max-h-[90vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>{dialogTitle}</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleSubmit}>
            <div className="space-y-4">
              {/* ... 所有表单内容不变 ... */}
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                {t("common.cancel")}
              </Button>
              <Button type="submit" disabled={!canSubmit}>
                {isLoading ? t("common.processing") : t("common.save")}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
```

- [ ] **Step 3: 验证** — 运行 `make check`，确认 tsc/eslint/prettier 通过

---

### Task 2: QuickAddDialog.tsx — 回车提交

**文件：**
- Modify: `src/components/transactions/QuickAddDialog.tsx`

- [ ] **Step 1: handleSubmit 加 FormEvent 参数**

```typescript
  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;

    const input: QuickAddInput = {
      mode,
      amount: parsedAmount.toFixed(2),
      sourceAccountId:
        mode === "expense" || mode === "transfer" ? sourceAccountId : undefined,
      destinationAccountId:
        mode === "income" || mode === "transfer"
          ? destinationAccountId
          : undefined,
      categoryId: categoryId === NO_CATEGORY_VALUE ? undefined : categoryId,
      description: description.trim() || undefined,
      date,
    };

    mutation.mutate(input);
  };
```

- [ ] **Step 2: JSX 包 `<form>` + 按钮改 type**

与 Task 1 Step 2 相同模式。

- [ ] **Step 3: 验证** — `make check`

---

### Task 3: TransactionsPage.tsx — pageSize + useMemo

**文件：**
- Modify: `src/pages/TransactionsPage.tsx`

- [ ] **Step 1: pageSize 20 → 50**

filter 初始值和 `handleClearFilter` 中同步改为 `pageSize: 50`。

- [ ] **Step 2: flattenedItems 用 useMemo**

```typescript
import { useState, useCallback, useEffect, useMemo } from "react";

const flattenedItems = useMemo(
  () => flattenToVirtualItems(mergeDateGroups(pages)),
  [pages],
);
```

移除原来的 `const flattenedItems = flattenToVirtualItems(allDateGroups);`（如果有 `allDateGroups` 中间变量也不用了，直接合并）。

- [ ] **Step 3: 验证** — `make check`

---

### Task 4: TransactionCard.tsx — React.memo

**文件：**
- Modify: `src/components/transactions/TransactionCard.tsx`

- [ ] **Step 1: 加 memo 包裹**

```typescript
import { useState, memo } from "react";

export const TransactionCard = memo(function TransactionCard({
```

- [ ] **Step 2: 验证** — `make check`

---

### Task 5: VirtualTransactionList.tsx — 性能优化

**文件：**
- Modify: `src/components/transactions/VirtualTransactionList.tsx`

- [ ] **Step 1: estimateSize 用 useCallback**

```typescript
import { useRef, useEffect, memo, useCallback } from "react";

const estimateSize = useCallback((index: number) => {
  const item = items[index];
  if (!item) return 64;
  return item.type === "date-header" ? 44 : 64;
}, [items]);
```

- [ ] **Step 2: IntersectionObserver 用 ref 稳定回调 + rootMargin 200px**

```typescript
const observerCallbackRef = useRef(fetchNextPage);
observerCallbackRef.current = fetchNextPage;

useEffect(() => {
  if (!hasNextPage || isFetchingNextPage) return;

  const observer = new IntersectionObserver(
    (entries) => {
      const [entry] = entries;
      if (entry?.isIntersecting) {
        observerCallbackRef.current();
      }
    },
    {
      root: scrollContainerRef.current,
      threshold: 0.1,
      rootMargin: "200px",
    },
  );

  if (sentinelRef.current) {
    observer.observe(sentinelRef.current);
  }

  return () => observer.disconnect();
}, [hasNextPage, isFetchingNextPage]);
```

- [ ] **Step 3: 去掉 sentinel 的 marginTop hack**

```tsx
      {/* 替换前 */}
      {hasNextPage && items.length > 0 && (
        <div
          ref={sentinelRef}
          className="h-4"
          style={{ marginTop: `${Math.max(0, items.length - 5) * 64}px` }}
        />
      )}

      {/* 替换后 */}
      {hasNextPage && items.length > 0 && (
        <div ref={sentinelRef} className="h-4" />
      )}
```

- [ ] **Step 4: 验证** — `make check`

---

### Task 6: 最终验证

- [ ] **Step 1: 全量检查**

```bash
cd /home/sophia-dawn/code/GlimmerX && make check
```

预期：tsc 无错误，eslint 无警告，prettier 无格式问题。

- [ ] **Step 2: Rust 回归测试**

```bash
cd /home/sophia-dawn/code/GlimmerX/src-tauri && cargo test
```

预期：全部测试通过（无后端改动，仅回归确认）。
