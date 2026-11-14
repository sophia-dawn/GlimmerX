# 25. AI 记账增强与 Agent 接入

> 状态：**提案（未实现）**
> 目标：让 AI 能记支出、收入、**转账**、**新建账户**；并让外部 agent（WorkBuddy / Claude / Cursor…）
> 通过 MCP 驱动这个账本，同时软件自身的 AI 也能驱动自己。
> 前提：本账本是 local-first 的加密单用户账本，没有服务端。

---

## 1. 现状：AI 记账是一条硬编码的旁路

今天的链路（`services/ai.rs` → `commands/ai.rs`）是这样一条直线：

```
文本 → prompt（写死 6 个字段）→ /chat/completions → JSON → 硬编码校验
     → 账户名模糊匹配 → quick_add（只支持 expense / income）→ 落库
```

四处写死了边界：

| 位置 | 写死的东西 | 后果 |
| --- | --- | --- |
| `services/ai.rs::build_system_prompt` | 返回 `{mode, amount, categoryName, accountHint, date, description}` | 转账要两个账户，建账户要路径与类型，字段表装不下 |
| `services/ai.rs::validate_mode` | 只放行 `expense` / `income` | 转账被拦（注释写着 "transfer is phase 2"） |
| `commands/ai.rs::resolve_source_account` | 账户靠**名字**双向 `contains` 猜 | 猜错就静默回落默认账户 |
| `commands/ai.rs` | 直接调 `quick_add_with_category` 落库 | 没有确认、没有审计、没有幂等 |

**已经具备、可以复用的底子**（这些不是问题）：

- UI 层已经完整支持转账：`QuickAddDialog` 的 `TransactionMode = "expense" | "income" | "transfer"`，
  转账的 UI 早就在了，缺的只是 AI 到不了那里。
- db 层已经有转账的**另一条**实现：`db/accounts/transfer.rs::create_transfer`，
  端点必须是 asset / liability，但它**不接受日期**（硬编码 `today_date()`）。
- `create_account_with_path` 已经支持 `Assets/xxx` / `Liabilities/xxx` 形式的层级路径。
- 期初余额有 `db/accounts/opening.rs::create_opening_balance`（走 equity 分录）。
- 安全面做得很到位：API Key 只在 Rust 侧加密库、请求不经 webview、金额全程整数分、
  落库是单事务、await 前释放 DB 锁。

---

## 2. 架构：一条能力层，两个驱动

核心判断：**不要把「AI 能做什么」和「agent 怎么驱动」混成一件事。**

正确做法是抽出一层**账本能力层**（Ledger Capability Layer）——写账本这件事的**唯一入口**——
然后让两个驱动共用它：

```
                 ┌──────────────────────────────────────────────┐
                 │        账本能力层（Rust，唯一入口）           │
                 │  list_accounts / list_categories            │
                 │  record_expense / record_income             │
                 │  record_transfer / create_account           │
                 │  query_transactions / get_balances          │
                 └───────────────┬──────────────────────────────┘
                                 │  （都直接复用现有 db 层函数）
        ┌────────────────────────┴────────────────────────┐
        │                                                  │
┌───────▼────────────────┐              ┌────────────────▼────────────────┐
│  驱动 A：软件自己驱动   │              │  驱动 B：外部 agent 驱动        │
│  内部 AI（应用内）      │              │  MCP Server（进程内，HTTP）      │
│  动作枚举 + 确认卡      │              │  工具集 + 信任模型              │
└────────────────────────┘              └─────────────────────────────────┘
        封闭、可控、可测                        开放、灵活、可扩展
```

**两条路共用 db 层，不共用编排。** 这是本方案最重要的分界线：

- **驱动 A（内部 AI）不需要 MCP**。MCP 是给"编排不由我们控制"的外部 agent 用的协议。
  内部 AI 的动作集合是**封闭且已知**的四个（支出 / 收入 / 转账 / 建账户），
  用一个**判别式 JSON schema**（discriminated union）表达就够了，
  比 function-calling 循环更可控、更省 token、更好测。
- **驱动 B（MCP）不需要 prompt**。外部 agent 自己会规划，它需要的是
  **窄的、类型化的、可审计的操作**，而不是我们替它想好的字段表。

