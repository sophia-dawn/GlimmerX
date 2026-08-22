# AI 记账功能设计文档

- **日期**: 2026-08-22
- **状态**: 已批准
- **作者**: brainstorming session

## 一、功能概述

用户在独立 AI 对话框输入自然语言（如"中午吃饭18元"），应用调用 AI 服务解析为结构化交易数据，自动录入一条支出/收入交易。支持云端（OpenAI/DeepSeek/自定义 OpenAI 兼容服务）和本地（Ollama）双模式，用户在设置页配置。

## 二、用户决策汇总

| 维度 | 决策 |
|---|---|
| AI 服务 | 云端 + 本地双模式，统一 OpenAI 兼容协议 + 自定义 BaseURL |
| 交互方式 | 独立对话框，AI 直接录入不确认 |
| 分类匹配 | 找不到自动创建 |
| 账户选择 | AI 推断 + 设置里预设默认账户 |
| 日期 | AI 返回 YYYY-MM-DD（注入当前日期上下文），后端校验，失败报错不默认今天 |
| 兜底 | 后端校验失败报错不录入 |
| Prompt 语言 | 英文 |
| 错误消息 | 不带 HTTP 状态码，只表达类别 |
| 交易类型 | 首期只支持 expense/income，transfer 留二期 |
| 测试连接 | 保留"测试连接"按钮 |
| 成功提示 | toast 只显示"已录入" |

## 三、端到端数据流

```
用户在 AiInputDialog 输入 "昨天午饭15块 微信付的"
  │
  ▼ invoke("ai_parse_transaction", { text })
  │
  ▼ Rust: commands/ai.rs::ai_parse_transaction
  │  1. 读 settings: ai.base_url / ai.api_key / ai.model / ai.default_source_account_id
  │     → 任一缺失报 errors.ai.noApiKey / noBaseUrl / noModel
  │  2. 查 accountList + categoryList 组装 AiContext
  │  3. services/ai.rs::parse_transaction(config, text, context)
  │     a. 构造英文 system prompt（注入当前日期/时区/分类清单/账户清单）
  │     b. reqwest POST {base_url}/chat/completions
  │        body: { model, messages, response_format:{type:"json_object"}, temperature:0.1 }
  │        timeout: 30s
  │     c. 非2xx → errors.ai.apiCallFailed
  │     d. 网络/超时 → errors.ai.networkError / timeout
  │     e. 解析 choices[0].message.content → JSON
  │        - JSON 解析失败 → 尝试正则提取 {...}；仍失败 → errors.ai.parseFailed
  │     f. 字段校验：
  │        - mode ∉ {expense,income} → errors.ai.invalidMode（transfer 二期）
  │        - amount ≤0 或非数字 → errors.ai.invalidAmount
  │        - date 格式非 YYYY-MM-DD 或无法解析或未来>1天或<1900 → errors.ai.invalidDate
  │        - date 为 null/空 → errors.ai.missingDate
  │     g. 返回 AiParseResult { mode, amount, category_name, account_hint, date, description }
  │  4. 分类匹配（commands/ai.rs）：
  │     a. 精确匹配 categoryName（忽略大小写/空格）
  │     b. 无精确 → 包含匹配（双向）
  │     c. 仍无 → categoryCreate({ name, type, icon: null })
  │  5. 账户推断：
  │     a. accountHint 在 asset 账户 name 中模糊匹配
  │     b. 匹配不到 → 用 ai.default_source_account_id
  │     c. expense 无 source → errors.ai.noSourceAccount
  │  6. 组装 QuickAddInput，调 db::transactions::quick_add_transaction（复用现有推导+校验）
  │  7. 返回 TransactionDto
  │
  ▼ 前端 AiInputDialog
     - 成功：invalidate transactionListPaginated / transactionDetail / accounts
       → 关闭对话框 → toast t("ai.success")
     - 失败：translateErrorMessage(err, t) 显示红字，不关闭对话框
```

## 四、后端改动清单

### 4.1 新增依赖（src-tauri/Cargo.toml）
```toml
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }
```

### 4.2 新增/修改文件

| 文件 | 动作 | 职责 |
|---|---|---|
| `src-tauri/src/db/mod.rs` | 修改 | 新增 `AppError::AiError(String)` 变体，`pub mod settings;` |
| `src-tauri/src/db/settings.rs` | 新增 | `get_setting` / `set_setting`（参数化查询，复用已有 settings 表） |
| `src-tauri/src/services/mod.rs` | 新增 | `pub mod ai;` |
| `src-tauri/src/services/ai.rs` | 新增 | AiConfig/AiParseResult/AiContext 结构体；AiHttpClient trait；parse_transaction async 函数；validate_date/mode/amount、parse_json_response、extract_json_from_content 纯函数 |
| `src-tauri/src/commands/settings.rs` | 新增 | get_setting / set_setting 命令 |
| `src-tauri/src/commands/ai.rs` | 新增 | ai_parse_transaction / ai_test_connection 命令；分类匹配/账户推断/组装 QuickAddInput |
| `src-tauri/src/commands/mod.rs` | 修改 | `pub mod settings; pub mod ai;` |
| `src-tauri/src/lib.rs` | 修改 | generate_handler! 追加 4 个命令 |

