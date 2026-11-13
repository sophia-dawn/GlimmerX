# Code Review 记录

> 五轴审查：正确性 / 可读性 / 架构 / 安全 / 性能。
> 严重级别：`Critical` 阻断合并 · 无前缀为 Required（合并前必须处理）· `Optional` 建议 · `FYI` 仅供参考。

---

## 审查 #1 — AI 记账链路与交易写入路径

| 项目 | 内容 |
|---|---|
| 日期 | 2026-11-12 |
| 代码状态 | `main` @ `a3042c9`，工作区无代码改动（仅 `.workbuddy/` 记忆文件） |
| 审查范围 | 最近合入的 AI 记账功能及相邻数据边界 |
| 涉及文件 | `src-tauri/src/commands/ai.rs`、`services/ai.rs`、`commands/transactions.rs`、`db/transactions.rs`、`commands/data.rs`、`db/import.rs` |
| 结论 | **Request changes** — 2 个 Critical 阻断合并 |

### 背景与意图

该变更引入自然语言记账：Rust 后端读取本地设置中的 AI 服务配置，构造注入当日日期 / 分类清单 / 账户清单的 prompt，调用 OpenAI 兼容 `/chat/completions`，解析 JSON 后校验金额与日期，匹配或自动创建分类、推断账户，最终复用 `quick_add_transaction` 生成平衡双分录并写入数据库。设计上不经过用户确认，直接落库。

审查重点因此落在：外部模型输出作为不可信输入的边界校验、写库的原子性、密钥处理。

### 测试评估（先审测试）

做得好的部分：`services/ai.rs` 用本地 TCP mock server 覆盖 HTTP 状态码、JSON 解析降级、超时、空 choices；`commands/ai.rs` 覆盖账户名匹配优先级与兜底逻辑。是行为测试，不是实现细节测试。

覆盖缺口（两个 Critical 因此不可见）：

- `db/transactions.rs` 中约 20 个 `quick_add_transaction` 测试**全部直传 `&Connection`**，无一条走事务路径，也无一条模拟"第二条分录插入失败"。
- `validate_amount` 覆盖 `0` / 负数 / 非数字，缺 `inf`、`NaN`、`1e309` 等边界；全仓无 `i64::MAX` / f64 溢出相关测试。
- `resolve_source_account` 的 4 个测试没有空字符串 hint 用例。

### 发现

#### Critical 1 — 两个主记账入口没有事务，失败会留下不平衡的孤儿交易

`transaction_create`（`commands/transactions.rs:265`）正确地 `conn.transaction()` → 写入 → `tx.commit()`。但用户最常用的两个入口没有：

- `commands/transactions.rs:359` `quick_add_transaction` — 直接 `db::transactions::quick_add_transaction(&conn, ...)`
- `commands/ai.rs:214` `ai_parse_transaction` — 同样直接调用

db 层 `create_transaction`（`db/transactions.rs:564`）在 autocommit 下先 `INSERT transactions`，再循环 `INSERT postings`。任一分录写入失败（外键约束、磁盘满、进程被杀）会留下一条**分录数为 1、金额不平衡**的交易，且 `deleted_at IS NULL`，余额聚合、报表、审计全部将其视为有效数据。

AI 路径更严重，一次调用存在多次独立提交：

1. `resolve_category`（`commands/ai.rs:16`）自动创建分类 — `INSERT categories`
2. `ensure_category_account`（`db/transactions.rs:430`）自动创建同名科目账户 — `INSERT accounts`
3. `create_transaction` — `INSERT transactions`
4. 循环 — `INSERT postings` × 2

任何一步失败都会留下部分数据。这违反项目「每笔交易的 postings 必须合计为零」硬规则，而 schema 中没有触发器或约束能够兜底。

**修复建议（结构性）**：不要在两个 command 里各补一遍 `let tx = ...`（下一个调用方还会忘）。把事务边界下沉到 `db::transactions::quick_add_transaction` 内部——它本身已是完整业务用例（校验 → 派生分录 → 落库 → 读回），用 `conn.unchecked_transaction()` 可保持现有 `&Connection` 签名，末尾 `tx.commit()`。AI 的 `resolve_category` 一并纳入同一事务，或在 AI 命令开外层事务并把两条路径都改用 `&tx`。

**回归测试**：制造第二条分录写入失败（让第二个账户失效或加临时 trigger），断言失败后 `transactions` 表行数不变、`SUM(postings.amount)` 仍为零。

#### Critical 2 — 不可信 LLM 输出可静默写入 `i64::MAX` 金额

`services/ai.rs:51` `validate_amount` 仅判断 `v > 0.0`，`"inf"`、`"1e309"` 均可通过；`db/transactions.rs:331` `parse_amount_to_cents` 为：

```rust
let value = amount_str.parse::<f64>()?;
let cents = (value * 100.0).round() as i64;
```

