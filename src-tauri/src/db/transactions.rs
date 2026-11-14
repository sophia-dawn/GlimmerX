//! Transaction storage: creation, quick-add, listing, updates and deletes.
//!
//! Each area lives in its own submodule; everything public is re-exported here
//! so callers keep using `crate::db::transactions::*`.

mod create;
mod delete;
mod paginate;
mod query;
mod quick_add;
mod types;
mod update;
mod validate;

#[cfg(test)]
mod tests;

pub use crate::utils::money::parse_amount_to_cents;

pub use create::create_transaction;
pub use delete::{delete_transaction, get_delete_preview};
pub use paginate::list_transactions_paginated;
pub use query::{get_transaction_detail, get_transaction_with_postings, list_transactions};
pub use quick_add::quick_add_transaction;
#[allow(unused_imports)]
pub use types::{
    DeletePreview, PaginationInfo, PostingDetail, PostingInput, PostingRecord, QuickAddInput,
    TransactionDateGroup, TransactionDetail, TransactionFilter, TransactionListItem,
    TransactionListResponse, TransactionRecord, TransactionWithPostings, UpdateTransactionInput,
};
pub use update::update_transaction;
