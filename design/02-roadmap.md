# 02 - 开发计划

> 返回 [DESIGN.md](../DESIGN.md)

## Phase 1: 基础搭建（骨架） ✅ 已完成

- [x] Tauri 2 + React 19 + TypeScript 项目初始化
- [x] shadcn/ui + Tailwind CSS 配置
- [x] SQLCipher 数据库集成
- [x] 基础布局（侧边栏 + 主内容区）
- [x] 主题切换（暗色/亮色）
- [x] 国际化（i18next，中文/英文）
- [x] 解锁/创建账本流程

## Phase 1.5: 账户字段参考 Firefly III ✅ 已完成

- [x] accounts 表扩展（23 列：description, account_number, subtype, iban, bic, credit_card_type 等）
- [x] 创建账户 Bug 修复（字段不再被静默丢弃）
- [x] 资产子类型枚举：defaultAsset / sharedAsset / savingAsset / ccAsset / cashWalletAsset
- [x] 负债子类型枚举：loan / debt / mortgage
- [x] 禁止创建 equity 账户
- [x] 前端条件字段渲染（信用卡/负债专用字段）
- [x] 完整的 i18n 支持

## Phase 2: 账户与交易（核心）

- [x] 账户 CRUD + 树形展示（含完整字段）
- [x] 交易录入（复式分录）
- [x] 交易列表（搜索、过滤、分页）
- [x] 交易编辑/删除
- [x] 账户余额计算
- [x] Dashboard 概览模块（收支汇总、财务健康、月度图表、分类统计、Top 支出、最近交易、账户余额列表）

## Phase 3: 分类与预算 ✅ P0 已完成

- [x] 分类管理（扁平列表 CRUD）
- [x] 预算设置与进度展示（BUD-1~BUD-3）
- [x] 快速记账模式（基础版已完成）
- [ ] 信封预算结余滚存（BUD-4，P1 预留）

## Phase 4: 报表与分析 ✅ P0/P1/P2 核心已完成

- [x] 报表页面架构（下拉选择器切换 + 通用过滤器）
- [x] `standard` 标准财务报表（收入支出汇总 + 净资产趋势）
- [x] `category` 分类分析报告（分类收支聚合 + 饼图）
- [x] `balanceSheet` 资产负债表（资产/负债/净资产快照）
- [x] `trend` 收支趋势图（多时间粒度）
- [x] `monthComparison` 月度对比报告（两月收支对比）
- [x] `yearSummary` 年度汇总报告（全年收支分解）
- [x] `accountTransactions` 账户交易报告（单账户交易明细）
- [x] `accountBalanceTrend` 账户余额趋势（单账户余额历史）
- [x] `audit` 审计报告（交易验证 + 异常检测）
- [x] 预算执行报告（预算模块，`report_budget_execution` API）
- [ ] 收款人分析报告（依赖收款人功能）
- [ ] 投资报告（依赖投资账户类型）
- [ ] 现金流预测（依赖定期交易功能）
- [ ] 自定义报表生成器（复杂度高，后期）

## Phase 5: 数据管理 & 完善 📝 已设计

- [x] 设计文档完成（见 [19-data-export](19-data-export.md)）
- [ ] 数据库备份（DATA-4）— 一键复制加密数据库文件
- [ ] CSV 导出（DATA-1）— 多行格式，含 transaction_id
- [ ] Beancount 导出（DATA-3）— 标准 Beancount 语法
- [ ] CSV 导入（DATA-2）
- [ ] 数据备份与恢复（DATA-4、DATA-5）
- [ ] 定期交易模板
- [ ] 快捷键
- [ ] 打包发布