Rust 的 `as i64` 为饱和转换。本次审查用 `rustc` 最小复现实测：

```
"1e309".parse::<f64>() * 100.0 as i64 == 9223372036854775807
```

随后 `derive_postings` 生成 `[+i64::MAX, -i64::MAX]`，`sum == 0` 平衡检查通过，`amount_cents <= 0` 检查也通过，于是**一笔天文数字交易静默入账，且因双分录平衡，审计报表无法察觉**。影响面不限于 AI：用户在表单手输 `1e309` 走同一函数。项目「金额存整数分、从不用浮点」的硬规则在此被绕过。

**修复建议**：`parse_amount_to_cents` 改为字符串解析（拆分小数点、只取两位）或引入 `rust_decimal`，彻底不经过 f64；最低限度须先 `is_finite()` 并设金额上界。`validate_amount` 同步加 `is_finite()` 与上界。

**回归测试**：`"inf"` / `"1e309"` / `"NaN"` / `"9e18"` / `"0.001"` 全部断言被拒。

#### Required — 空 accountHint 会静默选错账户

`resolve_source_account`（`commands/ai.rs:49`）未对 hint 做 `trim` 和判空。当模型返回 `"accountHint": ""` 时，`hint_lower == ""`，而 `cat_lower.contains("")` 恒为 `true`，函数直接返回按名字长度降序排列后的第一个账户——静默记入错误账户。先 `trim()` 并判空后跳到兜底分支；补空字符串测试用例。

#### Required — CSV 导入不校验日期格式

`db/import.rs:95-135` 反序列化后直接使用 `&first_row.date` 入库，无任何格式校验。任意字符串（如 `9999-99-99`）会污染所有按日期排序与区间过滤的报表。建议在反序列化后用 `NaiveDate::parse_from_str(..., "%Y-%m-%d")` 校验；抽出 `utils/validation::validate_business_date` 与 AI 的 `validate_date` 共用格式部分（注意 AI 那条带「不允许未来日期」的业务约束，不可整体复用）。

#### Required — 错误契约混合的具体实例

- `commands/data.rs:38`：`format!("errors.backupFailed: {}", e)`
- `db/import.rs:132`：`format!("errors.transactionUnbalanced: sum={}", sum)`

前端 `utils/errorTranslation.ts` 检测 `startsWith("errors.")` 后会去 i18n 查一个不存在的 key，用户最终看到 `"errors.backupFailed: 拒绝访问 (os error 5)"` 这类半翻译文本。与 `AGENTS.md` 中「后端出友好文案、前端原样展示」的硬规则冲突，需按上一轮分析中的方案统一错误契约。

#### Optional — 其它

- `AiConfig`（`services/ai.rs:9`）派生 `Debug`，`api_key` 明文在结构体中；已核实全仓目前无打印点，属潜在风险，建议手写 `Debug` 打码。
- `base_url` 无 scheme 校验，用户填 `http://` 时 Bearer Key 明文出网。
- `services/ai.rs:363` `test_connection` 用 `eprintln!` 打印上游响应体，未经日志框架。
- `db/import.rs` 将 CSV 全部 `rows` 一次性载入内存，无条数上限；`sum()` 对 `i64` 在极端输入下可能 wrapping（修 Critical 2 后自然缓解）。

#### FYI

`create_transaction` 的 `postings.iter().map(|p| p.amount).sum()` 在 release 下为 wrapping 加法，但仅在金额已失控时可达。`ai_parse_transaction` 的 Phase 1/2/3 注释清楚说明「网络 await 前释放 DB 锁」，这个处理是正确的，后续改动应保持。

### 五轴结论

| 轴 | 结论 | 说明 |
|---|---|---|
| 正确性 | **不通过** | 两条静默账目损坏路径，均未被现有测试覆盖 |
| 可读性 | 通过 | 分相注释清楚，命名一致，无死代码 |
| 架构 | 需改 | 事务责任散落在 IPC 层，db 层业务用例不自洽，与同仓 `transaction_create` 模式不一致 |
| 安全 | 部分 | SQL 全参数化、CSP 严格、API Key 不经 webview 均正确；外部数据（LLM 输出 / CSV）边界校验不足 |
| 性能 | 通过 | 网络请求前主动释放 `MutexGuard`，无长锁；列表分页已做 |

### 验证情况（如实记录）

| 检查 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过 |
| `npx eslint . --max-warnings=0` | 通过 |
| `npx vitest run` | 通过，8 文件 / 72 用例 |
| `cargo test` | **未运行** — 用户明确要求跳过（慢） |
| `cargo clippy` / `cargo fmt` | 未运行 |
| Critical 2 的饱和转换 | 已用 `rustc` 最小程序实测确认 |

因此本次 Critical 结论基于静态分析 + 针对性最小复现，**尚未经过完整后端测试套件验证**。修复后需补跑 `cargo test` 与 `make check`。

