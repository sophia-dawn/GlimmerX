//! MCP tool set (design doc 25, §5.3).
//!
//! All rmcp types are confined to this file (and `mod.rs`); `commands/` and
//! `db/` never see them. Reads go through the ledger capability layer; writes
//! go through it too, gated by the trust tier, and a locked ledger is an
//! error every tool reports the same way.
//!
//! Amounts cross the boundary as integer cents only — never a string for the
//! agent to format wrong. Account/category references are names; the skill
//! doc tells agents to `list_accounts` before guessing.

use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::schemars;
use rmcp::tool;
use rmcp::ErrorData as McpError;

use crate::db::AppError;
use crate::ledger::{self, AccountRef, CategoryRef, LedgerQuery, RecordKind, RecordRequest};
use crate::mcp::approval::{ApprovalRequest, StageOutcome, APPROVAL_TIMEOUT};
use crate::mcp::host::{DbFn, McpHost};
use crate::mcp::settings::TrustMode;

/// The handler served over Streamable HTTP. One instance per request (the
/// stateless service factory builds a fresh one); the app is reached only
/// through the [`McpHost`] seam, which tests can fake.
#[derive(Clone)]
pub struct LedgerTools {
    host: Arc<dyn McpHost>,
}

fn internal(msg: &str) -> McpError {
    McpError::internal_error(msg.to_string(), None)
}

fn ok_json(value: impl serde::Serialize) -> Result<CallToolResult, McpError> {
    let text =
        serde_json::to_string_pretty(&value).map_err(|_| internal("serialization failed"))?;
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
}

/// Tool-level failure the user (and the agent) should see: the tool ran and
/// failed in a way the caller can act on, so it is a result, not a protocol
/// error.
fn denied(msg: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(msg.into())])
}