---

## 3. 能力层：具体长什么样

新模块 `src-tauri/src/ledger/mod.rs`（或 `src-tauri/src/services/ledger.rs`）。

### 3.1 读

```rust
pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountView>, AppError>;
pub fn list_categories(conn: &Connection) -> Result<Vec<CategoryView>, AppError>;
pub fn account_balances(conn: &Connection) -> Result<Vec<BalanceView>, AppError>;
pub fn query_transactions(conn: &Connection, q: &Query) -> Result<Vec<TxView>, AppError>;
pub fn period_summary(conn: &Connection, r: &DateRange) -> Result<SummaryView, AppError>;
```

`SummaryView` 直接复用 `db/reports` 已有的汇总逻辑，不要另写一套。

### 3.2 写

```rust
pub struct RecordRequest {
    pub kind: RecordKind,            // Expense | Income | Transfer
    pub amount_cents: i64,           // 整数分，绝不接受字符串或浮点
    pub date: String,                // YYYY-MM-DD
    pub description: String,
    pub source_account: AccountRef,      // 支出/转账的起点
    pub destination_account: AccountRef, // 收入/转账的终点
    pub category: Option<CategoryRef>,   // 仅支出/收入
}

pub enum AccountRef {
    Id(String),
    Name(String),   // 名称由能力层解析成 id；解析不到则报错，不静默兜底
}
```

**`AccountRef` 同时接受 id 和名字**，但名字解析失败时**必须报错**，不能像现在的
`resolve_source_account` 那样静默回落默认账户 —— 静默兜底是当前 AI 路径最大的可诊断性缺陷。

### 3.3 写操作的结果要能展示

```rust
pub struct LedgerOutcome {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub postings: Vec<PostingView>,   // 已经派生好的分录，用于预览
    pub warnings: Vec<String>,        // "分类「吃」是新建的" / "账户「微信」按名称匹配到「微信钱包」"
}
```

`warnings` 是关键：**所有"我替你猜了"的地方都必须说出来**。
分类被自动创建、账户靠模糊匹配、金额被截断——都要能被用户看见。

### 3.4 纪律

- 能力层**不写 SQL**，只调用现有 db 层函数（`quick_add_with_category`、
  `create_account_with_path`、`create_opening_balance`、reports）。
  复式平衡、单事务、分类镜像账户这些不变量自动继承。
- 能力层是**唯一**允许写账本的地方。以后新增功能不许再从 `commands/` 直接写库。
- 能力的写操作**不返回 `void`**，一律返回 `LedgerOutcome`。

---

## 4. 驱动 A：软件自己驱动（内部 AI）

### 4.1 把字段表换成动作判别式

现在的 prompt 要求模型返回固定的 6 个字段。改成：

```json
{
  "action": "expense" | "income" | "transfer" | "create_account",
  "amount": "18.00",
  "date": "2026-11-14",
  "description": "中午吃饭",
  "sourceAccount": "微信钱包",
  "destinationAccount": null,
  "category": "餐饮美食",
  "accountPath": null
}
```

`transfer` 用 `sourceAccount` / `destinationAccount` 两端；
`create_account` 用 `accountPath`（如 `Assets/支付宝`）+ `kind` + 可选 `openingBalance`。

### 4.2 加确认卡（这是最重要的一条）

`ai_parse_transaction` 现在**直接落库**。改成两段：

```
模型产出提案 → 前端展示确认卡（金额/账户/分类/日期/派生出的分录）
             → 用户可改 → 确认 → 才写库
```

确认卡复用 `QuickAddDialog` 的表单结构（它已经支持三种 mode 和全部字段），
给它加一个 `variant: "create" | "review"`：review 模式下预填 + 可编辑 + 提交。

**为什么必须做**：本项目定了**硬删除、无撤销**。AI 的分类模糊匹配和账户名匹配都会出错，
一次错误识别 = 永久脏数据。而提案数据后端**本来就返回了**（`TransactionDto` 里带完整分录），
前端却把它丢掉了——渲染确认卡不需要新增任何后端字段。

