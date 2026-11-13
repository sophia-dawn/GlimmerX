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

- 数据库 schema 变更只走 `db/migrations.rs` 的 `M::up(...)` / `M::up_with_hook(...)`；
  `schema_v1.sql` 已冻结（`test_schema_v1_is_frozen` 校验 sha256），任何改动都会失败。
- 跨模块私有函数用 `pub(in crate::db::xxx)` 提权，不扩大 crate 内 API。
- 错误文案的键必须同时存在于 `src/i18n/locales/{zh,en}.json` 的 `errors` 段；
  改完跑 `python tools/gen_error_messages.py`（`--check` 已进 `make check` 与 CI）。
- Windows 上批量改文件必须 `newline="\n"`，否则 Python 会把整文件写成 CRLF。

## 工具链坑

- `cargo llvm-cov` 在 Git Bash 下会解析到 `/usr/bin/link` 并挂住，必须用 PowerShell 跑。
- 覆盖率门禁为 94%（排除 `lib.rs`/`main.rs`/生成的 `errors/messages.rs`），
  正则以 `src[\\/]` 写，避免 Windows 路径失配。