### 处理清单

- [x] Critical 1：事务下沉至 db 层 + 失败路径回归测试
- [x] Critical 2：金额解析去 f64 + 边界用例
- [x] Required：空 hint 判空 + 测试
- [x] Required：CSV 日期校验
- [x] Required：错误契约统一
- [x] 修复后跑等价的后端/前端全套检查（见文末记录）

---

## 附：上一轮项目全量分析的未决问题

2026-11-12 另一次项目分析发现、但不属于本次变更范围的项目级问题，详见**同目录** `2026-11-12.md`（当日工作日志快照，含项目全量分析与审查摘要）：

- **P0** 无数据库迁移机制（`db/schema.rs` 仅 `CREATE TABLE IF NOT EXISTS`，无 `user_version`），已发布 0.1.2 安装包，后续加字段老库不会迁移。
- **P1** 错误契约自相矛盾（97 处后端返回 i18n key，前端 36 处再翻译）。
- **P1** 前端无覆盖率门槛（134 源文件 / 8 测试文件，页面组件基本零测试）。
- **P2** 单文件过大、Makefile 三处不一致、全局单连接串行化。

---

## 审查 #2 — 技术架构评估与生产化缺口

| 项目 | 内容 |
|---|---|
| 日期 | 2026-11-12 |
| 代码状态 | `main`，工作区无源码改动 |
| 审查范围 | 全栈技术选型是否与 2026 年业界主流一致、是否存在迁移必要；配置与文档一致性 |
| 涉及文件 | `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/src/lib.rs`、`src-tauri/src/db/schema.rs`、`src/utils/api.ts`、`vite.config.ts`、`docs/design/01-overview.md`、`docs/design/08-constraints.md`、`docs/design/11-risks.md` |
| 结论 | **不需要框架级迁移**；应做一次「生产化补齐」迭代 |

### 背景与意图

用户提问「当前技术架构是否业界常用、有无迁移必要」。本轮为选型层与发布链审查，不修改任何源码或配置。重点是区分「选型问题」与「实现 / 运维缺口」，避免把正确性缺陷误判为架构问题、进而触发无收益的重写。

### 架构对标

| 层 | 当前选型 | 对标结论 |
|---|---|---|
| 桌面壳 | Tauri 2 + Rust + 系统 WebView | 主流。2026 年轻量桌面应用的可信默认；Electron 仍是生态最成熟、渲染最可控的保守选择，但已非唯一答案 |
| 前端 | React 19 + TS strict + Vite 7 + Tailwind 4 + shadcn/ui + Radix | 主流，逐项吻合 2026 React SPA 的默认配方 |
| 状态管理 | Zustand（客户端）+ TanStack Query v5（服务端） | 主流组合，Redux 在中轻量项目已退居存量 |
| 数据层 | SQLCipher 加密 SQLite + rusqlite 手写参数化 SQL + WAL | 主流。SQLCipher 是开源整库加密最稳妥选择；本地优先同类产品（Actual Budget / Joplin / Trilium）均为本地 SQLite |
| IPC 契约 | 63 个 `#[tauri::command]` + 手写 TS 类型镜像 | 常见但偏手工，唯一明确的「可升级项」 |
| 工程化 | `make check` / clippy `-D warnings` / pre-commit / CI 三 job / 覆盖率门槛 | 标准，偏严 |

生态信号（第三方 2026 年中统计，非官方口径，仅供量级参考）：Electron npm 周下载约 4.0M、GitHub 约 121K stars；Tauri 的 `@tauri-apps/api` 约 1.3M 周下载、约 107K stars。差距仍在，但 Tauri 已有生产级采用（1Password 8、DbGate 等）。

**做得好、后续改动应保持**：金额整数分、余额不落库靠 `SUM(postings.amount)` 实时聚合、双分录强制平衡；所有计算在 Rust 后端、前端仅展示；SQL 全参数化、CSP 严格、API Key 仅存 Rust 侧且 AI 请求走 reqwest 不经 webview；本地加密 + 用户口令满足 local-first 的「公司消失数据仍可读」标准。

### 迁移决策

结论：**不迁移**。换 Electron 会带来包体从十几 MB 涨到 100MB+ 的回退、丢掉 Rust 侧的加密与计算边界、重写 63 个 command，换来的只是渲染一致性与更成熟的打包生态，对本项目定位是净亏损；Vite 7 / React 19 / TS 5.8 / Tailwind 4 均已在当期主线上，跟 minor 升级即可，不构成迁移。

唯一会改变判断的三种情况：