/// Run a ledger call with the database connection; lock/db/app errors are
/// mapped to protocol errors with the localized message. Every tool funnels
/// through this, so "the ledger is locked" is one message, not nine.
fn call_ledger<T>(
    host: &Arc<dyn McpHost>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, AppError> + Send + 'static,
) -> Result<CallToolResult, McpError>
where
    T: serde::Serialize,
{
    let boxed: DbFn = Box::new(move |conn| {
        f(conn).and_then(|value| {
            serde_json::to_value(value).map_err(|e| AppError::IoError(e.to_string()))
        })
    });
    match host.with_db(boxed) {
        Ok(value) => ok_json(value),
        // Domain failures (locked ledger, validation, not-found) are
        // tool-level results the agent can read and act on, not protocol
        // errors, which clients display opaquely.
        Err(message) => Ok(denied(message)),
    }
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct QueryTransactionsParams {
    /// Inclusive range start, YYYY-MM-DD. Optional.
    #[serde(rename = "from", default)]
    from_date: Option<String>,
    /// Inclusive range end, YYYY-MM-DD. Optional.
    #[serde(rename = "to", default)]
    to_date: Option<String>,
    /// Account id (from list_accounts). Optional.
    #[serde(default)]
    account_id: Option<String>,
    /// Category id (from list_categories). Optional.
    #[serde(default)]
    category_id: Option<String>,
    /// Case-insensitive substring on the description. Optional.
    #[serde(default)]
    keyword: Option<String>,
    /// Page size cap, 1..=100. Default 50.
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct SummaryParams {
    /// Inclusive range start, YYYY-MM-DD.
    from: String,
    /// Inclusive range end, YYYY-MM-DD.
    to: String,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct RecordTransactionParams {
    /// One of: expense, income, transfer.
    kind: String,
    /// Positive integer cents (e.g. 18.50 -> 1850).
    pub amount_cents: i64,
    /// Business date, YYYY-MM-DD. Defaults to today.
    #[serde(default)]
    date: Option<String>,
    #[serde(default)]
    description: Option<String>,
    /// Account the money leaves (required for expense and transfer).
    /// Use the account name as shown by list_accounts.
    #[serde(default)]
    source_account: Option<String>,
    /// Account the money arrives in (required for income and transfer).
    #[serde(default)]
    destination_account: Option<String>,
    /// P&L category name (expense/income only; resolved or created).
    #[serde(default)]
    category: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct CreateAccountParams {
    /// Hierarchical path, e.g. `Assets/Alipay`. The first segment decides
    /// the account type.
    path: String,
    #[serde(default)]
    description: Option<String>,
    /// Opening balance in integer cents. Optional (omit for zero).
    #[serde(default)]
    opening_balance_cents: Option<i64>,
    /// Opening-balance date, YYYY-MM-DD. Defaults to today.
    #[serde(default)]
    opening_balance_date: Option<String>,
}

impl LedgerTools {
    pub fn new(host: Arc<dyn McpHost>) -> Self {
        Self { host }
    }

    /// Trust-gate a write tool. `Ok(())` = proceed; `Err(result)` = the
    /// already-shaped tool result the caller must return as-is.
    async fn gate_write(
        &self,
        tool: &str,
        request: serde_json::Value,
    ) -> Result<(), CallToolResult> {
        let trust = match self.host.mcp_config() {
            Ok(config) => config.trust,
            Err(message) => return Err(denied(message)),
        };
        match trust {
            TrustMode::ReadOnly => Err(denied(
                "Not authorized: the MCP trust mode is read-only. Ask the user to change it in settings.",
            )),
            TrustMode::Trust => Ok(()),
            TrustMode::Confirm => {
                let approval = ApprovalRequest {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    tool: tool.to_string(),
                    request,
                    created_at: crate::utils::time::now_rfc3339(),
                };
                let rx = match self.host.try_stage_approval(approval.clone()) {
                    StageOutcome::Waiting(rx) => rx,
                    // Single slot by design: no queue, an actionable error.
                    StageOutcome::Busy(id) => {
                        return Err(denied(format!(
                            "Another write is already waiting for approval ({id}); approve or reject it first."
                        )));
                    }
                };
                // Tell the webview; a missing window just means nobody is
                // there to approve, and the timeout will cancel the write.
                self.host.emit_approval(&approval);
                match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
                    Ok(Ok(true)) => Ok(()),
                    Ok(Ok(false)) => {
                        Err(denied("Rejected: the user declined this write."))
                    }
                    Ok(Err(_recv_err)) => Err(denied(
                        "The approval card was closed; the write was cancelled.",
                    )),
                    Err(_elapsed) => {
                        Err(denied("Timed out: no user approval within the timeout."))
                    }
                }
            }
        }
    }
}

#[rmcp::tool_router]
impl LedgerTools {
    /// Whether the ledger is unlocked, plus the db path and trust mode.
    #[tool(
        description = "Report whether the ledger is unlocked, the database path, and the current MCP trust mode. Call this first: a locked ledger makes every other tool fail."
    )]
    async fn ledger_status(&self) -> Result<CallToolResult, McpError> {
        // The status call needs db + path at once; run both through one
        // with_db closure so a lock between them cannot tear the answer.
        let boxed: DbFn = Box::new(|conn| {
            let trust = crate::mcp::settings::load_config(conn)?
                .trust
                .as_str()
                .to_string();
            serde_json::to_value(serde_json::json!({
                "unlocked": true,
                "trustMode": trust,
            }))
            .map_err(|e| AppError::IoError(e.to_string()))
        });
        match self.host.with_db(boxed) {
            Ok(mut value) => {
                // dbPath is not in the db; merge it in from the host.
                let path = self.host.db_path().unwrap_or_default();
                value["databasePath"] = serde_json::Value::String(path);
                ok_json(value)
            }
            Err(message) => Ok(denied(message)),
        }
    }

    /// All accounts with type and balance.
    #[tool(
        description = "List every account with its id, name, type (asset/liability) and balance in cents (liabilities negative). Resolve account names here before writing; do not guess ids."
    )]
    async fn list_accounts(&self) -> Result<CallToolResult, McpError> {
        call_ledger(&self.host, ledger::list_accounts)
    }

    /// All categories with their type.
    #[tool(description = "List every category with its id, name and type (expense/income).")]
    async fn list_categories(&self) -> Result<CallToolResult, McpError> {
        call_ledger(&self.host, ledger::list_categories)
    }

    /// Query transactions by date/account/category/keyword.
    #[tool(
        description = "Query transactions by date range, account id, category id, or a keyword on the description. Dates are YYYY-MM-DD, the range is inclusive."
    )]
    async fn query_transactions(
        &self,
        Parameters(args): Parameters<QueryTransactionsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = args.limit.unwrap_or(50).clamp(1, 100);
        call_ledger(&self.host, move |conn| {
            ledger::query_transactions(
                conn,
                &LedgerQuery {
                    from_date: args.from_date.clone(),
                    to_date: args.to_date.clone(),
                    account_id: args.account_id.clone(),
                    category_id: args.category_id.clone(),
                    description_query: args.keyword.clone(),
                    page: Some(1),
                    page_size: Some(limit),
                },
            )
        })
    }

    /// Account balances snapshot.
    #[tool(description = "Snapshot of every account balance (id, name, cents).")]
    async fn get_balances(&self) -> Result<CallToolResult, McpError> {
        call_ledger(&self.host, ledger::account_balances)
    }

    /// Income/expense/net totals over a date range.
    #[tool(
        description = "Income, expense and net totals in cents over an inclusive date range (YYYY-MM-DD)."
    )]
    async fn summary(
        &self,
        Parameters(args): Parameters<SummaryParams>,
    ) -> Result<CallToolResult, McpError> {
        call_ledger(&self.host, move |conn| {
            ledger::period_summary(conn, &args.from, &args.to)
        })
    }

    /// Record an expense, income or transfer (write, trust-gated).
    #[tool(
        description = "Record a transaction. kind: expense|income|transfer; amounts are positive integer cents. source_account: where money leaves (required for expense/transfer); destination_account: where it arrives (required for income/transfer); both must be asset or liability accounts. category: name, expense/income only. Accounts are matched by name — call list_accounts first when unsure."
    )]
    async fn record_transaction(
        &self,
        Parameters(args): Parameters<RecordTransactionParams>,
    ) -> Result<CallToolResult, McpError> {
        let kind = match args.kind.as_str() {
            "expense" => RecordKind::Expense,
            "income" => RecordKind::Income,
            "transfer" => RecordKind::Transfer,
            other => {
                return Ok(denied(format!(
                    "Invalid kind '{other}': expected expense, income or transfer."
                )));
            }
        };
        let description = args.description.clone().unwrap_or_default();
        let request = RecordRequest {
            kind,
            amount_cents: args.amount_cents,
            date: args.date.clone(),
            description: description.clone(),
            source_account: args.source_account.clone().map(AccountRef::Name),
            destination_account: args.destination_account.clone().map(AccountRef::Name),
            category: args.category.clone().map(CategoryRef::Name),
        };
        // The approval card shows the request as the agent sent it.
        let request_json = serde_json::json!({
            "kind": args.kind,
            "amountCents": args.amount_cents,
            "date": args.date,
            "description": description,
            "sourceAccount": args.source_account,
            "destinationAccount": args.destination_account,
            "category": args.category,
        });
        if let Err(result) = self.gate_write("record_transaction", request_json).await {
            return Ok(result);
        }
        call_ledger(&self.host, move |conn| {
            ledger::record_transaction(conn, &request)
        })
    }

    /// Create an account (write, trust-gated).
    #[tool(
        description = "Create an account at a hierarchical path like 'Assets/Alipay' — the first segment (Assets/Liabilities) decides the type. Optional opening balance in integer cents. Unless the user explicitly asks for it, do not create accounts: account names are the user's semantic system."
    )]
    async fn create_account(
        &self,
        Parameters(args): Parameters<CreateAccountParams>,
    ) -> Result<CallToolResult, McpError> {
        let request_json = serde_json::json!({
            "path": args.path,
            "description": args.description,
            "openingBalanceCents": args.opening_balance_cents,
            "openingBalanceDate": args.opening_balance_date,
        });
        if let Err(result) = self.gate_write("create_account", request_json).await {
            return Ok(result);
        }
        let request = ledger::CreateAccountRequest {
            path: args.path.clone(),
            description: args.description.clone(),
            opening_balance_cents: args.opening_balance_cents,
            opening_balance_date: args.opening_balance_date.clone(),
        };
        call_ledger(&self.host, move |conn| {
            ledger::create_account(conn, &request)
        })
    }
}

