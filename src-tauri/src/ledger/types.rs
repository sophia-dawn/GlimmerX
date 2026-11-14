//! Types of the ledger capability layer (design doc 25, §3).
//!
//! Everything that crosses this boundary is either a plain view (read) or an
//! explicit request (write). Amounts are integer cents; account references may
//! be an id or a name — names are resolved by [`super::resolve`] and a failed
//! resolution is an error, never a silent fallback to a default account.

use crate::db::categories::CategoryType;

/// The three quick-add record kinds the capability layer can write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Expense,
    Income,
    Transfer,
}

impl RecordKind {
    /// Wire form shared with the quick-add db layer.
    pub fn as_str(self) -> &'static str {
        match self {
            RecordKind::Expense => "expense",
            RecordKind::Income => "income",
            RecordKind::Transfer => "transfer",
        }
    }

    /// The P&L category type this kind posts to; transfers carry none.
    pub fn category_type(self) -> Option<CategoryType> {
        match self {
            RecordKind::Expense => Some(CategoryType::Expense),
            RecordKind::Income => Some(CategoryType::Income),
            RecordKind::Transfer => None,
        }
    }
}

/// Reference to an account by id or by name.
///
/// [`AccountRef::Name`] resolution follows the four rules in design doc 25
/// (§10.3): exact case-insensitive hit; unique contains hit with a warning;
/// ambiguous or missing names are errors. There is no default-account
/// fallback.
#[derive(Debug, Clone)]
pub enum AccountRef {
    Id(String),
    Name(String),
}

/// Reference to a category by id or by name. Name resolution goes through
/// `resolve_or_create` inside the quick-add transaction (P0-3).
#[derive(Debug, Clone)]
pub enum CategoryRef {
    Id(String),
    Name(String),
}

/// A single write request into the ledger.
///
/// `amount_cents` is an integer; the boundary never accepts string or float
/// amounts (the decimal-string needs of the quick-add db layer are an internal
/// detail handled in [`super::write`]).
#[derive(Debug, Clone)]
pub struct RecordRequest {
    pub kind: RecordKind,
    pub amount_cents: i64,
    /// Business date, `YYYY-MM-DD`; `None` means today (local timezone).
    pub date: Option<String>,
    pub description: String,
    /// Where the money leaves. Required for expense and transfer.
    pub source_account: Option<AccountRef>,
    /// Where the money arrives. Required for income and transfer.
    pub destination_account: Option<AccountRef>,
    /// P&L category, only meaningful for expense/income.
    pub category: Option<CategoryRef>,
}

/// A posting, enriched with account identity for preview surfaces
/// (the proposal confirmation card, MCP tool results).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingView {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    /// Signed amount in cents, exactly as it posts.
    pub amount: i64,
}

/// Result of [`super::write::record_transaction`].
///
/// Write operations never return `void`: the outcome carries the derived
/// postings so a caller can preview what landed, plus every warning for a
/// guess the layer made on the caller's behalf ("账户「微信」按名称匹配到
/// 「微信钱包」", "分类「吃」是新建的").
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerOutcome {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub postings: Vec<PostingView>,
    pub warnings: Vec<String>,
}

/// Request to create an account (design doc 25, §3.2 / §5.3).
///
/// `path` is a hierarchical path like `Assets/支付宝`; the first segment
/// decides the account type, so no separate `kind` field exists — a caller
/// that disagrees with the path root is a caller bug.
#[derive(Debug, Clone)]
pub struct CreateAccountRequest {
    pub path: String,
    pub description: Option<String>,
    /// Opening balance in cents; `None` or `Some(0)` writes no opening entry.
    pub opening_balance_cents: Option<i64>,
    /// Opening-balance business date, `YYYY-MM-DD`; `None` means today.
    pub opening_balance_date: Option<String>,
}

/// Result of [`super::write::create_account`].
///
/// Same discipline as [`LedgerOutcome`]: no `void`, derived postings when an
/// opening balance was written, warnings for anything guessed.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountOutcome {
    pub account_id: String,
    /// The canonical path, e.g. `Assets/支付宝`.
    pub account_path: String,
    /// `asset` / `liability` — derived from the path root.
    pub account_type: String,
    /// Transaction id of the opening-balance entry, if one was written.
    pub opening_transaction_id: Option<String>,
    pub opening_postings: Vec<PostingView>,
    pub warnings: Vec<String>,
}

/// Account as seen from the capability layer: identity plus balance.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub account_type: String,
    pub is_active: bool,
    pub include_net_worth: bool,
    /// Balance in cents (liabilities are negative).
    pub balance_cents: i64,
}

/// Category as seen from the capability layer.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryView {
    pub id: String,
    pub name: String,
    pub category_type: CategoryType,
    pub icon: Option<String>,
}

/// Account balance snapshot.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceView {
    pub account_id: String,
    pub account_name: String,
    pub balance_cents: i64,
}

/// Income/expense/net totals over a date range (reuses `db::reports`).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryView {
    pub start_date: String,
    pub end_date: String,
    pub income_cents: i64,
    pub expense_cents: i64,
    pub net_cents: i64,
}

/// Transaction query for [`super::read::query_transactions`].
#[derive(Debug, Clone, Default)]
pub struct LedgerQuery {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    /// Transaction involves this account.
    pub account_id: Option<String>,
    /// Transaction touches this category.
    pub category_id: Option<String>,
    /// LIKE match on the description.
    pub description_query: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}