### 4.3 Settings 键（存数据库 settings 表，无 schema 变更）
| key | value 示例 | 说明 |
|---|---|---|
| `ai.provider` | `openai`/`deepseek`/`ollama`/`custom` | 服务商 |
| `ai.base_url` | `https://api.openai.com/v1` | API 端点 |
| `ai.api_key` | `sk-...` | 密钥（Ollama 可空） |
| `ai.model` | `gpt-4o-mini` | 模型名 |
| `ai.default_source_account_id` | `<uuid>` | 默认支出账户 |

## 五、前端改动清单

| 文件 | 动作 | 职责 |
|---|---|---|
| `src/types/index.ts` | 修改 | 新增 AiConfig interface |
| `src/utils/api.ts` | 修改 | 新增 aiParseTransaction / aiTestConnection / getSetting / setSetting |
| `src/components/transactions/AiInputDialog.tsx` | 新增 | 独立 AI 对话框 |
| `src/components/layout/AppShell.tsx` | 修改 | aiInputOpen state + 渲染 + 快捷键 Ctrl/Cmd+Shift+I |
| `src/components/layout/Header.tsx` | 修改 | AI 按钮（Sparkles 图标） |
| `src/pages/SettingsPage.tsx` | 修改 | 新增 AI 记账区块 |
| `src/i18n/locales/en.json` | 修改 | ai 顶层域 + errors.ai 子域 |
| `src/i18n/locales/zh.json` | 修改 | 同上平行 |
| `src/components/transactions/AiInputDialog.test.tsx` | 新增 | Vitest 组件测试 |

## 六、错误处理与 i18n

### 6.1 AppError 新变体
```rust
#[error("errors.ai.{0}")]
AiError(String),
```

### 6.2 i18n 键
- `ai` 顶层域：title / inputPlaceholder / button / recognizing / success / settings.*
- `errors.ai` 子域：noApiKey / noBaseUrl / noModel / apiCallFailed / parseFailed / invalidDate / missingDate / invalidMode / invalidAmount / networkError / timeout / noSourceAccount

错误消息不带 HTTP 状态码。前端用现有 translateErrorMessage。

## 七、测试策略

### Rust 单元测试（覆盖率 ≥ 95%）
- services/ai.rs：validate_date_*(5) / validate_mode / validate_amount / parse_json_response_*(3) / extract_json_from_content — ~10 个
- db/settings.rs：set_and_get / get_nonexistent / overwrite — 3 个
- commands/ai.rs：no_api_key / auto_create_category / match_existing_category / account_hint_match / account_hint_fallback_default / no_default_account — 6 个（mock AiHttpClient trait）

### 前端测试
- AiInputDialog.test.tsx：输入→点击→成功关闭 / 失败显示错误

### 不测试
- 真实 AI API 调用 → 手动验收
- reqwest 网络层

## 八、实施顺序

| 阶段 | 内容 | 验证点 |
|---|---|---|
| 1 | Cargo.toml + db/settings.rs + commands/settings.rs + 命令注册 | cargo test settings 用例 |
| 2 | db/mod.rs 加 AiError + services/ai.rs（结构体/trait/纯函数）+ 单测 | cargo test ai services |
| 3 | commands/ai.rs（分类匹配/账户推断/组装）+ 集成测试 | cargo test 全通过 |
| 4 | 前端 types + api.ts + i18n 键 | make check |
| 5 | AiInputDialog.tsx + AppShell/Header 入口 + 快捷键 + 组件测试 | make check + vitest |
| 6 | SettingsPage AI 区块 | make check |
| 7 | 端到端手动验收 | "中午吃饭18元" → 录入餐饮支出 |

## 九、硬规则确认

- 无 schema 变更（复用 settings 表）
- SQL 注入防护（参数化查询）
- 错误处理（thiserror + i18n 键，前端 translateErrorMessage）
- 金额由后端处理（AI 返回字符串 → validate → quick_add_transaction → cents）
- 前端不缓存（useMutation + invalidateQueries）
- 日期处理（AI 返回 YYYY-MM-DD，后端 chrono::NaiveDate 校验）
- API Key 安全（存加密数据库，前端密码框回显）