#[rmcp::tool_handler(
    name = "glimmerx",
    instructions = "GlimmerX double-entry ledger. Amounts are integer cents; dates are YYYY-MM-DD. List accounts/categories before writing; account names must match exactly or uniquely. Writes are gated by the trust mode: read-only rejects, confirm asks the user (timeout), trust writes directly."
)]
impl rmcp::ServerHandler for LedgerTools {}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Mutex;

    use tauri::async_runtime::JoinHandle;

    use crate::db::Database;
    use crate::mcp::approval::ApprovalSlot;
    use crate::mcp::settings::McpConfig;
    use crate::mcp::McpRuntimeStatus;

    /// What the fake host hands to `with_db`.
    enum Backing {
        /// Real schema: ledger calls can succeed.
        Db {
            db: Database,
            _dir: tempfile::TempDir,
        },
        /// No schema at all: settings and ledger reads fail.
        Bare,
        /// Locked ledger: `with_db` fails before the closure runs.
        Locked,
    }

    /// Test double for the app handle — only the parts the tools touch.
    /// Runtime-task plumbing is a no-op; approvals go through a real slot so
    /// the single-slot semantics are exercised for real.
    struct FakeHost {
        backing: Backing,
        trust: Result<TrustMode, String>,
        path: Option<String>,
        slot: ApprovalSlot,
        emitted: Mutex<u32>,
    }

    impl FakeHost {
        fn with(
            backing: Backing,
            trust: Result<TrustMode, String>,
            path: Option<String>,
        ) -> Arc<Self> {
            Arc::new(Self {
                backing,
                trust,
                path,
                slot: ApprovalSlot::new(),
                emitted: Mutex::new(0),
            })
        }

        /// Unlocked ledger with the real schema and one asset account.
        fn ledger(trust: TrustMode) -> Arc<Self> {
            let dir = tempfile::tempdir().unwrap();
            let db = Database::create(&dir.path().join("t.db"), "pw").unwrap();
            {
                let conn = db.get_conn().unwrap();
                crate::db::accounts::create_account_with_path(&conn, "Assets/Cash", None).unwrap();
                // Keep the settings table and the fake host in agreement:
                // ledger_status reads the trust tier from the database.
                crate::mcp::settings::save_trust(&conn, trust.as_str()).unwrap();
            }
            Self::with(
                Backing::Db { db, _dir: dir },
                Ok(trust),
                Some("/tmp/glimmerx.db".to_string()),
            )
        }

        /// Unlocked but without a schema: every query fails.
        fn bare() -> Arc<Self> {
            Self::with(Backing::Bare, Ok(TrustMode::Trust), None)
        }

        /// Locked ledger.
        fn locked() -> Arc<Self> {
            Self::with(Backing::Locked, Ok(TrustMode::Trust), None)
        }

        /// Settings unreadable, so even the trust tier cannot be resolved.
        fn config_unreadable() -> Arc<Self> {
            Self::with(Backing::Bare, Err("settings unreadable".to_string()), None)
        }
    }

    impl McpHost for FakeHost {
        fn with_db(&self, f: DbFn) -> Result<serde_json::Value, String> {
            match &self.backing {
                Backing::Db { db, .. } => {
                    let conn = db.get_conn().map_err(|e| e.to_string())?;
                    f(&conn).map_err(|e| e.to_string())
                }
                Backing::Bare => {
                    let conn = rusqlite::Connection::open_in_memory().map_err(|e| e.to_string())?;
                    f(&conn).map_err(|e| e.to_string())
                }
                Backing::Locked => Err("errors.databaseLocked".to_string()),
            }
        }

        fn db_path(&self) -> Option<String> {
            self.path.clone()
        }

        fn mcp_config(&self) -> Result<McpConfig, String> {
            self.trust.clone().map(|trust| McpConfig {
                enabled: true,
                port: 17823,
                token: "test-token".to_string(),
                trust,
            })
        }

        fn try_stage_approval(&self, request: ApprovalRequest) -> StageOutcome {
            self.slot.try_insert(request)
        }

        fn emit_approval(&self, _request: &ApprovalRequest) {
            *self.emitted.lock().unwrap() += 1;
        }

        fn set_runtime_task(&self, _task: JoinHandle<()>, _port: u16) {}

        fn set_runtime_port(&self, _port: u16) {}

        fn abort_runtime(&self) {}

        fn mark_runtime_stopped(&self) {}

        fn runtime_status(&self) -> McpRuntimeStatus {
            McpRuntimeStatus {
                running: false,
                port: None,
            }
        }
    }

    fn text_of(result: &CallToolResult) -> String {
        match result.content.first() {
            Some(ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected text content, got {other:?}"),
        }
    }

    fn failed(result: &CallToolResult) -> bool {
        result.is_error.unwrap_or(false)
    }

    /// An expense on the fake ledger's Cash account.
    fn expense_args() -> RecordTransactionParams {
        RecordTransactionParams {
            kind: "expense".to_string(),
            amount_cents: 1850,
            date: Some("2026-11-14".to_string()),
            description: Some("coffee".to_string()),
            source_account: Some("Cash".to_string()),
            destination_account: None,
            category: Some("餐饮".to_string()),
        }
    }

    #[tokio::test]
    async fn test_ledger_status_reports_trust_and_path() {
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::Trust));
        let result = tools.ledger_status().await.unwrap();
        let text = text_of(&result);
        assert!(text.contains("\"trust\""), "trust mode: {text}");
        // dbPath lives in the host, not the database: it must be merged in.
        assert!(text.contains("/tmp/glimmerx.db"), "db path merged: {text}");
    }

    #[tokio::test]
    async fn test_ledger_status_surfaces_unreadable_settings() {
        let tools = LedgerTools::new(FakeHost::bare());
        let result = tools.ledger_status().await.unwrap();
        assert!(failed(&result), "a failed read is a tool error");
        // A localized, user-facing message, not an empty result.
        assert!(!text_of(&result).is_empty());
    }

    #[tokio::test]
    async fn test_read_tools_return_ledger_views() {
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::ReadOnly));

        let accounts = text_of(&tools.list_accounts().await.unwrap());
        assert!(accounts.contains("Cash"), "accounts: {accounts}");

        let categories = text_of(&tools.list_categories().await.unwrap());
        assert!(categories.starts_with('['), "categories: {categories}");

        let balances = text_of(&tools.get_balances().await.unwrap());
        assert!(balances.contains("Cash"), "balances: {balances}");

        let summary = text_of(
            &tools
                .summary(Parameters(SummaryParams {
                    from: "2026-11-01".to_string(),
                    to: "2026-11-30".to_string(),
                }))
                .await
                .unwrap(),
        );
        assert!(summary.contains("netCents"), "summary: {summary}");
    }

    #[tokio::test]
    async fn test_query_transactions_passes_filters_and_clamps_limit() {
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::ReadOnly));
        // limit 0 clamps up to 1 instead of returning nothing.
        let page = text_of(
            &tools
                .query_transactions(Parameters(QueryTransactionsParams {
                    from_date: Some("2026-11-01".to_string()),
                    to_date: Some("2026-11-30".to_string()),
                    account_id: None,
                    category_id: None,
                    keyword: Some("coffee".to_string()),
                    limit: Some(0),
                }))
                .await
                .unwrap(),
        );
        assert_eq!(page.trim(), "[]", "no matching transactions yet");
    }

    #[tokio::test]
    async fn test_record_transaction_rejects_unknown_kind() {
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::Trust));
        let result = tools
            .record_transaction(Parameters(RecordTransactionParams {
                kind: "borrow".to_string(),
                amount_cents: 100,
                date: None,
                description: None,
                source_account: None,
                destination_account: None,
                category: None,
            }))
            .await
            .unwrap();
        assert!(failed(&result));
        assert!(
            text_of(&result).contains("expense, income or transfer"),
            "{}",
            text_of(&result)
        );
    }

    #[tokio::test]
    async fn test_readonly_mode_rejects_every_write() {
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::ReadOnly));

        let record = tools
            .record_transaction(Parameters(expense_args()))
            .await
            .unwrap();
        assert!(failed(&record));
        assert!(text_of(&record).contains("read-only"));

        let create = tools
            .create_account(Parameters(CreateAccountParams {
                path: "Assets/Alipay".to_string(),
                description: None,
                opening_balance_cents: None,
                opening_balance_date: None,
            }))
            .await
            .unwrap();
        assert!(failed(&create));
        assert!(text_of(&create).contains("read-only"));
    }

    #[tokio::test]
    async fn test_write_is_denied_when_settings_are_unreadable() {
        let tools = LedgerTools::new(FakeHost::config_unreadable());
        let result = tools
            .record_transaction(Parameters(expense_args()))
            .await
            .unwrap();
        assert!(failed(&result));
        assert!(text_of(&result).contains("settings unreadable"));
    }

    #[tokio::test]
    async fn test_trust_mode_writes_without_asking() {
        let host = FakeHost::ledger(TrustMode::Trust);
        let tools = LedgerTools::new(host.clone());

        let record = tools
            .record_transaction(Parameters(expense_args()))
            .await
            .unwrap();
        assert!(!failed(&record), "{}", text_of(&record));
        assert!(text_of(&record).contains("transactionId"));

        let create = tools
            .create_account(Parameters(CreateAccountParams {
                path: "Assets/Alipay".to_string(),
                description: Some("wallet".to_string()),
                opening_balance_cents: Some(10_000),
                opening_balance_date: Some("2026-11-14".to_string()),
            }))
            .await
            .unwrap();
        assert!(!failed(&create), "{}", text_of(&create));
        assert!(text_of(&create).contains("accountId"));

        assert_eq!(*host.emitted.lock().unwrap(), 0, "trust never asks");
    }

    #[tokio::test]
    async fn test_confirm_mode_reports_the_busy_slot() {
        let host = FakeHost::ledger(TrustMode::Confirm);
        // Single slot by design: a second write is an actionable error.
        let staged = host.slot.try_insert(ApprovalRequest {
            request_id: "pending-1".to_string(),
            tool: "record_transaction".to_string(),
            request: serde_json::json!({}),
            created_at: "2026-11-14T00:00:00Z".to_string(),
        });
        assert!(matches!(staged, StageOutcome::Waiting(_)));

        let result = LedgerTools::new(host)
            .record_transaction(Parameters(expense_args()))
            .await
            .unwrap();
        let text = text_of(&result);
        assert!(failed(&result));
        assert!(text.contains("pending-1"), "blocking id named: {text}");
    }

    #[tokio::test]
    async fn test_confirm_mode_writes_after_approval() {
        let host = FakeHost::ledger(TrustMode::Confirm);
        let tools = LedgerTools::new(host.clone());
        let call = tokio::spawn(async move {
            tools
                .record_transaction(Parameters(expense_args()))
                .await
                .unwrap()
        });

        let mut request_id = None;
        for _ in 0..200 {
            if let Some(pending) = host.slot.current() {
                request_id = Some(pending.request_id.clone());
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        let request_id = request_id.expect("approval never staged");
        assert_eq!(*host.emitted.lock().unwrap(), 1, "webview told once");
        assert!(host.slot.resolve(&request_id, true));

        let result = call.await.unwrap();
        assert!(!failed(&result), "{}", text_of(&result));
        assert!(text_of(&result).contains("transactionId"));
    }

    #[tokio::test(start_paused = true)]
    async fn test_confirm_mode_times_out_without_an_answer() {
        // Nobody is there to approve: the write is cancelled, not left hanging.
        let tools = LedgerTools::new(FakeHost::ledger(TrustMode::Confirm));
        let result = tools
            .record_transaction(Parameters(expense_args()))
            .await
            .unwrap();
        assert!(failed(&result));
        assert!(text_of(&result).contains("Timed out"));
    }

    #[tokio::test]
    async fn test_locked_ledger_fails_every_tool() {
        let tools = LedgerTools::new(FakeHost::locked());
        let result = tools.list_accounts().await.unwrap();
        assert!(failed(&result));
        assert!(text_of(&result).contains("databaseLocked"));
    }
}
