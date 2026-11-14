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
  ECharts 画进 `<canvas>`，**既不解析 `var()` 也不认 CSS 类**，所以必须取 computed style
  后在 JS 里给值，并依赖主题三元组重算。
  分类色板是 `--chart-1..8`（chroma 压在 0.09–0.16），网格线单独一支 `--chart-grid`
  （比 `--color-border` 更淡，否则抢数据）。已删掉 `CHART_COLORS` / `CATEGORY_COLORS` /
  `REPORT_CHART_COLORS` / `REPORT_CATEGORY_COLORS` 四份硬编码常量。
- **交给 ECharts 的颜色必须是 zrender 能解析的 `rgb()`/`hex`，不能是原始 `oklch()`**。
  zrender 自带解析器（`zrender/lib/tool/color.js`）只认 hex/rgb/hsl/hsv，
  `parse()` 认不出 `oklch()` 就返回空，而 `lift()`（ECharts 用 `liftColor()` 重建
  强调态样式的唯一入口）**解析失败时返回 `undefined` 而不是原色**——悬停态的描边
  因此变成 `undefined`，线在鼠标底下直接消失。自定义属性的 computed value 还保留
  未求值的 `calc()`（`oklch(0.23 calc(0.006 * 1) 250)`），所以也不能直接读。
  解法在 `chartColors.ts::resolveColor()`：把颜色画进 1×1 canvas 再读像素，
  拿到 canvas 真正会涂的 sRGB 三元组（`getComputedStyle` 与 `ctx.fillStyle`
  都会把 oklch 原样吐回来，不可用）。
- **ECharts 组件必须注册**：用了 `title`（环形图中心的总额）就得
  `echarts.use([TitleComponent])`，否则组件被跳过、数字根本不画，控制台只留一句
  `[ECharts] Component title is used but not imported`。注册清单在
  `src/components/charts/echarts.ts`，按需登记，别多也别少。
- ECharts 6 的 `grid.containLabel` 已属 legacy，不注册 `LegacyGridContainLabel`
  会每次画图都告警；本项目注册它，保留调好的旧布局。
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

## 材质与图表（2026-11-14 起）

- **图表库是 `echarts ^6.1.0`（2026-11-14 换掉 recharts）**；图标是 `lucide-react ^1.8.0`。
  按需注册在 `src/components/charts/echarts.ts`（3 种图 + 4 个组件 + canvas），
  **不要 `import * as echarts`**——包要塞进 Tauri 二进制，不上 CDN，map/geo/dataZoom 全是白重。
  图上样式一律走 `src/utils/chartTheme.ts`（轴/网格/tooltip/图例），不要逐图手写。
- **`EChart` 三个必需细节**（`components/charts/EChart.tsx`）：容器 0×0 时延迟 `init`
  （flex 子项首帧没高度）；`setOption(..., { notMerge: true })`（区间变短时旧序列会残留）；
  `beforeprint` 重绘（canvas 是位图，不会缩放）。`ResizeObserver` 缺失时降级到 window resize。
- **报表 6 张**（现金流/分类构成/净资产归因/资产负债表/账户明细/预算执行），
  状态在 URL（`?r=&p=&g=&d=&s=&e=&a=`），见 `useReportQuery` + `docs/design/24-reports.md`。
  四个旧报表（standard/trend/monthComparison/yearSummary）与两个账户报表已合并删除，
  后端 `report_month_comparison` / `report_year_summary` 一并移除——**不要加回来**。
- **审计在设置 → 数据体检**（`components/settings/AuditPanel.tsx`，弹窗展出），不在报表页。
- **`budgets` 表没有历史快照**：预算执行报表用今天的预算定义算旧月份，必须在 scope 行写明。
- **表面三态**：`Card` 的 `elevation="raise | well | float"` → `data-elevation`，
  CSS 在 `globals.css` 用属性选择器画；**默认不带 elevation**，首页之外暂保持平面。
  光源唯一（左上 45°）；内容面板不透明，`float`（Acrylic）只给临时浮层。
  **首页所有面板统一 `raise`**：单独给一张卡用 `well` 会被读成边框渲染出错，
  `well` 规则留作备用，不要在一屏对等面板里混用。
- 深度令牌写在 `:root` / `.dark`，**不要放 `@theme`**（没工具类引用会被 Tailwind 裁掉）。
  `@media print` 里全部复位为 `transparent` / `none`。
- `body::before` 是 32px/160px 校准网格（= 周 / 五周），不是壁纸；
  90 日热力带与它共用 13 个周列。**打印时 `display: none`**。
- **别用 `cn()` 包「未知自定义类 + 已知颜色类」**：tailwind-merge 会把
  `text-display` 当 text-color 删掉。这类组合用模板字符串。
- 首页打印是**单列文档流**：三栏 pane 强塞两列 grid 会重叠（`flex-1` + auto 高度）。

## 工具链坑

- `cargo llvm-cov` 在 Git Bash 下会解析到 `/usr/bin/link` 并挂住，必须用 PowerShell 跑。
- 覆盖率门禁为 94%（排除 `lib.rs`/`main.rs`/生成的 `errors/messages.rs`），
  正则以 `src[\\/]` 写，避免 Windows 路径失配。
- CI 的 clippy 带 `--all-targets`，比 `make check` 更严（连测试代码一起 lint）；
  本地验覆盖率/严格 clippy 时按 CI 口径跑，否则 push 后才发现。
- 新增 `commands/*.rs` 会立刻拉低覆盖率（`State<AppState>` 包不进普通单测）：
  按既有约定补 `mod ipc_tests`（`tauri::test::mock_builder` + `get_ipc_response`），
  先 `Database::create` 拿 `db.get_conn()` 塞夹具，再把 `db` 交给 mock app。