确认之后**不需要新的写命令**：走现有的 `quick_add_transaction`
（带上 P0-3 新增的 `categoryName`）与 `account_create` 即可。
AI 只负责提案，落库仍走既有命令路径。

### 4.3 建账户的默认策略

账户名是用户的语义体系，模型乱建会污染分类树。所以：

- 模型**可以提出**建账户的提案（`create_account` 动作）。
- 但默认**不自动建**，走确认卡。
- 提案前先与现有账户做名称匹配，能匹配上就复用，别新建。
- 设置里给一个"允许 AI 自动建账户"的开关，默认关。

---

## 5. 驱动 B：外部 agent 驱动（MCP）

### 5.1 架构选型：MCP Server 跑在 App 进程内

**不要**做成独立二进制。三条理由：

1. **密钥**。数据库是 SQLCipher 加密的，密钥只在解锁后的内存里。
   独立进程要么再问一次密码，要么把密钥缓存进 OS keychain —— 两者都是新的密钥暴露面。
2. **锁语义**。进程内跑 = 共享同一把 `Mutex<Connection>`、同一套校验、同一套 i18n 错误文案；
   锁屏时 `AppState.database` 是 `None`，工具自然返回"账本已锁定"。
3. **分发**。不需要第二个可执行文件，用户装的是原来那个 App。

**也不要**照抄 `tauri-plugin-mcp-bridge` 那套（截图 + DOM + IPC 执行）。
那套是给"数据不能给 agent 看的应用"设计的，它的文档自己就写了
*"Not a fit when: the app holds data you must not show an agent"*。
账本恰恰相反：我们要给的是**窄的类型化操作**，不是数据库快照，也不是界面。

### 5.2 技术选型

| 项 | 选择 | 理由 |
| --- | --- | --- |
| SDK | `rmcp` 3.10（官方 Rust SDK） | 协议实现自己写容易漏：版本协商、流式响应、JSON-RPC 错误码 |
| 传输 | **Streamable HTTP**，绑 `127.0.0.1` | rmcp 明确不提供旧的 2024-11-05 HTTP+SSE 传输；Streamable HTTP 是当前规范预期的唯一端点 |
| Web 框架 | `axum` | `StreamableHttpService` 是 Tower service，直接 mount |
| 会话 | 无状态（`with_legacy_session_mode(false)` + `with_json_response(true)`） | 每次请求重建 handler；共享状态靠捕获的 `AppHandle`，不依赖进程内 session |
| 认证 | `Authorization: Bearer <token>` | token 存加密的 settings 表，跨重启稳定 |
| 端口 | 默认 `127.0.0.1:17823`，被占用则顺延 | 固定端口让用户的 `mcp.json` 不用每次改 |

`rmcp` 3.10 要求 Rust ≥ 1.88（项目当前 1.99.1 ✓）。
注意 3.x 有过多次破坏性变更（`Error` → `ErrorData`、`with_stateful_mode` → `with_legacy_session_mode`），
**必须把 SDK 关在一个模块里**，别让它的类型漏到 `commands/` 和 `db/`。

**接入方式**：在 `lib.rs` 的 `.setup(|app| { ... })` 钩子里起服务，
工具内部通过 `app.state::<AppState>()` 取库——`AppState` 本身**不需要改动**。

### 5.3 工具集（初版）

**读**（任何模式都可用）

| 工具 | 说明 |
| --- | --- |
| `ledger_status` | 是否解锁、当前账本路径、最近一次活动 |
| `list_accounts` | 账户 + 类型 + 余额 |
| `list_categories` | 分类 + 类型 |
| `query_transactions` | 按日期 / 账户 / 分类 / 关键字查 |
| `get_balances` | 账户余额 |
| `summary` | 区间收支汇总（复用 reports） |

**写**（受信任模型约束）

| 工具 | 参数 |
| --- | --- |
| `record_transaction` | `kind`(expense/income/transfer) · `amount_cents` · `date` · `description` · `source_account` · `destination_account` · `category` · `idempotency_key` |
| `create_account` | `path`(如 `Assets/支付宝`) · `kind`(asset/liability) · `description` · `opening_balance_cents` · `opening_balance_date` · `idempotency_key` |

