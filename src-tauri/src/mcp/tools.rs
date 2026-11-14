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