1. 明确要做多端 / 云同步 —— 需要新增同步层（CRDT + HLC + Merkle，参考 Actual Budget），属于功能演进，不是换技术栈；且 `01-overview.md` 已将云同步、移动端列为 Out of Scope，当前不触发。
2. 强需求像素级一致渲染或大量 Node 原生依赖 —— 才重新评估 Electron。
3. 团队失去 Rust 维护能力 —— 属人力问题，应通过补人 / 简化 native 层解决，而非重写。

### 发现

#### Required — 已发版但无自动更新通道

全仓检索 `updater` / `tauri_plugin_updater` 无匹配：`Cargo.toml` 无 `tauri-plugin-updater`，`tauri.conf.json` 无 `plugins.updater`，capabilities 亦无相关权限。v0.1.2 已产出 NSIS / MSI 安装包，但用户只能手动下载新版本。这不只是便利性缺口：审查 #1 的两个 Critical 修好后，若无更新通道，存量用户不会升级，等于没修。

建议：接入 `tauri-plugin-updater` + 签名公钥，CI 产出更新清单；与 P0 修复同批发布。

#### Required — WebView2 安装模式 skip 与风险文档矛盾

`tauri.conf.json:34-38` 配置 `webviewInstallMode: { "type": "skip" }`。Tauri v2 官方 Windows Installer 文档将该模式标注为不推荐，并明确「用户未装运行时时应用不会工作，也不会尝试安装」。而 `docs/design/11-risks.md` 的应对措施写的是「安装包自动检测并安装」，二者不一致。

Win10（2018-04 后）/ Win11 随系统分发 WebView2，但精简镜像、LTSC、企业受管设备、离线环境不保证存在；缺失时症状具有欺骗性 —— 安装成功、双击白屏、无任何报错。

建议：改回默认 `downloadBootstrapper`，或 `embedBootstrapper`（约 +1.8MB，对 Win7 / .msi 更友好）；`make portable-windows` 产出的单文件 exe 没有安装器兜底，需在启动前用 `tauri::webview_version()` 探测并给出可读提示。

#### Required — 数据库缺版本化迁移（沿用 P0，本轮复核）

`db/schema.rs` 仅有 `CREATE TABLE IF NOT EXISTS`，全仓无 `PRAGMA user_version` / 迁移数组；`ensure_transaction_hashes_table()` 是补丁式补救，反而印证机制缺失。已发布 0.1.2，后续任何加列 / 改约束都会让老库失配。

建议：`PRAGMA user_version` + 顺序迁移脚本（`rusqlite_migration` 或自维护数组），执行前先备份。

注意：`AGENTS.md` 硬规则要求任何数据库 schema / 表结构 / 迁移脚本修改必须先经用户确认，本轮仅记录、不实施。

#### Required — 错误契约与前端测试门槛（沿用 P1）

后端 `AppError` 约 97 处返回 i18n key，前端 `utils/errorTranslation.ts` 再翻译（36 处调用），另有英文句子与 rusqlite 原始错误混出，与「后端出友好文案、前端原样展示」的硬规则冲突；`vitest` 无 coverage 配置，134 个源文件中仅 8 个测试文件，页面组件基本零测试。

#### Optional — IPC 契约为手写双份类型

`lib.rs` 注册 63 个 command，`src/utils/api.ts` 与 `src/types/` 手工镜像 Rust 结构体，命令名是字符串字面量。字段改名在编译期不报错，只在运行时出现 `undefined`。业界已有 `tauri-specta` 从 Rust 生成 `bindings.ts` 并在 CI 做 drift 检查的成熟做法。属一次性局部升级（加代码生成层），不是迁移。

#### Optional — 非主流但可辩护的三处

- React Query 统一 `staleTime:0 / gcTime:0 / refetchOnMount:"always"`，等于把缓存库当无缓存 RPC 层用。本地 IPC 场景下是一致性优先的合理选择，代价是页面切换重复查询（已有列表性能优化记录）。允许短 `staleTime` + 精确 invalidate 可减少往返，但需同步调整 `AGENTS.md` 硬规则。
- `AppState.database: Mutex<Option<Database>>` 全局单连接，WAL 已开但未利用读并发，导出 / 报表长查询会阻塞录入。可考虑读写连接池，个人单机规模下非紧急。
- 大文件：`db/transactions.rs` 2912 行、`db/reports.rs` 2221、`db/accounts.rs` 1775。

#### FYI — 文档与配置漂移

- `docs/design/01-overview.md` 列的 TanStack Table、`react-number-format` 未出现在 `package.json`（实际使用 `@tanstack/react-virtual` 与自研 `utils/format.ts`）。
- `docs/design/08-constraints.md` 写 accounts 23 列、`subtype` 多态字段、`(name, COALESCE(parent_id,''))` 唯一索引，实际 schema 约 13 列且无 `subtype` / `parent_id`；同文件「前端用 react-number-format」亦失效。
- `package.json` 的 `name` 仍为 `tauri-app`；`markdownlint-cli` 误放 dependencies；`i18next` 版本 `^26.0.4` 等依赖较激进，升级时需回归。
- Makefile：`fmt-md` 的 `-name "*.md "` 多了空格永不匹配；`check` 中 `prettier --check --write` 既检查又写入，与 CI 的纯 `--check` 行为不一致；`AGENTS.md` 提到的 `make build` / `make bundle` 目标不存在。

