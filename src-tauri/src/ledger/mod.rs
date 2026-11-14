//! The ledger capability layer (design doc 25, §3 / §10.3).
//!
//! The single write entry into the book. Three disciplines, enforced in
//! review:
//!
//! 1. This layer **writes no SQL** — it only calls existing `db::` functions
//!    (`quick_add_transaction`, `create_account_with_path`,
//!    `create_opening_balance`, reports). The double-entry balance, the
//!    single-transaction boundary and the category mirror-account invariant
//!    are inherited, not re-implemented.
//! 2. This layer is the **only** place allowed to write the book; future
//!    features must not write from `commands/` directly.
//! 3. Write operations **never return `void`** — they return
//!    [`LedgerOutcome`] / [`AccountOutcome`] with derived postings and every
//!    "guessed on your behalf" warning.
//!
//! Two drivers consume this layer: the in-app AI (P2, discriminated-union
//! proposals) and the MCP server (P3, typed tools). They share the db layer
//! through this module but never share orchestration.

mod read;
mod resolve;
#[cfg(test)]
mod tests;
mod types;
mod write;

pub use read::{
    account_balances, list_accounts, list_categories, period_summary, query_transactions,
};
pub use resolve::resolve_account;
pub use types::{
    AccountOutcome, AccountRef, AccountView, BalanceView, CategoryRef, CategoryView,
    CreateAccountRequest, LedgerOutcome, LedgerQuery, PostingView, RecordKind, RecordRequest,
    SummaryView,
};
pub use write::{create_account, record_transaction};
