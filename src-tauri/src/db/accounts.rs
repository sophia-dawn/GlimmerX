//! Account storage: CRUD, hierarchy paths, metadata, opening balances and
//! transfers.
//!
//! Each area lives in its own submodule; everything public is re-exported here
//! so callers keep using `crate::db::accounts::*`.

mod create;
mod crud;
mod ledger;
mod meta;
mod opening;
mod path;
mod transfer;
mod types;
mod validate;

#[cfg(test)]
mod tests;

// Cross-module but crate-internal: the quick-add transfer branch in
// `db::transactions` shares this rule (design doc 25, P0-1/P0-2).
pub(in crate::db) use validate::validate_endpoint;

pub use create::{batch_create_accounts, create_account_with_path};
#[allow(unused_imports)]
pub use crud::{
    delete_account, find_account, find_account_by_type_and_name, get_account_balance,
    get_account_balances_batch, get_opening_balance_for_account, insert_account, list_accounts,
    list_accounts_filtered, update_account,
};
pub use ledger::get_account_transactions;
#[allow(unused_imports)]
pub use meta::{
    delete_meta_for_account, get_meta_for_account, get_meta_value, insert_meta, set_meta_batch,
};
pub use opening::create_opening_balance;
pub use transfer::create_transfer;
pub use types::{
    AccountMetaRecord, AccountRecord, BatchAccountInput, CreateAccountFullInput, TransactionRecord,
    UpdateAccountParams,
};