### 五轴结论

| 轴 | 结论 | 说明 |
|---|---|---|
| 正确性 | 不通过 | 审查 #1 的两个 Critical 尚未修复，本轮不重复展开 |
| 可读性 | 通过 | 分层与命名清晰；文档与实现存在漂移 |
| 架构 | 通过 | 逐层对标 2026 主流，无迁移必要；缺生成式 IPC 契约 |
| 安全 | 通过 | 加密、CSP、参数化、密钥边界均正确；外部输入校验缺口见审查 #1 |
| 性能 | 部分 | 单连接串行化与零缓存策略有优化空间，个人单机规模下可接受 |

### 验证情况（如实记录）

| 检查 | 结果 |
|---|---|
| 配置文件 / 依赖清单 / 设计文档静态核对 | 已逐项完成 |
| 检索 `user_version`、`migration` | 无迁移机制；仅有 WAL / foreign_keys / key / rekey 等 PRAGMA |
| 检索 `updater`、`specta`、`ts-rs` | 全仓无匹配 |
| Tauri v2 Windows Installer 官方文档 | 已核对 `skip` 模式为不推荐、且不安装运行时 |
| `cargo test` / `make check` | 未运行 —— 用户此前明确要求跳过（慢），本轮亦未改代码 |

本轮为静态审查 + 官方文档核对，未做运行时验证；所有结论不涉及代码改动。

### 处理清单

- [x] Critical 1：事务下沉至 db 层 + 失败路径回归测试（审查 #1）
- [x] Critical 2：金额解析去 f64 + 边界用例（审查 #1）
- [x] Required：schema 版本化 —— 用户确认后实施（见文末修复记录）
- [ ] Required：接入自动更新插件与签名 —— **不实施**（产品决策：不做自动升级，见文末）
- [x] Required：WebView2 安装模式改默认 + 便携版启动探测
- [x] Required：空 hint 判空、CSV 日期校验、错误契约统一（审查 #1）
- [x] Required：补前端覆盖率门槛
- [ ] Optional：评估生成式 IPC 契约 —— 结论：tauri-specta 面向 Tauri 2 仅有 RC，改用 ts-rs 分阶段（未实施）
- [x] Optional：React Query 缓存策略（`staleTime` 保持 0，集中默认值并调整 `gcTime` / 聚焦刷新）
- [ ] Optional：读写连接池 —— 需先决定是否在内存保留数据库密钥（未实施）
- [x] Optional：拆分大文件（已按功能域拆成模块目录，见文末 2026-11-13 记录）
- [x] FYI：修正文档漂移与 Makefile / `package.json` 小问题，并加文档校验脚本
- [x] 修复后跑 `make check` 与 `cargo test`（后端/前端全套，见文末记录）

---

## 修复记录 — 2026-11-13（三项 P0 已实施）

用户确认后按「金额边界 → 事务 → 迁移」顺序落地，迁移框架选用
`rusqlite_migration = "=2.5.0"`（依赖 `rusqlite ^0.39.0`，与本项目一致；
2.6.0 依赖 0.40，会产生两个不兼容的 `Connection` 类型）。

| 问题 | 修复要点 | 主要文件 |
| --- | --- | --- |
| Critical 2 金额 f64 饱和 | 纯字符串/十进制解析（i128 checked、1 亿元上限、拒绝 inf/NaN/科学计数法）；由 db 层 re-export 供标准交易/预算/quick-add 共用；AI 的 `validate_amount` 复用同一函数；平衡校验改 i128 | `src/utils/money.rs`、`db/transactions.rs`、`services/ai.rs` |
| Critical 1 无事务 | 新增 `quick_add_with_category`，用 `unchecked_transaction()` 把分类解析、科目账户创建、交易与分录写入包进单事务；`resolve_category` 下沉为 `db::categories::resolve_or_create`；AI 命令 Phase 3 只调一次 db 层函数 | `db/transactions.rs`、`db/categories.rs`、`commands/ai.rs` |
| P0 无迁移机制 | `schema_v1.sql` 作为唯一基线；`db/migrations.rs` 用 `pending_migrations`/`to_latest` 管理 `user_version`，PRAGMA 在事务外、迁移前整库备份、`current>latest` 拒绝打开；`create`/`open` 都调用；删除补丁式 `ensure_transaction_hashes_table` | `db/migrations.rs`、`db/schema_v1.sql`、`db/schema.rs`、`db/mod.rs`、`db/import.rs` |

