# 08 - 技术约束与约定

> 返回 [DESIGN.md](../DESIGN.md)

## 金额处理

- 数据库中金额以**分**（整数）为单位存储
- 所有金额计算在 Rust 后端完成
- 前端使用 `react-number-format` 展示格式化的货币

## 复式记账约束

- 每笔交易的 `postings` 金额总和**必须为零**
- 在后端 `commands/transactions.rs` 中强制校验
- 前端在提交前做预校验并提示用户

## 数据库加密

- 使用 **SQLCipher**（SQLite 的 AES-256 加密扩展）
- 密码**不存储**在本地，仅用于派生 SQLCipher 密钥（PBKDF2 + salt）
- 密码错误时表现为"数据库损坏"（防止暴力试探）
- 修改密码：旧密码解密 → 新密码重新加密（原地替换）

## accounts 表设计

- 当前 23 列，直接修改 schema 而非迁移（Phase 1.5 重构完成）
- `subtype` 列为**多态字段**：
  - asset 类型下解释为 `defaultAsset`/`sharedAsset`/`savingAsset`/`ccAsset`/`cashWalletAsset`
  - liability 类型下解释为 `loan`/`debt`/`mortgage`
- 唯一索引: `(name, COALESCE(parent_id, ''))`
- Equity 账户由系统管理，`account_create` 拦截 `equity`/`equities` 路径
- `is_system` 标记系统内置账户（不可编辑/删除）

## 最近数据库列表

- 存储位置：`~/.config/glimmerx/recent_dbs.json`（Linux）/ `%APPDATA%`（Windows）/ `~/Library/Application Support/`（macOS）
- 最多 **10 条**，包含 path、label、last_opened
- 点击条目时验证文件存在，不存在则清理

## 并发与安全

- SQLite 使用 WAL 模式支持并发读写
- Tauri 命令通过 IPC 串行处理写操作

## 状态管理

- 服务端状态：**TanStack Query**（通过 Tauri adapter）
- 客户端状态：**Zustand**

## 代码规范

**前端**：ESLint（flat config）+ Prettier（80 列、双引号、尾逗号、2 空格）
**后端**：`cargo clippy -- -D warnings` + `cargo fmt`
**Pre-commit Hook**：`make check`（前端 tsc + eslint + prettier，后端 cargo fmt + clippy）

## 键盘快捷键

| 快捷键 | 操作 | 页面 |
|---|---|---|
| `Ctrl/Cmd + N` | 快速记账 | 全局 |
| `Ctrl/Cmd + K` | 搜索交易 | 全局 |
| `Ctrl/Cmd + /` | 显示快捷键列表 | 全局 |
| `Ctrl/Cmd + W` | 锁定数据库（锁屏） | 全局 |
| `Escape` | 关闭弹窗/取消编辑 | 任意 |
| `Delete` | 删除选中交易 | 交易列表 |
| `Enter` | 保存表单 | 表单页 |
| `Ctrl/Cmd + Z` | 撤销上一步操作 | 交易编辑 |
