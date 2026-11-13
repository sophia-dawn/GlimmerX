# 03 - 核心概念模型

> 返回 [DESIGN.md](../DESIGN.md)

## 复式记账模型

```
每笔交易 (Transaction)
├── 日期 (Date)
├── 描述 (Description)
├── 分类 (Category) [可选]
├── 交易类型 (TransactionType) [可选]
│   ├── withdrawal       — 支出：资产 → 支出
│   ├── deposit          — 收入：收入 → 资产
│   ├── transfer         — 转账：资产 → 资产
│   ├── opening_balance  — 期初余额（系统）
│   ├── reconciliation   — 对账调整（系统）
│   └── NULL             — 自由复式记账（默认）
└── 分录 (Postings)
    ├── 借方: 账户A  +¥100
    └── 贷方: 账户B  -¥100
    （借贷必须平衡，总和为零）
```

> **交易类型设计理念**: `transaction_type` 是可选辅助字段，用于「快速记账」场景（预设账户组合）、导入识别、报表统计。其值为 `NULL` 时表示自由复式记账，不限制 Posting 组合，保留复式记账本质灵活性。

## 账户体系

```
账户 (Account)
├── 类型
│   ├── 资产 (Assets)     — 现金、银行卡、支付宝、微信余额等
│   ├── 负债 (Liabilities) — 信用卡、花呗、借款等
│   ├── 收入 (Income)      — 工资、奖金、投资收入等
│   ├── 支出 (Expenses)    — 餐饮、交通、房租、订阅等
│   └── 权益 (Equity)      — 初始余额、调整项（复式记账平衡用）
│
├── 层级结构 (树形)
│   └── 如: Assets/Bank/招商银行
│       Assets/Cash/钱包
│       Expenses/Food/外卖
│
└── 状态
    ├── 活跃 (active)
    └── 已关闭 (inactive)

> **注意**: 权益 (Equity) 账户由系统自动管理（用于期初余额），禁止用户手动创建。
```

## 预算模型

```
预算 (Budget)
├── 绑定分类 (category_id) — 仅 expense 类型分类
├── 周期 (period)
│   ├── monthly — 月度预算
│   └── weekly  — 周度预算
├── 限额金额 (amount) — 必须为正数，整数存储（分）
├── 结余滚存 (rollover) — 开启后上月结余自动累加到本月
│
├── 计算字段（BudgetStatus）
│   ├── spent       — 当前周期已支出金额
│   ├── available   — 可用预算 = amount + rolloverAmount
│   ├── remaining   — 剩余预算 = available - spent
│   └── overBudget  — 超支标记 = spent > available
│
└── 约束规则
    ├── 每个分类只能有一个预算（唯一索引）
    ├── 只有 expense 类型分类可设置预算
    └── 预算金额必须为正数
```

> **结余滚存**: `rollover` 字段已预留，实际计算逻辑待 BUD-4 实现。当前 `rolloverAmount` 始终为 0。

## 分类

```
分类 (Category) — 互斥的，一笔交易只能有一个分类
├── 类型 (type)
│   ├── income  — 收入分类（用于 deposit 交易）
│   └── expense — 支出分类（用于 withdrawal 交易，可设置预算）
│
├── 扁平结构（无层级）
│   ├── 每个分类必须指定 type
│   ├── 同一 type 下 name 唯一
│   └── 不同 type 可有相同 name（如 income/餐饮 和 expense/餐饮）
│
└── 删除约束：
    ├── 有预算绑定 → 阻止删除，提示先删除预算
    └── 有交易关联 → 解除关联（category_id 设为 NULL，交易保留）

示例：
  Expense 分类：餐饮、交通、房租
  Income 分类：工资、奖金、投资收益
```

> **分类与预算关系**: 只有 `type='expense'` 的分类可设置预算限额。预算绑定后删除分类会被阻止，需先删除预算。
