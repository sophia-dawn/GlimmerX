# GlimmerX - 阶段实施计划

> 版本 v1.0 | 2026-04-12

---

## 前置准备

在开始任何编码之前，先完成以下准备工作。

### 任务清单

| # | 任务 | 产出物 |
|---|---|---|
| P-1 | 初始化 Tauri 2 + React 19 + TypeScript 项目骨架 | `package.json`, `src-tauri/`, `src/` 目录结构 |
| P-2 | 配置 Vite 构建工具 | `vite.config.ts` |
| P-3 | 配置 Tailwind CSS + shadcn/ui | `tailwind.config.ts`, `components.json`, `src/components/ui/` |
| P-4 | 配置 TypeScript 严格模式 | `tsconfig.json` (strict: true) |
| P-5 | 配置 ESLint + Prettier | `eslint.config.js`, `.prettierrc` |
| P-6 | 初始化 Rust 后端依赖 | `Cargo.toml` (含 sqlcipher, serde, tauri, chrono 等) |
| P-7 | 配置路径别名 `@/` 指向 `src/` | `tsconfig.json` + `vite.config.ts` |
| P-8 | 创建 Makefile | `Makefile`（dev/lint/fmt/build/test/help） |
| P-9 | 验证 `npm run tauri dev` 可正常启动空白窗口 | 运行截图 |

### 验收标准

- [ ] `npm run tauri dev` 启动后显示一个空白窗口，无控制台报错
- [ ] `cargo test` 在 `src-tauri/` 下执行通过（无测试也需通过）
- [ ] `npx tsc --noEmit` 无错误

### 风险检查

- [ ] 验证 React 19 与 shadcn/ui 的兼容性
- [ ] 验证 `rusqlite` + `sqlcipher` 在开发环境（Linux WSL2）下可编译
- [ ] 确认 Tauri 2 的开发依赖已安装（`libwebkit2gtk-4.1-dev`, `build-essential` 等）

---

## Phase 1: 基础架构搭建

**目标**: 建立项目骨架、UI 框架、数据库连接、基础布局。

### 1.1 数据库集成

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| DB-1 | 添加 `sqlcipher` 依赖到 Cargo.toml | `src-tauri/Cargo.toml` | 使用 `libsqlite3-sys` + `sqlcipher` feature |
| DB-2 | 实现数据库连接管理器 | `src-tauri/src/db/mod.rs` | `Database` 结构体，持有 `rusqlite::Connection` |
| DB-3 | 实现数据库 Schema 初始化 | `src-tauri/src/db/schema.rs` | 包含所有 CREATE TABLE 语句的迁移脚本 |
| DB-4 | 实现 `db_create` Tauri Command | `src-tauri/src/commands/db.rs` | 接收密码 + 路径，创建加密数据库 |
| DB-5 | 实现 `db_unlock` Tauri Command | `src-tauri/src/commands/db.rs` | 接收密码，尝试打开数据库 |
| DB-6 | 实现 `db_change_password` Tauri Command | `src-tauri/src/commands/db.rs` | 旧密码解密 → 新密码加密 |
| DB-7 | 实现数据库状态管理 | `src-tauri/src/lib.rs` | 使用 `Mutex<Option<Database>>` 管理连接状态 |
| DB-8 | 编写数据库单元测试 | `src-tauri/src/db/` | 测试密码创建、解锁、加密连接 |

### 1.2 基础布局

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| UI-1 | 配置 React Router | `src/App.tsx` | 使用 `react-router-dom` v7 |
| UI-2 | 实现侧边栏组件 | `src/components/layout/Sidebar.tsx` | Logo + 导航项 + 底部状态栏 |
| UI-3 | 实现 Header 组件 | `src/components/layout/Header.tsx` | 页面标题 + 操作按钮 |
| UI-4 | 实现主布局 Shell | `src/components/layout/AppShell.tsx` | 侧边栏 + Header + 主内容区 |
| UI-5 | 实现解锁页 | `src/pages/UnlockPage.tsx` | 密码输入表单 + 首次使用引导 |
| UI-6 | 实现占位首页 | `src/pages/DashboardPage.tsx` | 显示"开发中"占位 |
| UI-7 | 主题切换 | `src/stores/themeStore.ts` | Zustand store + 跟随系统/暗色/亮色 |
| UI-8 | 全局 Toast 组件 | `src/components/ui/Toast.tsx` | 使用 shadcn Toast 或 sonner |

### 1.3 类型与工具

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| TYPE-1 | TypeScript 类型定义 | `src/types/index.ts` | Account, Transaction, Posting, Category, Tag, Budget 等 |
| UTIL-1 | 金额格式化工具 | `src/utils/format.ts` | 分 → 元转换，货币格式化 |
| UTIL-2 | 日期格式化工具 | `src/utils/date.ts` | 基于 date-fns |
| UTIL-3 | Tauri 调用封装 | `src/utils/api.ts` | 统一封装 `invoke()` 调用，错误处理 |

