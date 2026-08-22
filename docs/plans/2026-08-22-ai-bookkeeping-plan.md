# AI 记账功能 Implementation Plan

**Goal:** 用户输入自然语言，AI 自动录入交易
**Design doc:** `docs/plans/2026-08-22-ai-bookkeeping-design.md`

## Tasks (TDD, frequent commits)

### Phase 1: Backend settings layer
- [ ] T1: Add `reqwest` to `Cargo.toml`
- [ ] T2: `db/settings.rs` (get/set_setting + tests) + `AppError::AiError` in `db/mod.rs`
- [ ] T3: `commands/settings.rs` (get/set_setting commands) + register in `lib.rs`

### Phase 2: AI service layer
- [ ] T4: `services/ai.rs` — structs + validate_mode/amount/date + parse_json_response + tests
- [ ] T5: `services/ai.rs` — build_system_prompt + parse_transaction + test_connection (HTTP)

### Phase 3: AI command layer
- [ ] T6: `commands/ai.rs` — resolve_category + resolve_source_account + ai_parse_transaction + ai_test_connection + register

### Phase 4: Frontend types + API + i18n
- [ ] T7: `types/index.ts` (AiConfig) + `api.ts` (aiParseTransaction/aiTestConnection/getSetting/setSetting)
- [ ] T8: i18n keys in `en.json` + `zh.json` (ai domain + errors.ai sub-domain)

### Phase 5: Frontend UI
- [ ] T9: `AiInputDialog.tsx` component (+ textarea if missing)
- [ ] T10: Integrate into `AppShell.tsx` (state + shortcut Ctrl+Shift+I) + `Header.tsx` (Sparkles button)
- [ ] T11: `AiSettingsSection.tsx` + integrate into `SettingsPage.tsx`

### Phase 6: Tests
- [ ] T12: `AiInputDialog.test.tsx` (Vitest)

### Phase 7: Verification
- [ ] T13: `make check` (tsc+eslint+prettier) + `cargo test` + `cargo clippy`

## Key Patterns
- Settings stored in existing `settings` table (key-value, no schema change)
- SQL: parameterized queries (`?1` placeholders)
- Errors: `AppError::AiError("keySuffix")` → `#[error("errors.ai.{0}")]` → frontend `translateErrorMessage`
- Amount: AI returns string → backend validates → `quick_add_transaction` converts to cents
- API Key: stored in encrypted DB, not localStorage