**金额一律 `amount_cents`（整数分）**，不要让 agent 传字符串金额再解析——
`parse_amount_to_cents` 已经在能力层内部了，工具边界上直接收整数最省事也最不容易出错。

### 5.4 信任模型（三档）

| 档位 | 写操作行为 |
| --- | --- |
| **只读**（默认给外部 agent 的第一档） | 写工具直接返回"未授权" |
| **确认**（默认） | 后端 `emit` 事件 → 前端弹确认卡 → 用户点确认 → 放行；超时（120s）取消 |
| **信任** | 直接写，只记审计日志 |

确认流程（需要新增的前后端基础设施，项目目前**完全没有** Tauri 事件机制）：

```
MCP 写调用 → 能力层 → 需要确认 → 后端 emit "ledger://approval"
           → webview 弹确认卡 → 用户点确认
           → 前端调 approve_request(request_id, approved)
           → 后端 oneshot 放行 → 能力层写入 → 返回 LedgerOutcome
```

单槽设计：同一时刻只允许一个待确认（`Mutex<Option<PendingApproval>>`），
第二个请求明确报错而不是排队——排队会让 agent 那边超时语义变复杂。

---

## 6. 审计与幂等

agent 会重试。同一笔账记两次是不可接受的。

新增一张表（v3 未发版，可以直接进 `schema_v3.sql`；若 v3 已冻结则走 `schema_v4`）：

```sql
CREATE TABLE automation_log (
  id               TEXT PRIMARY KEY,
  source           TEXT NOT NULL,     -- 'mcp' | 'ai'
  tool             TEXT NOT NULL,
  request_json     TEXT NOT NULL,
  idempotency_key  TEXT,
  status           TEXT NOT NULL,     -- 'confirmed' | 'auto' | 'rejected' | 'failed'
  result_json      TEXT,
  transaction_id   TEXT,
  created_at       TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_automation_log_idem
  ON automation_log(idempotency_key) WHERE idempotency_key IS NOT NULL;
```

一张表同时解决三件事：

1. **幂等**：`idempotency_key` 唯一索引 → 重复调用直接返回上次的结果，不重复写。
2. **审计**："agent 帮我记了哪几笔"可查、可导出、可回溯。
3. **信任档位的证据**：信任模式下写的事，至少留得下痕迹。

---

## 7. Skill 的角色：用法说明书，不是能力

**能力靠 MCP 工具集，Skill 靠说明书。** 两者不能互相替代。

`skills/glimmerx-ledger/SKILL.md` 该写什么：

- **记账约定**：什么算转账（资产/负债之间搬钱，不产生损益）、金额单位是整数分、
  日期 `YYYY-MM-DD`、账户路径 `Assets/招商银行`（首段决定类型）。
- **何时用哪个工具**：`record_transaction` 记一笔、`create_account` 建账户、
  `query_transactions` 查历史、`summary` 看区间。
- **红线**：
  - 不确定账户对应哪个 id 时，先 `list_accounts`，**不要猜 id**；
  - 除非用户明确要求，不要调 `create_account`（账户名是用户的语义体系）；
  - 一次只记一笔，多笔要用户逐笔确认。
- **错误处理**：工具返回"未找到账户"时，重新 `list_accounts` 再试；
  不要自己编 id 重试。

Skill 安装位置：`~/.workbuddy/skills/glimmerx-ledger/`（用户级）或仓库 `skills/` 供分发。

---

## 8. 配置怎么设

### 8.1 App 侧（新增「自动化」设置区块）

- **本地 MCP 服务**：开关（默认关）+ 端口（默认 17823）+ 实际端口显示
- **访问令牌**：自动生成、只显示一次、可复制、可重置
- **信任模式**：只读 / 确认 / 信任
- **允许 AI 自动建账户**：开关（默认关）
- **复制 MCP 配置**按钮：一键复制下面这段

### 8.2 客户端侧（WorkBuddy）

写到 `~/.workbuddy/mcp.json`（注意是 `mcp.json`，不是 `.mcp.json`）：