### Phase 1 验收标准

- [ ] 应用启动显示解锁页，输入密码后可进入主界面
- [ ] 主界面显示侧边栏 + 顶部 Header + 占位内容区
- [ ] 主题切换正常工作（暗色/亮色/跟随系统）
- [ ] 数据库创建和解锁端到端可通
- [ ] Rust 单元测试覆盖率 ≥ 95%（db 模块）
- [ ] TypeScript 类型零 any

---

## Phase 2: 账户与交易（核心）

**目标**: 实现账户 CRUD、复式记账交易录入、交易列表、账户余额计算。

### 2.1 账户模块

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| ACC-1 | 实现账户数据模型 | `src-tauri/src/models/account.rs` | Account 结构体 + 序列化 |
| ACC-2 | 实现账户 CRUD 数据库操作 | `src-tauri/src/db/accounts.rs` | insert/update/select/delete |
| ACC-3 | 实现账户树查询 | `src-tauri/src/db/accounts.rs` | 递归查询父/子账户关系 |
| ACC-4 | 实现 `account_create` Command | `src-tauri/src/commands/accounts.rs` | 含父账户关联、类型校验 |
| ACC-5 | 实现 `account_list` Command | `src-tauri/src/commands/accounts.rs` | 返回树形结构 |
| ACC-6 | 实现 `account_update` Command | `src-tauri/src/commands/accounts.rs` | |
| ACC-7 | 实现 `account_close` Command | `src-tauri/src/commands/accounts.rs` | 关闭前检查是否有未结交易 |
| ACC-8 | 实现 `account_balance` Command | `src-tauri/src/commands/accounts.rs` | 汇总所有 postings 计算余额 |
| ACC-9 | 实现 `account_transfer` Command | `src-tauri/src/commands/accounts.rs` | 自动生成平衡分录 |
| ACC-10 | 账户管理前端页面 | `src/pages/AccountsPage.tsx` | 账户树展示 + 增删改弹窗 |
| ACC-11 | 账户表单组件 | `src/components/accounts/AccountForm.tsx` | 名称、类型、父账户、初始余额 |
| ACC-12 | 账户树组件 | `src/components/accounts/AccountTree.tsx` | 可展开/折叠的树形展示 |

### 2.2 交易模块

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| TXN-1 | 实现交易数据模型 | `src-tauri/src/models/transaction.rs` | Transaction, Posting 结构体 |
| TXN-2 | 实现复式记账校验器 | `src-tauri/src/utils/validation.rs` | `sum(postings.amount) == 0` 校验 |
| TXN-3 | 实现交易 CRUD 数据库操作 | `src-tauri/src/db/transactions.rs` | 含事务包裹（交易+分录原子写入） |
| TXN-4 | 实现 `transaction_create` Command | `src-tauri/src/commands/transactions.rs` | 调用校验器，失败返回错误 |
| TXN-5 | 实现 `transaction_get` Command | `src-tauri/src/commands/transactions.rs` | 含关联 postings |
| TXN-6 | 实现 `transaction_list` Command | `src-tauri/src/commands/transactions.rs` | 按时间倒序，支持分页参数 |
| TXN-7 | 实现 `transaction_update` Command | `src-tauri/src/commands/transactions.rs` | 先删旧分录再写新分录 |
| TXN-8 | 实现 `transaction_delete` Command | `src-tauri/src/commands/transactions.rs` | CASCADE 删除关联分录 |
| TXN-9 | 实现 `transaction_search` Command | `src-tauri/src/commands/transactions.rs` | 全文搜索 description + 账户过滤 |
| TXN-10 | 交易录入前端页面 | `src/pages/TransactionNewPage.tsx` | 复式分录表单，实时借贷平衡校验 |
| TXN-11 | 交易列表页面 | `src/pages/TransactionsPage.tsx` | 按日期分组卡片列表 |
| TXN-12 | 交易卡片组件 | `src/components/transactions/TransactionCard.tsx` | 日期、描述、分类、金额（着色） |
| TXN-13 | 分录表单组件 | `src/components/transactions/PostingForm.tsx` | 动态增减分录行 |
| TXN-14 | 交易编辑页面 | `src/pages/TransactionEditPage.tsx` | 复用录入表单，预填充数据 |

### 2.3 分类与标签

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| CAT-1 | 实现分类数据模型 + CRUD | `src-tauri/src/db/categories.rs` | 树形结构 |
| CAT-2 | 实现标签数据模型 + CRUD | `src-tauri/src/db/tags.rs` | 扁平结构 |
| CAT-3 | 分类/标签相关 Commands | `src-tauri/src/commands/categories.rs`, `src-tauri/src/commands/tags.rs` | |
| CAT-4 | 分类管理页面 | `src/pages/CategoriesPage.tsx` | 树形增删改 |
| CAT-5 | 标签管理页面 | `src/pages/TagsPage.tsx` | 列表 + 颜色设置 |

