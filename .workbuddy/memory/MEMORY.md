# GlimmerX 项目长期约定

## 产品决策（已定，不要再提议推翻）

- **单币种**：明确不做多币种。所有金额固定 CNY，不引入汇率表、不做账户级币种选择。
  多币种相关代码/UI/文案视为遗留，应清理而非扩展。（2026-11-13）
- **复式记账是核心**：保留 income/expense 账户，不允许"取消支出账户只留分类"。
  分类与损益账户按 1:1 绑定（`categories.account_id`），账本真相唯一定义在 posting 的账户上。
- **硬删除**：不做回收站、不做软删除、不做撤销。删除即 `DELETE`，恢复只能靠数据库备份。
- **导入导出暂时下线**：CSV / Beancount / CSV 导入已删除，不要再加回来。
  数据库备份（`db_backup`）保留，属于安全兜底。

## 架构约定

- **当前 schema 版本 = 3**（`db/migrations.rs` 的 `SCHEMA_VERSION`，迁移后校验
  `user_version`）。v2：`categories.account_id`、`postings.cleared` /
  `reconcile_id` + `reconciliations`、`savings_goals`、`v_posting_category` 视图，
  删 `transactions.deleted_at` / `is_reconciled`；v3：删 `transactions.category_id`，
  删除前归档到 `legacy_transaction_categories`（不可逆操作必须留审计痕迹）。
- **分类只能从分录取**：`categories.account_id ← postings.account_id`。
  不要再引入交易级分类缓存（拆分交易必然记错），聚合/筛选一律走分录。
- 数据库 schema 变更只走 `db/migrations.rs` 的 `M::up(...)` / `M::up_with_hook(...)`。
  v1/v2 已随 0.1.2 发版并冻结（`test_schema_v1_is_frozen` 用 sha256 锁住），
  **v3 尚未发版**，仍可直接改 `schema_v3.sql`；发版后同样冻结，只能新增 `schema_v4`。
- **迁移里的顺序不能靠直觉**：先 `ADD COLUMN` 再回填；`DROP COLUMN` 前必须先
  `DROP INDEX`（SQLite 拒绝删除被索引引用的列）。带 hook 的迁移无法 `const`，
  所以 `MIGRATIONS` 是 `static ... LazyLock<Migrations>`。
- 测试夹具一律调 `crate::db::init_schema` / `init_baseline`，不要再手写建表 DDL；
  建账户夹具要先查后建（分类镜像账户占着 `UNIQUE(type, name)`）。
- **对账粒度是 posting 不是 transaction**（GnuCash 口径）：一笔转账可能只有一侧
  上了对账单。修改/删除分录前必须 `assert_not_reconciled`（它同时看 `cleared = 1`
  与 `reconcile_id`，v2 回填会留下前者有后者无的状态）。
- **对账金额一律用"对账单符号"**：`db/reconcile.rs::statement_sign`（asset→1、
  liability→−1），由后端翻符号，前端不碰。信用卡账单印正数、账本里负债是负数。
- **期初余额靠谓词识别**：`OPENING_BALANCE_PREDICATE`（交易含 equity 分录），
  不要再用 description 字符串匹配。改权益侧不要依赖重新解析的 `equity_id`。
- **净资产账户谓词只有一处**：`reports::net_worth_accounts(alias)`。
- **前端金额解析只有一处**：`src/utils/money.ts::parseAmountToCents`，
  与 Rust `parse_amount_to_cents` 逐规则对齐（含 `MAX_AMOUNT_CENTS`）。
  禁止 `parseFloat(...)*100`。
- 跨模块私有函数用 `pub(in crate::db::xxx)` 提权，不扩大 crate 内 API。
- 错误文案的键必须同时存在于 `src/i18n/locales/{zh,en}.json` 的 `errors` 段；
  改完跑 `python tools/gen_error_messages.py`（`--check` 已进 `make check` 与 CI）。
  断言错误文案时照 zh.json 原文抄，不要意译（"已有一份对账单" ≠ "已对账"）。
- Windows 上批量改文件必须 `newline="\n"`，否则 Python 会把整文件写成 CRLF。

## 工具链坑

- `cargo llvm-cov` 在 Git Bash 下会解析到 `/usr/bin/link` 并挂住，必须用 PowerShell 跑。
- 覆盖率门禁为 94%（排除 `lib.rs`/`main.rs`/生成的 `errors/messages.rs`），
  正则以 `src[\\/]` 写，避免 Windows 路径失配。
- CI 的 clippy 带 `--all-targets`，比 `make check` 更严（连测试代码一起 lint）；
  本地验覆盖率/严格 clippy 时按 CI 口径跑，否则 push 后才发现。
- 新增 `commands/*.rs` 会立刻拉低覆盖率（`State<AppState>` 包不进普通单测）：
  按既有约定补 `mod ipc_tests`（`tauri::test::mock_builder` + `get_ipc_response`），
  先 `Database::create` 拿 `db.get_conn()` 塞夹具，再把 `db` 交给 mock app。
