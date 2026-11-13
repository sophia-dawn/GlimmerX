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

- **当前 schema 版本 = 2**（`db/migrations.rs` 的 `SCHEMA_VERSION`，迁移后校验
  `user_version`）。v2 带来：`categories.account_id`、`postings.cleared` /
  `reconcile_id` + `reconciliations`、`savings_goals`、`v_posting_category` 视图，
  并删掉 `transactions.deleted_at` / `is_reconciled`。
- 数据库 schema 变更只走 `db/migrations.rs` 的 `M::up(...)` / `M::up_with_hook(...)`；
  `schema_v1.sql`、`schema_v2.sql` 发布后都冻结，改动只能新增 `schema_v3`。
- **迁移里的顺序不能靠直觉**：先 `ADD COLUMN` 再回填；`DROP COLUMN` 前必须先
  `DROP INDEX`（SQLite 拒绝删除被索引引用的列）。带 hook 的迁移无法 `const`，
  所以 `MIGRATIONS` 是 `static ... LazyLock<Migrations>`。
- 测试夹具一律调 `crate::db::init_schema` / `init_baseline`，不要再手写建表 DDL；
  建账户夹具要先查后建（分类镜像账户占着 `UNIQUE(type, name)`）。
- **对账粒度是 posting 不是 transaction**（GnuCash 口径）：一笔转账可能只有一侧
  上了对账单。修改/删除分录前必须 `assert_not_reconciled`。
- 跨模块私有函数用 `pub(in crate::db::xxx)` 提权，不扩大 crate 内 API。
- 错误文案的键必须同时存在于 `src/i18n/locales/{zh,en}.json` 的 `errors` 段；
  改完跑 `python tools/gen_error_messages.py`（`--check` 已进 `make check` 与 CI）。
  断言错误文案时照 zh.json 原文抄，不要意译（"已有一份对账单" ≠ "已对账"）。
- Windows 上批量改文件必须 `newline="\n"`，否则 Python 会把整文件写成 CRLF。

## 工具链坑

- `cargo llvm-cov` 在 Git Bash 下会解析到 `/usr/bin/link` 并挂住，必须用 PowerShell 跑。
- 覆盖率门禁为 94%（排除 `lib.rs`/`main.rs`/生成的 `errors/messages.rs`），
  正则以 `src[\\/]` 写，避免 Windows 路径失配。
- 新增 `commands/*.rs` 会立刻拉低覆盖率（`State<AppState>` 包不进普通单测）：
  按既有约定补 `mod ipc_tests`（`tauri::test::mock_builder` + `get_ipc_response`），
  先 `Database::create` 拿 `db.get_conn()` 塞夹具，再把 `db` 交给 mock app。