### Phase 2 验收标准

- [ ] 可创建账户树（资产/负债/收入/支出/权益）
- [ ] 可录入复式记账交易，借贷不平衡时拒绝保存
- [ ] 交易列表按时间倒序展示，支持搜索
- [ ] 可编辑和删除交易
- [ ] 账户余额计算正确
- [ ] 分类树和标签列表可管理
- [ ] Rust 单元测试覆盖率 ≥ 95%（交易校验器 100%）
- [ ] Playwright E2E 覆盖：新建账户 → 新建交易 → 验证列表

---

## Phase 3: 预算与快速记账

**目标**: 实现预算管理、信封预算、快速记账模式、定期交易。

### 3.1 预算模块

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| BUD-1 | 实现预算数据模型 + CRUD | `src-tauri/src/db/budgets.rs` | 含月份字段区分周期 |
| BUD-2 | 实现 `budget_set` / `budget_delete` Commands | `src-tauri/src/commands/budgets.rs` | |
| BUD-3 | 实现预算状态计算 | `src-tauri/src/db/reports.rs` | 已用/剩余/超支状态 |
| BUD-4 | 实现 `budget_list` Command | `src-tauri/src/commands/budgets.rs` | 返回 BudgetStatus 列表 |
| BUD-5 | 预算设置页面 | `src/pages/BudgetsPage.tsx` | 分类选择 + 金额输入 + 周期选择 |
| BUD-6 | 预算进度页面 | `src/components/budgets/BudgetProgress.tsx` | 进度条 + 超支高亮 + 结余 |

### 3.2 快速记账

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| QA-1 | 实现 `transaction_quick_add` Command | `src-tauri/src/commands/transactions.rs` | 自动推导分录 |
| QA-2 | 快速记账弹窗组件 | `src/components/transactions/QuickAddDialog.tsx` | 金额 + 分类 + 备注 |
| QA-3 | 全局快捷键绑定 | `src/hooks/useKeyboardShortcuts.ts` | Ctrl+K 搜索, Ctrl+N 快速记账 |

### 3.3 定期交易

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| REC-1 | 实现定期交易模板 CRUD | `src-tauri/src/db/recurring.rs` | 含模板分录表 |
| REC-2 | 实现定期交易生成逻辑 | `src-tauri/src/commands/recurring.rs` | 根据频率自动生成交易 |
| REC-3 | 定期交易管理 UI | `src/pages/RecurringPage.tsx` | 模板列表 + 创建/编辑/删除 |

### Phase 3 验收标准

- [ ] 可为分类设置月度/周度预算，显示进度条
- [ ] 超支分类红色高亮
- [ ] 快速记账弹窗通过快捷键触发，自动生成平衡分录
- [ ] 定期交易模板可创建，自动生成交易记录
- [ ] Rust 单元测试覆盖率 ≥ 95%（预算计算 100%）

---

## Phase 4: 报表与分析

**目标**: 实现资产负债表、收支趋势图、分类统计图。

### 4.1 报表后端

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| RPT-1 | 实现资产负债表聚合 | `src-tauri/src/db/reports.rs` | `report_balance_sheet` |
| RPT-2 | 实现收支统计聚合 | `src-tauri/src/db/reports.rs` | `report_income_expense` |
| RPT-3 | 实现分类统计聚合 | `src-tauri/src/db/reports.rs` | `report_category_breakdown` |
| RPT-4 | 实现趋势数据聚合 | `src-tauri/src/db/reports.rs` | `report_trend` |

### 4.2 报表前端

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| RPT-UI-1 | 报表页面框架 | `src/pages/ReportsPage.tsx` | Tab 切换不同报表 |
| RPT-UI-2 | 资产负债表组件 | `src/components/reports/BalanceSheet.tsx` | 资产 = 负债 + 权益 |
| RPT-UI-3 | 收支趋势图 | `src/components/reports/IncomeExpenseChart.tsx` | Recharts 折线+柱状组合图 |
| RPT-UI-4 | 分类占比图 | `src/components/reports/CategoryPieChart.tsx` | Recharts 环形图 |
| RPT-UI-5 | 概览页数据卡片 | `src/pages/DashboardPage.tsx` | 本月收支、账户余额、最近交易 |

### Phase 4 验收标准

- [ ] 资产负债表严格满足 资产 = 负债 + 权益
- [ ] 收支趋势图可按月度/周度切换
- [ ] 分类占比图按金额排序
- [ ] 概览页显示关键数据卡片
- [ ] Rust 单元测试覆盖率 ≥ 95%

---

## Phase 5: 数据管理 & 完善