**新增回归测试（10 个）**：金额边界（inf/1e309/NaN/9e18/上限/四舍五入）、
第二条分录失败时交易/分录/分类/科目账户全部回滚、迁移基线 validate、
老库迁移数据保留与幂等、拒绝新版库、create/open 的 PRAGMA 与 user_version。

**验证**：`make test-backend` 380 passed / 0 failed（bin 与 doc-tests 均通过）；
`cargo fmt -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo build` 全通过；
正式 exe 与测试 exe 的 Common Controls v6 manifest 仍完好。
前端未改动，因此未跑 `make check` 的前端部分。

**仍未处理**：空 accountHint、CSV 日期校验、错误契约统一、前端覆盖率门槛、
自动更新通道、WebView2 skip 等 P1/P2 项（见上方处理清单）。


---

## 修复记录 — 2026-11-13（P1，按产品决策）

用户拍板：**做国际化**（错误文案保留 zh/en）、**安装包不带运行时**、**覆盖率接受「先测基线 → 防下降门禁 → 逐轮提升」**；
并明确 **不做自动升级**。

| P1 | 修复要点 | 主要文件 |
| --- | --- | --- |
| 错误契约违反硬规则 | 新增 `src/errors.rs` + `src/errors/messages.rs`（由 `tools/gen_error_messages.py` 从 `src/i18n/locales/{zh,en}.json` 的 `errors` 段生成，并调用 rustfmt）；`AppError` 手写 `Display` 统一渲染友好文案、英文内部错误替换为通用文案并写 stderr；新增 `set_ui_language` command + 前端 `setLanguage`/启动同步；`translateErrorMessage` 改为原样展示；补齐 11 个缺失 zh/en 键；命令边界的裸 key 与英文硬编码全部替换 | `src-tauri/src/errors.rs`、`errors/messages.rs`、`db/mod.rs`、`commands/*.rs`、`src/utils/errorTranslation.ts`、`src/i18n/index.ts`、`src/utils/api.ts` |
| 空 accountHint | `resolve_source_account` trim + 判空，空 hint 落到默认账户分支；补 3 种空值 + 「不得返回最长账户」回归断言 | `commands/ai.rs` |
| CSV 日期无校验 | `import_csv` 反序列化后整批前置校验；抽出 `utils::validation::parse_business_date` 供 CSV 与 AI `validate_date` 共用格式部分（AI 的「不允许未来 >1 天」约束保留）；补 6 种非法日期 + 「无写入」断言 | `db/import.rs`、`utils/validation.rs`、`services/ai.rs` |
| WebView2 `skip` | `downloadBootstrapper`（silent，安装包不捆绑运行时）；风险文档同步；`lib.rs` 在 Windows 启动前用 `tauri::webview_version()` 探测，缺失时弹可读提示并退出 | `tauri.conf.json`、`docs/design/11-risks.md`、`src-tauri/src/lib.rs` |
| 前端无覆盖率门槛 | `vitest.config.ts` 配 `coverage`（v8、全量 include、排除测试/类型/locales）；实测基线 7.61% stmts / 7.6% lines / 8.41% funcs / 3.42% branches，设「略低于基线」防下降阈值并写明每轮 +5 点；CI frontend job 改用 `npm run test:coverage` | `vitest.config.ts`、`.github/workflows/ci.yml` |

**关于不做自动升级**：不引入 `tauri-plugin-updater`、不做静默更新。替代触达可用已装的
`tauri-plugin-opener` 做「检查更新」按钮打开发布页（本次未实现，属后续可选项）。
如实风险：P0 修复与数据库迁移只对主动下载新版的用户生效；0.1.2 用户需手动升级，
首次打开新版才会备份并迁移。发布说明需显著提示。

**验证**：`make test-backend` 384 passed / 0 failed；`cargo fmt -- --check`、
`cargo clippy --all-targets -- -D warnings`、`cargo build` 全通过；
`npm run test:coverage` 72 tests 通过且覆盖率门禁生效（All files 7.61/3.42/8.41/7.6）；
`npx tsc --noEmit`、`npx eslint . --max-warnings=0`、
`npx prettier --check "src/**/*.{ts,tsx,css,json}"` 全通过。
三平台 CI 仍需在 push/PR 后实际运行确认。

**仍未处理**：P2 单文件过大、Makefile 三处不一致、全局单连接、IPC 手写双份类型、
React Query 零缓存策略，以及 P3 文档漂移与 `package.json` 小问题。


---

## 修复记录 — 2026-11-13（P2 / P3 第一批）

用户确认「按建议实施」后落地低风险项；三项大改（拆文件、IPC 类型生成、连接池）按下述原因延后。

