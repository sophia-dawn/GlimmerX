# 09 - 测试策略

> 返回 [DESIGN.md](../DESIGN.md)

## 测试层级

| 层级 | 工具 | 范围 | 阶段 |
| --- | --- | --- | --- |
| Rust 单元测试 | `cargo test` + `cargo-llvm-cov` | 数据库操作、复式记账校验、金额计算 | Phase 2 |
| Rust 集成测试 | `cargo test --test` | 完整 Command 流程（Tauri mock runtime） | Phase 2 |
| React 组件测试 | Vitest + Testing Library | 表单组件、交易列表、预算进度条 | Phase 2 |
| 前端 E2E | **Playwright**（Vite dev server） | 表单交互、路由跳转、主题切换 | Phase 2-4 |
| 桌面 E2E | **tauri-driver + WebdriverIO** | 数据库解锁、交易 CRUD、CSV 导入导出 | Phase 5+ |

## 覆盖率要求

- Rust 单元测试 **≥ 95%**（`cargo-llvm-cov`）
- 关键模块（复式记账校验、金额计算、数据库加密）**100%**
- CI 中低于阈值则构建失败

## 重点测试用例

- 交易创建时必须校验 `sum(postings.amount) == 0`
- 账户关闭后不可新增交易
- 预算超支计算正确
- 数据库解锁流程
- CSV 导入导出数据一致性