**目标**: 数据导入/导出、备份恢复、设置页面、快捷键、打包发布。

### 5.1 数据导入/导出

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| IMP-1 | CSV 导出实现 | `src-tauri/src/commands/export.rs` | `export_csv` |
| IMP-2 | CSV 导入实现 | `src-tauri/src/commands/import.rs` | `import_csv` 含字段映射 |
| IMP-3 | Beancount 导出 | `src-tauri/src/commands/export.rs` | `export_beancount` |
| IMP-4 | 前端导入/导出 UI | `src/components/settings/DataManagement.tsx` | 文件选择器 + 进度提示 |

### 5.2 备份与恢复

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| BAK-1 | `db_backup` Command | `src-tauri/src/commands/db.rs` | 复制加密数据库文件 |
| BAK-2 | `db_restore` Command | `src-tauri/src/commands/db.rs` | 从备份恢复，需密码验证 |
| BAK-3 | 备份管理 UI | `src/components/settings/BackupManager.tsx` | 手动备份 + 恢复选择器 |

### 5.3 设置页面

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| SET-1 | 设置页面框架 | `src/pages/SettingsPage.tsx` | 多 Tab: 通用/数据/快捷键 |
| SET-2 | 货币/日期格式设置 | `src/components/settings/GeneralSettings.tsx` | |
| SET-3 | 快捷键帮助弹窗 | `src/components/settings/KeyboardShortcuts.tsx` | Ctrl+/ 触发 |
| SET-4 | 密码修改 | `src/components/settings/SecuritySettings.tsx` | 旧密码 → 新密码 |

### 5.4 打包与发布

| # | 任务 | 文件 | 说明 |
|---|---|---|---|
| REL-1 | 配置 Tauri 构建 | `src-tauri/tauri.conf.json` | 应用名称、图标、权限 |
| REL-2 | GitHub Actions CI/CD | `.github/workflows/build.yml` | 三平台自动构建 |
| REL-3 | 应用图标和资源 | `src-tauri/icons/` | 多尺寸图标 |
| REL-4 | 版本号管理 | `src-tauri/Cargo.toml` + `package.json` | 统一版本 |

### Phase 5 验收标准

- [ ] CSV 导出数据与数据库一致，可循环导入
- [ ] Beancount 导出格式可被 Beancount 解析
- [ ] 数据备份文件可在密码解锁后恢复
- [ ] 三平台 CI 构建通过
- [ ] 快捷键全局绑定生效
- [ ] 所有 P0 功能完成，P1 功能完成 ≥ 80%

---

## 全局任务（贯穿所有阶段）

### 测试

| # | 任务 | 阶段 | 说明 |
|---|---|---|---|
| TEST-1 | Rust 单元测试框架搭建 | Phase 1 | `cargo test` + `cargo-llvm-cov` 配置 |
| TEST-2 | Playwright E2E 测试框架 | Phase 2 | Vite dev server + 页面流程测试 |
| TEST-3 | tauri-driver + WebdriverIO 桌面 E2E | Phase 5 | 完整 Tauri 应用测试 |

### CI/CD

| # | 任务 | 阶段 | 说明 |
|---|---|---|---|
| CI-1 | 代码检查: clippy, eslint, tsc | Phase 1 | 任一失败则 CI 失败 |
| CI-2 | 三平台构建 | Phase 2 | macOS / Linux / Windows |
| CI-3 | 覆盖率门禁 | Phase 2 | Rust 覆盖率 < 95% 则失败 |
| CI-4 | Release 发布 | Phase 5 | GitHub Releases 上传构建产物 |

### 文档

| # | 任务 | 阶段 | 说明 |
|---|---|---|---|
| DOC-1 | README.md | Phase 1 | 项目介绍、开发指南 |
| DOC-2 | API 文档 | Phase 2 | Tauri Commands 说明 |
| DOC-3 | 用户手册 | Phase 5 | 使用指南、FAQ |

---

## 优先级总结

| 阶段 | 核心交付 | 预计工作量 | 前置依赖 |
|---|---|---|---|
| **前置准备** | 项目骨架、构建工具、依赖安装 | 1天 | 无 |
| **Phase 1** | 数据库 + 布局 + 解锁 | 3-4天 | 前置准备 |
| **Phase 2** | 账户 + 交易 + 分类标签 | 5-7天 | Phase 1 |
| **Phase 3** | 预算 + 快速记账 + 定期交易 | 4-5天 | Phase 2 |
| **Phase 4** | 报表分析 | 3-4天 | Phase 2 |
| **Phase 5** | 导入导出 + 备份 + 打包 | 4-5天 | Phase 3, 4 |

> **注意**: 工作量为粗粒度估算，实际根据开发节奏调整。Phase 2 和 Phase 4 可部分并行（报表后端不依赖预算功能）。
