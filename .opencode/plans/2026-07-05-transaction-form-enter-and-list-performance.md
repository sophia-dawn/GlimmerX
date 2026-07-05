# Transaction Form Enter 提交 & 列表性能优化

## Issue 1：回车提交表单

### 涉及文件

- `src/components/transactions/TransactionForm.tsx`
- `src/components/transactions/QuickAddDialog.tsx`

### 方案

两个对话框采用相同模式：

1. `handleSubmit` 函数接受 `React.FormEvent` 参数，首行调用 `e.preventDefault()`
2. 把对话框内容区（`<div className="space-y-4">` + `<DialogFooter>`）用 `<form onSubmit={handleSubmit}>` 包裹
3. 取消按钮显式设为 `type="button"`，保存按钮改为 `type="submit"`

### 具体变更

**TransactionForm.tsx：**

```typescript
// handleSubmit 签名变更
const handleSubmit = (e: React.FormEvent) => {
  e.preventDefault();
  if (!canSubmit) return;
  // ... 原有逻辑不变
};

// JSX 结构
<DialogContent>
  <DialogHeader>...</DialogHeader>
  <form onSubmit={handleSubmit}>
    <div className="space-y-4">...</div>
    <DialogFooter>
      <Button type="button" onClick={() => onOpenChange(false)}>
        {t("common.cancel")}
      </Button>
      <Button type="submit" disabled={!canSubmit}>
        {isLoading ? t("common.processing") : t("common.save")}
      </Button>
    </DialogFooter>
  </form>
</DialogContent>
```

**QuickAddDialog.tsx：** 完全相同的模式。

---

## Issue 2：列表页性能优化

### 涉及文件

- `src/pages/TransactionsPage.tsx`
- `src/components/transactions/TransactionCard.tsx`
- `src/components/transactions/VirtualTransactionList.tsx`

### 方案

#### 文件 1：`TransactionsPage.tsx`

1. `pageSize` 从 20 改为 50（`handleClearFilter` 同步更新），减少 IPC 往返 60%
2. `flattenedItems` 用 `useMemo` 稳定引用，配合 `React.memo` 阻止 VirtualList 不必要重渲染

```typescript
// filter 初始值
const [filter, setFilter] = useState<EnhancedTransactionFilter>({
  pageSize: 50,
  sortBy: "date",
  sortOrder: "desc",
});

// handleClearFilter
const handleClearFilter = useCallback(() => {
  setFilter({ pageSize: 50, sortBy: "date", sortOrder: "desc" });
  setSearchQuery("");
}, []);

// flattenedItems 用 useMemo
const flattenedItems = useMemo(
  () => flattenToVirtualItems(mergeDateGroups(pages)),
  [pages],
);
```

#### 文件 2：`TransactionCard.tsx`

`React.memo` 包裹组件，阻止数据未变化时的重渲染。

```typescript
import { useState, memo } from "react";

export const TransactionCard = memo(function TransactionCard({
  transaction,
  onEdit,
  onDelete,
}: TransactionCardProps) {
  // 代码不变
});
```

#### 文件 3：`VirtualTransactionList.tsx`

1. `estimateSize` 用 `useCallback` 稳定引用，防止 Virtualizer 反复重算
2. IntersectionObserver 回调通过 ref 获取最新 `fetchNextPage`，去掉 `fetchNextPage` 依赖，减少 observer 重建
3. `rootMargin` 从 `100px` 改为 `200px`，提前触发下一页加载
4. 去掉 sentinel 元素上错误的 `marginTop` 公式（该公式将 sentinel 过度推后，导致需要额外滚动才触发加载）

```typescript
import { useRef, useEffect, memo, useCallback } from "react";

// useCallback 稳定 estimateSize
const estimateSize = useCallback((index: number) => {
  const item = items[index];
  if (!item) return 64;
  return item.type === "date-header" ? 44 : 64;
}, [items]);

// ref 稳定 fetchNextPage 引用
const observerCallbackRef = useRef(fetchNextPage);
observerCallbackRef.current = fetchNextPage;

// IntersectionObserver effect
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

// sentinel 去掉 marginTop hack
{hasNextPage && items.length > 0 && (
  <div ref={sentinelRef} className="h-4" />
)}
```

---

## 测试验证

所有修改不改变业务逻辑，回归验证：
1. `make check` — tsc + eslint + prettier 通过
2. 手动测试 Enter 提交正常
3. 手动测试列表滚动加载正常