```json
{
  "mcpServers": {
    "glimmerx": {
      "type": "http",
      "url": "http://127.0.0.1:17823/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

写完之后要在 **连接器管理页点「信任」** 才会启用（不会自动激活）。

其他客户端（Claude Code / Cursor）的配置格式相同，只是入口不同。

### 8.3 Skill 侧

把 `skills/glimmerx-ledger/` 放到 `~/.workbuddy/skills/` 下，或从仓库分发。

---

## 9. 必须先修的地基

在放开转账之前，有一个**真实的不一致**要补：

`db/transactions/quick_add.rs::derive_postings` 的 transfer 分支**不校验端点账户类型**：

```rust
"transfer" => {
    let debit_account  = input.destination_account_id.unwrap();
    let credit_account = input.source_account_id.unwrap();
    if debit_account == credit_account { return Err(...transferToSelf); }
    Ok(vec![...])            // ← 没有 validate_endpoint
}
```

而 `db/accounts/transfer.rs::create_transfer` **有** `validate_endpoint`
（端点必须是 asset 或 liability，否则报错 —— 因为"从一个支出账户转出"会凭空造钱）。

也就是说：**两条转账路径的校验强度不一样**。今天 AI 走不到 quick_add 的 transfer 分支，
所以一直没暴露；一旦按上面放开 AI 转账，这条路就通了。

**修法**：把 `validate_endpoint` 提到公共位置，两条路径共用；
并统一走 quick_add 那条（它支持日期，`create_transfer` 硬编码了 `today_date()`）。

另外，能力层的 `AccountRef::Name` 解析失败必须**报错**，
不能沿用 `resolve_source_account` 的静默兜底。

---

## 10. 实施计划

### 10.1 依赖关系

```
P0 地基 ──→ P1 能力层 ──┬──→ P2 内部 AI ──┐
                        │                ├──→ P4 审计幂等 ──→ P5 Skill
                        └──→ P3 MCP ──────┘