| 条目 | 修复要点 | 主要文件 |
| --- | --- | --- |
| P2 安全：密钥可打印 | `AiConfig` 改手写 `Debug`，`api_key` 固定输出 `***` | `services/ai.rs` |
| P2 安全：明文出网 | `ai.base_url` 写入与读取双重校验：仅 `https://`，`http://` 只放行 loopback；新增 `errors.ai.insecureBaseUrl` 双语文案 | `commands/settings.rs`、`commands/ai.rs`、`services/ai.rs`、`locales/*.json` |
| P2 安全：日志泄数据 | 上游响应体日志截断到 200 字符（`log_body`），三处调用统一 | `services/ai.rs` |
| P2 安全：CSV 内存 | 导入行数上限 200000（`take(MAX+1)`）、`i128` 累加平衡校验、`ImportError.message` 统一走 `errors::message` | `db/import.rs` |
| P2 Makefile | `fmt-md` 的 `-name "*.md "` 修正；`check` 去掉 `prettier --write`；新增 `build` / `bundle` / `lint-md`；`MD_FILES` 变量去重 | `Makefile` |
| P2 React Query | 全局默认值集中到 `App.tsx`（`staleTime: 0`、`gcTime: 5min`、`refetchOnWindowFocus: false`），删除 23 个文件里的重复覆盖；AGENTS.md 与 src/AGENTS.md 措辞同步为「不得返回陈旧数据」 | `src/App.tsx`、`src/constants/query.ts`、23 个 hook/页面、`AGENTS.md`、`src/AGENTS.md` |
| P3 包元数据 | `name` 改 `glimmerx`；`markdownlint-cli` 移入 devDependencies；锁文件同步 | `package.json`、`package-lock.json` |
| P3 文档漂移 | 01-overview / 08-constraints / 02-roadmap / 11-risks 共 6 处改为实际实现（12 列、无 `subtype`/`parent_id`、`@tanstack/react-virtual`、自研 `utils/format.ts`） | `docs/design/*.md` |
| P3 防复发 | 新增 `tools/check_docs.py`：校验技术栈依赖存在于 package.json、accounts 列数与 `schema_v1.sql` 一致、不得描述不存在的列；CI frontend job 增加该步骤 | `tools/check_docs.py`、`.github/workflows/ci.yml` |

**验证**：`make test-backend` 390 passed / 0 failed（新增 6 个测试：密钥脱敏、日志截断、
base_url 允许/拒绝、set_setting 拒绝明文远端、CSV 行数边界）；`cargo fmt -- --check`、
`cargo clippy --all-targets -- -D warnings`、`cargo build` 通过；
`npm run test:coverage` 72 tests 通过且门禁不变（7.61/3.42/8.41/7.6）；
`npx tsc --noEmit`、`npx eslint . --max-warnings=0`、`prettier --check` 通过；
`python tools/check_docs.py` 通过。

### 未实施项及原因

- ~~**拆分大文件**~~：已于 2026-11-13 完成（纯移动，无逻辑改动），见下节记录。
- **IPC 契约生成**：tauri-specta 面向 Tauri 2 仍为 `2.0.0-rc.28`，不宜引入预发布依赖；`ts-rs` 路线需覆盖 75 个前端类型 / 95 个 Rust 结构，属独立工程，建议先只生成 DTO 再切 import。
- **读写连接池**：SQLCipher 下新连接必须重新 `PRAGMA key`，池化意味着在内存中长期保留密钥（或每次重问口令）。这是安全取舍，需产品确认后再做；当前 `staleTime: 0` + WAL 已缓解大部分读放大。
- **报表聚合 i128**：单笔上限 1 亿元时需约 9.2 亿行才可能溢出 `i64`，暂不改，避免无收益的类型扩散。

---

## 修复记录 — 2026-11-13（拆分大文件）

`db/transactions.rs`（3000 行）、`db/reports.rs`（2221 行）、`db/accounts.rs`（1775 行）
按功能域拆成模块目录，**全部为纯移动**：函数体、SQL、注释逐字保留，仅新增 `mod` 声明与
`pub use` 再导出，调用方（`commands/*`）无需任何改动。跨模块私有函数的可见性收紧到
`pub(in crate::db::xxx)`，不扩大 crate 内部 API 面。

| 模块 | 拆分结果 | 最大文件 |
| --- | --- | --- |
| `db/reports/` | `types` / `common` / `standard` / `comparison` / `trend` / `account` / `audit` / `tests` | 359 行（`trend.rs`） |
| `db/accounts/` | `types` / `meta` / `path` / `crud` / `create` / `opening` / `transfer` / `ledger` / `tests` | 315 行（`crud.rs`） |
| `db/transactions/` | `types` / `validate` / `quick_add` / `create` / `query` / `update` / `delete` / `paginate` + `tests/`（6 个分域测试模块） | 272 行（`paginate.rs`） |

三个 `mod.rs` 之外的门面文件（`db/reports.rs` 等）只保留模块声明与再导出（各 ≤ 36 行），
`crate::db::reports::*`、`crate::db::accounts::*`、`crate::db::transactions::*` 的公共路径不变。

