//! Row types, input DTOs and row mappers shared by the transaction module.

/// Internal DB row representation for a transaction.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct TransactionRecord {
    pub id: String,
    pub date: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Internal DB row representation for a posting.
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct PostingRecord {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: i64,
    pub sequence: i32,
    pub created_at: String,
}

/// Transaction with all its postings (for display/API).
#[derive(Debug, Clone, serde::Serialize)]
#[allow(dead_code)]
pub struct TransactionWithPostings {
    pub transaction: TransactionRecord,
    pub postings: Vec<PostingRecord>,
}

/// Input for creating a posting.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PostingInput {
    pub account_id: String,
    pub amount: i64,
}

/// QuickAdd input structure for simplified transaction creation.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct QuickAddInput {
    pub mode: String,
    pub amount: String,
    pub source_account_id: Option<String>,
    pub destination_account_id: Option<String>,
    pub category_id: Option<String>,
    /// Category by name: the db layer derives the type from `mode` and
    /// resolves-or-creates the category (and its mirror account) inside the
    /// same transaction as the posting writes (design doc 25, P0-3).
    /// Only meaningful for expense/income; rejected for transfers.
    pub category_name: Option<String>,
    pub description: Option<String>,
    pub date: Option<String>,
}

/// Transaction detail with full posting information.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TransactionDetail {
    pub id: String,
    pub date: String,
    pub description: String,
    /// Categories live on the postings, so a split transaction has several.
    pub postings: Vec<PostingDetail>,
    pub created_at: String,
    pub updated_at: String,
    pub is_balanced: bool,
    pub posting_count: u32,
    pub debit_total: i64,
    pub credit_total: i64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PostingDetail {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    /// Set when this account is the ledger account of a category, i.e. when the
    /// posting *is* the categorised side of the transaction.
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub amount: i64,
    pub amount_display: String,
    pub is_debit: bool,
    pub sequence: u32,
    pub created_at: String,
}

/// Update transaction input (partial update, all fields optional).
#[derive(Debug, Clone, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct UpdateTransactionInput {
    pub date: Option<String>,
    pub description: Option<String>,
    pub postings: Option<Vec<PostingInput>>,
}

/// Delete preview information.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct DeletePreview {
    pub transaction_id: String,
    pub description: String,
    pub date: String,
    pub posting_count: u32,
    pub can_delete: bool,
    pub warning_message: Option<String>,
}

// ---------------------------------------------------------------------------
// Transaction List Data structures (for paginated list view)
// ---------------------------------------------------------------------------

/// Transaction list item (optimized for frontend display).
///
/// Unlike full TransactionWithPostings:
/// - postings simplified to account name + amount pairs
/// - includes aggregated display_amount (debit total)
/// - category name included for display
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionListItem {
    pub id: String,
    pub date: String,
    pub description: String,
    /// Names of every category this transaction touches, in posting order.
    /// A split transaction legitimately has more than one.
    pub category_names: Vec<String>,
    pub category_icon: Option<String>,
    /// Postings summary: (account_name, amount) pairs
    pub postings_summary: Vec<(String, i64)>,
    /// User-visible amount (debit total, always positive)
    pub display_amount: i64,
    /// Net movement on balance-sheet accounts: negative for a payment,
    /// positive for a receipt, zero for a transfer between own accounts.
    /// `display_amount` cannot distinguish these — it is always >= 0.
    pub flow_amount: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Pagination metadata.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub page_size: u32,
    /// Unknown for infinite-scroll lists; omitted to avoid a full COUNT scan
    /// on every page fetch. Use `has_next` to drive pagination.
    pub total_count: Option<u32>,
    pub total_pages: Option<u32>,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Transaction list response with pagination and date groups.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionListResponse {
    pub items: Vec<TransactionListItem>,
    pub pagination: PaginationInfo,
    /// Date groups with day_total calculated by backend
    pub date_groups: Vec<TransactionDateGroup>,
}

/// Transactions grouped by date (for frontend display).
///
/// **Design constraint**: day_total is calculated by backend,
/// frontend does NOT compute any amounts (AGENTS.md rule).
#[derive(Debug, Clone, serde::Serialize)]
pub struct TransactionDateGroup {
    pub date: String,
    /// Formatted date display (e.g., "2024年3月15日")
    pub date_display: String,
    pub items: Vec<TransactionListItem>,
    /// Day total (sum of display_amount for all items)
    pub day_total: i64,
}

/// Enhanced transaction filter with all filter options and pagination.
#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct TransactionFilter {
    /// Date range filter
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    /// Amount range filter (cents)
    #[allow(dead_code)]
    pub min_amount: Option<i64>,
    #[allow(dead_code)]
    pub max_amount: Option<i64>,
    /// Filter by account (transaction involves this account)
    pub account_id: Option<String>,
    /// Filter by category
    pub category_id: Option<String>,
    /// Description search (LIKE match)
    pub description_query: Option<String>,
    /// Pagination
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// Sorting
    pub sort_by: Option<String>, // "date" | "amount" | "description"
    pub sort_order: Option<String>, // "asc" | "desc"
}

#[allow(dead_code)]
pub(in crate::db::transactions) const TRANSACTION_COLUMNS: &str =
    "id, date, description, created_at, updated_at";

#[allow(dead_code)]
pub(in crate::db::transactions) const POSTING_COLUMNS: &str =
    "id, transaction_id, account_id, amount, sequence, created_at";

#[allow(dead_code)]
pub(in crate::db::transactions) fn row_to_transaction(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<TransactionRecord> {
    Ok(TransactionRecord {
        id: row.get(0)?,
        date: row.get(1)?,
        description: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

#[allow(dead_code)]
pub(in crate::db::transactions) fn row_to_posting(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<PostingRecord> {
    Ok(PostingRecord {
        id: row.get(0)?,
        transaction_id: row.get(1)?,
        account_id: row.get(2)?,
        amount: row.get(3)?,
        sequence: row.get(4)?,
        created_at: row.get(5)?,
    })
}
