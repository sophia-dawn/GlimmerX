# GlimmerX 项目长期约定

## 产品决策（已定，不要再提议推翻）

- **单币种**：明确不做多币种。所有金额固定 CNY，不引入汇率表、不做账户级币种选择。
  多币种相关代码/UI/文案视为遗留，应清理而非扩展。（2026-11-13）
- **复式记账是核心**：保留 income/expense 账户，不允许"取消支出账户只留分类"。
  分类与损益账户按 1:1 绑定（`categories.account_id`），账本真相唯一定义在 posting 的账户上。
- **硬删除**：不做回收站、不做软删除、不做撤销。删除即 `DELETE`，恢复只能靠数据库备份。
- **导入导出暂时下线**：CSV / Beancount / CSV 导入已删除，不要再加回来。
  数据库备份（`db_backup`）保留，属于安全兜底。

## 视觉系统（2026-11-13 起，terminal 语言）

- **风格定位**：深色交易终端（Bloomberg 血统）+ 工业仪器的纪律。命名 `terminal`，
  浅色配套叫 `paper`。**不做**霓虹渐变、发光、玻璃拟态——那些是任何产品都能穿的外套。
- **令牌只在 `src/styles/globals.css` 的 `@theme`**。表面阶梯是参数化的：
  `--s-base/--s-card/--s-hover/--s-float/--s-sunken` + `--hue/--chroma`，
  所以氛围主题只覆盖色相（每个一行），不要写第二份完整调色板。
- **圆角收敛但不清零**：控件 6px（`--radius-md`）/ 卡片 10px（`--radius-xl`）/ 弹窗 8px，
  阴影全部为 none（`--shadow-*: none`）。层级靠表面亮度与 1px 发丝线，不靠 blur。
  2026-11-13 曾压到 2px，32px 高的按钮看着像被削角，用户要求改回——**别再往下压**。
  `rounded-full` 只给开关与圆点，不做药丸徽章。
- **强调色只有一支** `--color-signal`（琥珀 `#FFA02F` / 浅色 `#8A5410`），
  只用在：关键数字、当前项、超支与待办。`--color-primary` 是墨色（近白/近黑），
  不是强调色——满屏琥珀按钮会让强调失效。
- **语义色不走强调色系**：`--color-income` / `--color-expense` / `--color-info`
  独立定义，换强调色不能让红绿变紫。
- **禁止 Tailwind 调色板类**（`text-red-600` / `bg-emerald-500` 等）。
  一律用语义令牌：`text-income` / `text-expense` / `text-signal` / `text-info`。
  已全量替换 22 个文件、73 处；新增代码照此办理。
- **数字用 `font-mono`（JetBrains Mono，本地 woff2，含 OFL.txt）**，
  `html` 上已挂 `font-feature-settings: "tnum" 1`，不要再逐个加 `tabular-nums`。
  中文**不要**走 mono：汉字等宽会撑爆列宽，`--font-mono` 里刻意不列 CJK 字体。
- **字阶 11 / 13 / 15 / 20**（`--text-xs` … 覆盖了 Tailwind 默认的 12/14/16）。
- 动效 120ms `cubic-bezier(.2,0,0,1)`，无回弹；`prefers-reduced-motion` 全局降级。
- 对比度按实测记账：深色底 `#7E8B9C` = 5.46:1 可做小字；**`#5B6675` 只有 3.24:1，
  只许用于非文字元素**（网格线、禁用态）。
- **主题 = `mode`（light/dark/system）× `preset`（4 套）**，不是原来的 18 强调色 × 7 氛围。
  预设用 `data-preset` 属性写在 `<html>`：`terminal` / `terminal-cool` / `paper` / `contrast`。
  一个预设只给「色相 + **色度倍数** + 信号三元组（+ contrast 的发丝线）」；
  色度是 `--chroma: calc(var(--chroma-base) * var(--chroma-scale))`，
  **不要**让预设直接写 `--chroma`（浅色 0.006 / 深色 0.019，同一个值照顾不了两边）。
  预设块必须排在 `.dark` 块**之后**（同权重靠顺序取胜）。老 localStorage
  （`glimmerx:color`/`glimmerx:atmosphere`）在 `themeStore` 里映射成预设后删除，
  不要再保留 18+7 个 CSS 别名类。设置页缩略图靠 `.preset-preview` 把预设套在子树上。
- **数字按位置分三级**：页面级汇总 `text-xl font-mono font-medium`（20px）、
  卡片关键数字 `text-base font-mono`（15px）、表格与列表 `text-sm font-mono`（13px）。
  全站已无 `text-2xl`/`text-3xl`/`font-bold`；中文最强只到 `font-medium`（600 会合成糊块）。
- **图表颜色从主题读，不要写死 hex**：`src/utils/chartColors.ts::useChartColors()`。
  Recharts 的 `fill`/`stroke` 是 SVG presentation attribute，**不解析 `var()`**，
  所以必须取 computed style 后在 JS 里给值，并依赖主题三元组重算。
  分类色板是 `--chart-1..8`（chroma 压在 0.09–0.16），网格线单独一支 `--chart-grid`
  （比 `--color-border` 更淡，否则抢数据）。已删掉 `CHART_COLORS` / `CATEGORY_COLORS` /
  `REPORT_CHART_COLORS` / `REPORT_CATEGORY_COLORS` 四份硬编码常量。
- **金额负数写作 `-¥50.00`**（符号在货币符号前），不是 `¥-50.00`。`format.test.ts` 锁住。
- **首页是状态条 + 三栏**，不是卡片流：`DashboardStatusBar`（净资产/收入/支出/结余，
  无外框、发丝线分隔）+ 左栏图表 / 中栏交易表 / 右栏预算·账户·Top 支出。
  `DashboardSummaryCards` 与 `FinancialHealthCards` 已删（7 张卡读 7 个数字，
  且每个数字各在自己的基线上，没法横向比较）。
- 列表金额用 `flow_amount` 着色（只汇总资产/负债侧）：支出负、收入正、自有账户互转 0。
  `display_amount` 是借方合计、恒为正，拿它判断方向是死分支。
- 浅色态 `--line` 是 0.15（不是 0.13）：没有阴影时这条线是卡片与页面的唯一分界。

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