**验证**：`cargo test` **390 passed / 0 failed**（数量与拆分前一致，未丢失用例）；
`cargo fmt -- --check`、`cargo clippy --all-targets -- -D warnings`、`make check` 全部通过。

**遗留**：`db/accounts/tests.rs`（836 行）与 `db/reports/tests.rs`（809 行）尚未按域细分，
不影响主代码可读性，必要时可按 `db/transactions/tests/` 的方式再拆。

---

## 修复记录 — 2026-11-13（新发现 8 项）

上一轮的修复记录挂账项之外，本轮复查代码又发现 8 个问题，全部已修。

| # | 问题 | 根因 | 修复 |
| --- | --- | --- | --- |
| 1 | 错误文案被「通用数据库错误」覆盖 | `errors::user_text` 用「纯 ASCII 即内部错误」的启发式兜底，`Invalid date` / `Invalid year/month` / `Database file not found` 全被替换成「数据库操作失败」 | 给这些位置补专用键（`invalidDateValue` / `invalidYearMonth` / `databaseFileNotFound` / `databaseOpenFailed` / `databaseCreateFailed` / `backupSamePath` / `unknownError`），详情写 stderr；`user_text` 只作为最后兜底 |
| 2 | 缺键时把内部 key 直接吐给 UI | `render` 找不到键时 `return input`，前端会显示 `errors.xxx` | 缺键 → 记 stderr + 返回 `errors.unknownError`；生成脚本新增「Rust 引用的键必须在 JSON 中存在」校验与 `--check` 模式（CI + `make check` 都跑） |
| 3 | 迁移基线可被静默改写 | `schema_v1.sql` 只执行一次，若日后往里加列，新库有列、老库无列 | 文件头标注 FROZEN；新增 `test_schema_v1_is_frozen`（sha256 冻结，CRLF 归一化），改动即失败 |
| 4 | 两套建库路径 | `init_schema` / `init_baseline` 不写 `user_version`，与真实启动路径并存 | 两者加 `#[cfg(test)]`，生产代码只能走 `Database::create`/`open` → `migrate`；去掉失效的 `allow(dead_code)` |
| 5 | 95% 覆盖率门禁从未本地验证 | `cargo-llvm-cov` 未安装，且 CI 参数与 `Makefile` 不一致（CI 少 `--ignore-filename-regex`） | CI 与 `make coverage` 参数对齐；本地实测见下方「验证」 |
| 6 | 备份可覆盖正在使用的库文件 | 目标路径与库文件相同时，先 `remove_file` 再 `rename`，Unix 上会把连接指向已删除的 inode | 备份逻辑抽成 `backup_database`，同文件（canonicalize 比较）直接拒绝；失败清理 `*.tmp-backup`；补 3 个测试 |
| 7 | Beancount 导出可产出非法文件 | 描述未转义（引号/换行直接写进 `"..."`）；账户名含空格/引号会把账户名拆开 | `escape_beancount_text`（引号→单引号、反斜杠→`/`、换行/控制符→空格）、`sanitize_account_component`（非法字符折叠成 `-` 并按段首字母大写）；补 3 个测试 |
| 8 | React Query 收敛遗漏一处 | `AccountFields` 仍内联 `staleTime: Infinity` | 改用 `...QUERY_CONFIG.STATIC`，默认值只保留在 `constants/query.ts` |

**已知限制（未改）**：Beancount 账户名中的非 ASCII 字符（如「餐饮」）仍原样输出。beancount 语法只允许
`[A-Za-z0-9-]`，彻底修需要给账户生成稳定别名，会改变现有导出格式，留作后续决策。

**验证**：
- `cargo test` **399 passed / 0 failed**（新增 9 个：基线冻结 1、备份 3、Beancount 3、错误兜底 2）
- `make check` 全绿（新增错误文案表 `--check` 步骤）
- `npm run test:coverage` 72 passed，覆盖率 7.61/3.42/8.41/7.6 与门禁一致
- `cargo llvm-cov`：**实测门禁原本不可能通过**。原先 CI 写的是 `--fail-under-lines 95`
  且 `--ignore-filename-regex 'src/lib\.rs|src/main\.rs'`，但：
  1. 该正则用 `/` 作分隔符，Windows 上路径是 `src\lib.rs`，**排除根本没生效**；
  2. 即使排除生效，实测行覆盖 94.39%（排除入口与生成的 `errors/messages.rs` 后），
     95% 达不到。

  现改为平台无关正则 `src[\\/](lib|main)\.rs|src[\\/]errors[\\/]messages\.rs`，
  阈值按前端同一策略设为「实测基线略下」的 94，并在 Makefile 注明每轮提升。
  实测 `--fail-under-lines 94` 退出码 0（94.39%）。