```

P1 是所有后续工作的地基；P2 与 P3 可并行（两者共用 P1 的能力层，
以及 P2 产出的"提案确认卡"组件）。

### 10.2 P0 地基（约 1~2 天，不改变任何现有产品行为）

| # | 任务 | 落点 | 验收 |
| --- | --- | --- | --- |
| P0-1 | 把 `validate_endpoint` 从 `db/accounts/transfer.rs` 提到公共位置 | 新建 `db/accounts/validate.rs` | 两条转账路径共用同一份端点校验 |
| P0-2 | `derive_postings` 的 transfer 分支调用它 | `db/transactions/quick_add.rs` | 用 expense 账户做转账端点 → 报错 |
| P0-3 | `QuickAddInput` 增加 `category_name`，db 层按 mode 推导类型后 `resolve_or_create` | `db/transactions/types.rs`、`db/transactions/quick_add.rs` | 按**分类名**写入 → 分类与镜像账户一并建出，且在**同一事务**内 |
| P0-4 | 统一日期语义：转账统一走 quick_add 路径 | `db/accounts/transfer.rs` 保留但不再作为 AI/能力层的入口 | 两条路径的日期处理一致 |

**新增回归测试（3 条）**：expense 账户做转账端点被拒；转账两端相同被拒；
按分类名写入后分类被创建且 `accounts` 表多出一条镜像账户。

### 10.3 P1 账本能力层（约 3~4 天）

新目录 `src-tauri/src/ledger/`：

| 文件 | 内容 |
| --- | --- |
| `types.rs` | `RecordKind` / `AccountRef` / `RecordRequest` / `LedgerOutcome` / `AccountView` / `CategoryView` |
| `read.rs` | `list_accounts` / `list_categories` / `account_balances` / `query_transactions` / `period_summary` |
| `write.rs` | `record_transaction` / `create_account` |
| `resolve.rs` | 名称 → id 解析。**解析不到就报错**，并把"按名称模糊匹配到哪个账户"记进 `warnings` |
| `mod.rs` | 门面 re-export |

纪律（三条，评审要盯）：

1. 能力层**不写 SQL**，只调现有 db 层函数（`quick_add_with_category` /
   `create_account_with_path` / `create_opening_balance` / reports）。
2. 能力层是**唯一**允许写账本的地方；`commands/` 不许再直接写库。
3. 写操作**一律返回 `LedgerOutcome`**（含派生好的分录与 warnings），不许返回 `void`。

`AccountRef::Name` 的解析规则（替代现在的静默兜底）：

1. 精确名匹配（忽略大小写）→ 命中。
2. 唯一包含匹配 → 命中，但记 warning「账户「钱」按名称匹配到「微信钱包」」。
3. 多个包含匹配 → **报错**，让调用方给 id。
4. 无匹配 → **报错**。绝不静默回落默认账户。

### 10.4 P2 内部 AI（约 4~5 天）

**后端**

| # | 任务 | 落点 |
| --- | --- | --- |
| P2-1 | prompt 从固定 6 字段改成**动作判别式**：`action ∈ {expense, income, transfer, create_account}` | `services/ai.rs::build_system_prompt` |
| P2-2 | `AiParseResult` 换成 `AiProposal`，新增 `action` / `account_path` / `account_kind` / `opening_balance` | `services/ai.rs` |
| P2-3 | `ai_parse_transaction` **改为只提案、不落库**；返回 `AiProposalDto`（含解析后的 id、名称、warnings） | `commands/ai.rs` |
| P2-4 | 建账户提案：解析路径 → 校验根类型（asset/liability）→ 与现有账户查重 | `commands/ai.rs` |

**关键设计**：提案之后**不再需要新的写命令** —— 用户确认后走现有的
`quick_add_transaction`（带上 P0-3 新增的 `categoryName`）即可。
建账户走现有的 `account_create`。**AI 只负责提案，落库仍走既有命令路径。**

**前端**

| # | 任务 | 落点 |
| --- | --- | --- |
| P2-5 | `QuickAddDialog` 增加 `variant: "create" \| "review"` 与 `initial` 预填。**注意两个 reset effect 在 review 模式不能清空**（`[mode, open]` 会清 category 与对侧账户） | `components/transactions/QuickAddDialog.tsx` |
| P2-6 | 抽 `components/transactions/ProposalReview.tsx`：展示金额/账户/分类/日期/派生分录 + warnings | 新建 |
| P2-7 | `AiInputDialog`：识别成功后**不关窗**，改为转入确认卡 | `components/transactions/AiInputDialog.tsx` |
| P2-8 | i18n 文案（zh/en）+ `errors` 段新键 | `src/i18n/locales/*.json` |

**验收**：说「把支付宝转到微信 500」→ 弹出确认卡显示两端账户与金额 →
改个分类 → 确认 → 交易出现在列表且分录平衡。

### 10.5 P3 MCP 服务（约 5~7 天，风险最高的一期）

**依赖**（版本与 feature 名已核实）

```toml
rmcp  = { version = "3.10", features = ["server", "macros",
          "transport-streamable-http-server", "elicitation"] }
axum  = "0.8"
tokio = { version = "1", features = ["rt-multi-thread", "net", "macros", "sync"] }
```

- `rmcp` 最新 **3.10.0**，要求 Rust ≥ 1.88（项目 1.99.1 ✓）。
- `tokio` 从 `dev-dependencies` 移到 `dependencies`（lock 里已有 1.52.1，不会重复引入版本）。
- `axum` 是新依赖；`rmcp` 本身只依赖 `tower-service`，不绑 axum。

**模块**

| 文件 | 内容 |
| --- | --- |
| `src-tauri/src/mcp/mod.rs` | server 装配、工具注册、路由 |
| `src-tauri/src/mcp/tools.rs` | 9 个工具（6 读 2 写 + `ledger_status`） |
| `src-tauri/src/mcp/approval.rs` | 确认代理（单槽 + oneshot + 超时） |
| `src-tauri/src/mcp/settings.rs` | 端口 / 令牌 / 信任档位 |

**接入点**：`lib.rs` 加 `.setup(|app| { ... })` 钩子。
`app.handle()` 拿到 `AppHandle`，工具内部用 `app.state::<AppState>()` 取库——
**`AppState` 一个字都不用改**（现有 18 处测试构造点零改动）。
⚠️ 不要为此把 `AppState.database` 改成 `Arc<Mutex<...>>`：那会波及 18 处构造点，收益却只有「少 clone 一次 AppHandle」。

**安全基线**：绑 `127.0.0.1`；`Authorization: Bearer <token>`（token 存加密的
`settings` 表，跨重启稳定）；只暴露白名单工具；写操作默认走确认。

**验收**：MCP Inspector 直连

```bash
npx @modelcontextprotocol/inspector --cli http://127.0.0.1:17823/mcp \
  --method tools/list
```

### 10.6 P4 审计与幂等（约 2 天）

新增 `automation_log` 表（v3 未发版，直接进 `schema_v3.sql`；若 v3 已冻结则走 `schema_v4`），
唯一索引锁 `idempotency_key`。同一张表解决幂等、审计、信任档位的证据三件事。

**测试**：同一个 `idempotency_key` 调两次 → 只写一笔，第二次返回首次的结果。

### 10.7 P5 Skill（约 1 天）

`skills/glimmerx-ledger/SKILL.md`：记账约定、工具选择、红线、错误处理。
安装到 `~/.workbuddy/skills/`。

> 注：`rmcp` 3.10 带一个 `skills-fs` feature，MCP 生态本身也在长"技能"这套机制。
> P5 开工前先确认要不要直接对齐 MCP 的 skills 约定，而不是只做 WorkBuddy 私有目录。

### 10.8 风险与回退

| 风险 | 影响 | 缓解 | 回退 |
| --- | --- | --- | --- |
| `rmcp` 3.x 破坏性变更 | 编译不过 | 关在 `mcp/` 模块内，锁 `3.10`；不改 `commands/` 与 `db/` | 降到 2.x，或手写 JSON-RPC（工具面小，可控） |
| axum/tokio 与 Tauri 内置 runtime 冲突 | 运行时异常 | 用 `tauri::async_runtime::spawn`，不自建 runtime | 换 `tiny_http` 自己实现 Streamable HTTP 传输 |
| 本地端口暴露给同机进程 | 越权记账 | 127.0.0.1 + 随机 token + 工具白名单 + 默认只读 | 设置页一键关停 |
| agent 乱写账本 | 数据污染 | 默认只读；确认档弹卡；`warnings` 把"我替你猜了"摊开 | 信任档默认关 |
| agent 重试 | 重复记账 | `idempotency_key` 唯一索引 | — |
| 提案里的分类名污染分类树 | 分类树长歪 | 建分类也要确认；提案前先做名称查重 | 「允许 AI 自动建分类」开关默认关 |

### 10.9 里程碑与验证矩阵

| 期 | 主要验证手段 | 通过标准 |
| --- | --- | --- |
| P0 | `cargo test` | 3 条新回归测试通过，且既有 414 条全绿 |
| P1 | 能力层单测（`crate::db::init_schema` 夹具） | 读 5 + 写 2 全部有测试；名称解析的 4 条分支都有测试 |
| P2 | `vitest` + 真 AI 调用手工验 | 三种模式 + 建账户都能出确认卡；确认后交易正确落库 |
| P3 | MCP Inspector `tools/list` + `tools/call` | 锁屏时报「账本已锁定」；只读档下写工具被拒；确认档下能写 |
| P4 | Rust 测试 | 同一幂等键调两次只写一笔 |
| P5 | WorkBuddy 实连 | agent 能查账、能记一笔（经确认） |

每一期结束时跑 `make check`（tsc / eslint / prettier / cargo fmt / clippy）
与 `cargo test`，覆盖率门禁维持 94%。

---

## 11. 明确不做的事

- **不做多用户 / 服务端 / 账号体系**。local-first 加密单用户账本是产品前提，不是待办。
- **不做多币种**（见 01）。
- **不把 MCP 暴露到局域网**。只绑 `127.0.0.1` + Bearer token。
- **不让 agent 直接执行 SQL**。能力层是唯一入口。
- **不给 agent 原始数据库快照**。工具返回的是聚合过的视图，不是 `SELECT *`。
- **不在内部 AI 上引入 function-calling 循环**。动作封闭且已知，固定 schema 更可控。

---

## 12. 关联

- `docs/design/21-double-entry-upgrade.md` —— 复式记账与迁移
- `docs/design/24-reports.md` —— 报表与区间口径（`summary` 工具复用其实现）
- `docs/reviews/CODE_REVIEW.md` 审查 #1 —— AI 记账链路的原始评审（两个 Critical 已修）
