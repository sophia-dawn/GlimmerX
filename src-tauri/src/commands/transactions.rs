use serde::Deserialize;
use tauri::State;

use crate::db::transactions::{
    list_transactions_paginated, PostingInput, PostingRecord, TransactionFilter,
    TransactionListItem, TransactionListResponse, TransactionWithPostings,
};
use crate::AppState;

// ---------------------------------------------------------------------------
// DTO types (serializable for Tauri IPC with camelCase for frontend)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDto {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub is_reconciled: bool,
    pub created_at: String,
    pub updated_at: String,
    pub postings: Vec<PostingDto>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingDto {
    pub id: String,
    pub transaction_id: String,
    pub account_id: String,
    pub amount: i64,
    pub sequence: i32,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Input types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTransactionInput {
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub postings: Vec<CreatePostingInput>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostingInput {
    pub account_id: String,
    /// Decimal amount string (e.g. "35.00"); converted to cents on the backend.
    pub amount: String,
}

/// Command-layer update input; the DB layer receives already-converted
/// (i64 cents) postings.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTransactionInput {
    pub date: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<String>,
    pub postings: Option<Vec<CreatePostingInput>>,
}

/// Convert command-layer posting inputs (decimal strings) into DB-layer
/// postings (cents). The frontend never computes monetary values.
fn to_db_postings(postings: Vec<CreatePostingInput>) -> Result<Vec<PostingInput>, String> {
    postings
        .into_iter()
        .map(|p| {
            Ok(PostingInput {
                account_id: p.account_id,
                amount: crate::db::transactions::parse_amount_to_cents(&p.amount)
                    .map_err(|e| e.to_string())?,
            })
        })
        .collect()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionListFilter {
    pub account_id: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnhancedTransactionFilter {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub min_amount: Option<i64>,
    pub max_amount: Option<i64>,
    pub account_id: Option<String>,
    pub category_id: Option<String>,
    pub description_query: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionListItemDto {
    pub id: String,
    pub date: String,
    pub description: String,
    pub category_id: Option<String>,
    pub category_name: Option<String>,
    pub category_icon: Option<String>,
    pub postings_summary: Vec<PostingSummaryDto>,
    pub display_amount: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostingSummaryDto {
    pub account_name: String,
    pub amount: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationInfoDto {
    pub page: u32,
    pub page_size: u32,
    pub total_count: Option<u32>,
    pub total_pages: Option<u32>,
    pub has_next: bool,
    pub has_prev: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDateGroupDto {
    pub date: String,
    pub date_display: String,
    pub items: Vec<TransactionListItemDto>,
    pub day_total: i64,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionListResponseDto {
    pub items: Vec<TransactionListItemDto>,
    pub pagination: PaginationInfoDto,
    pub date_groups: Vec<TransactionDateGroupDto>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_dto(tx_with_postings: TransactionWithPostings) -> TransactionDto {
    TransactionDto {
        id: tx_with_postings.transaction.id,
        date: tx_with_postings.transaction.date,
        description: tx_with_postings.transaction.description,
        category_id: tx_with_postings.transaction.category_id,
        is_reconciled: tx_with_postings.transaction.is_reconciled,
        created_at: tx_with_postings.transaction.created_at,
        updated_at: tx_with_postings.transaction.updated_at,
        postings: tx_with_postings
            .postings
            .into_iter()
            .map(|p| PostingDto {
                id: p.id,
                transaction_id: p.transaction_id,
                account_id: p.account_id,
                amount: p.amount,
                sequence: p.sequence,
                created_at: p.created_at,
            })
            .collect(),
    }
}

#[allow(dead_code)]
fn posting_record_to_dto(record: PostingRecord) -> PostingDto {
    PostingDto {
        id: record.id,
        transaction_id: record.transaction_id,
        account_id: record.account_id,
        amount: record.amount,
        sequence: record.sequence,
        created_at: record.created_at,
    }
}

fn list_item_to_dto(item: TransactionListItem) -> TransactionListItemDto {
    TransactionListItemDto {
        id: item.id,
        date: item.date,
        description: item.description,
        category_id: item.category_id,
        category_name: item.category_name,
        category_icon: item.category_icon,
        postings_summary: item
            .postings_summary
            .into_iter()
            .map(|(name, amount)| PostingSummaryDto {
                account_name: name,
                amount,
            })
            .collect(),
        display_amount: item.display_amount,
        created_at: item.created_at,
        updated_at: item.updated_at,
    }
}

fn response_to_dto(response: TransactionListResponse) -> TransactionListResponseDto {
    TransactionListResponseDto {
        items: response
            .items
            .iter()
            .cloned()
            .map(list_item_to_dto)
            .collect(),
        pagination: PaginationInfoDto {
            page: response.pagination.page,
            page_size: response.pagination.page_size,
            total_count: response.pagination.total_count,
            total_pages: response.pagination.total_pages,
            has_next: response.pagination.has_next,
            has_prev: response.pagination.has_prev,
        },
        date_groups: response
            .date_groups
            .into_iter()
            .map(|g| TransactionDateGroupDto {
                date: g.date,
                date_display: g.date_display,
                items: g.items.into_iter().map(list_item_to_dto).collect(),
                day_total: g.day_total,
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Tauri Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn transaction_create(
    input: CreateTransactionInput,
    state: State<'_, AppState>,
) -> Result<TransactionDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let postings: Vec<PostingInput> = to_db_postings(input.postings)?;

    let tx_id = crate::db::transactions::create_transaction(
        &tx,
        &input.date,
        &input.description,
        input.category_id.as_deref(),
        &postings,
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;

    let tx_with_postings = crate::db::transactions::get_transaction_with_postings(&conn, &tx_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Created transaction not found".to_string())?;
    Ok(to_dto(tx_with_postings))
}

#[tauri::command]
pub async fn transaction_list(
    filter: Option<TransactionListFilter>,
    state: State<'_, AppState>,
) -> Result<Vec<TransactionDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let transactions = crate::db::transactions::list_transactions(
        &conn,
        filter.as_ref().and_then(|f| f.account_id.as_deref()),
        filter.as_ref().and_then(|f| f.from_date.as_deref()),
        filter.as_ref().and_then(|f| f.to_date.as_deref()),
    )
    .map_err(|e| e.to_string())?;
    Ok(transactions.into_iter().map(to_dto).collect())
}

#[tauri::command]
pub async fn transaction_get(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<TransactionDto>, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx_with_postings = crate::db::transactions::get_transaction_with_postings(&conn, &id)
        .map_err(|e| e.to_string())?;
    Ok(tx_with_postings.map(to_dto))
}

#[tauri::command]
pub async fn transaction_list_paginated(
    filter: Option<EnhancedTransactionFilter>,
    state: State<'_, AppState>,
) -> Result<TransactionListResponseDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let backend_filter = TransactionFilter {
        from_date: filter.as_ref().and_then(|f| f.from_date.clone()),
        to_date: filter.as_ref().and_then(|f| f.to_date.clone()),
        min_amount: filter.as_ref().and_then(|f| f.min_amount),
        max_amount: filter.as_ref().and_then(|f| f.max_amount),
        account_id: filter.as_ref().and_then(|f| f.account_id.clone()),
        category_id: filter.as_ref().and_then(|f| f.category_id.clone()),
        description_query: filter.as_ref().and_then(|f| f.description_query.clone()),
        page: filter.as_ref().and_then(|f| f.page),
        page_size: filter.as_ref().and_then(|f| f.page_size),
        sort_by: filter.as_ref().and_then(|f| f.sort_by.clone()),
        sort_order: filter.as_ref().and_then(|f| f.sort_order.clone()),
    };

    let response =
        list_transactions_paginated(&conn, &backend_filter).map_err(|e| e.to_string())?;

    Ok(response_to_dto(response))
}

// ---------------------------------------------------------------------------
// Quick Add Command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn quick_add_transaction(
    input: crate::db::transactions::QuickAddInput,
    state: State<'_, AppState>,
) -> Result<TransactionDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    let tx_with_postings =
        crate::db::transactions::quick_add_transaction(&conn, &input).map_err(|e| e.to_string())?;

    Ok(to_dto(tx_with_postings))
}

// ---------------------------------------------------------------------------
// Transaction Detail Command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn transaction_detail(
    id: String,
    state: State<'_, AppState>,
) -> Result<crate::db::transactions::TransactionDetail, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::transactions::get_transaction_detail(&conn, &id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Transaction Update Command
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn transaction_update(
    id: String,
    input: UpdateTransactionInput,
    state: State<'_, AppState>,
) -> Result<crate::db::transactions::TransactionDetail, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    let db_input = crate::db::transactions::UpdateTransactionInput {
        date: input.date,
        description: input.description,
        category_id: input.category_id,
        postings: input
            .postings
            .map(to_db_postings)
            .transpose()
            .map_err(|e| e.to_string())?,
    };

    crate::db::transactions::update_transaction(&mut conn, &id, &db_input)
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Transaction Delete Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn transaction_delete_preview(
    id: String,
    state: State<'_, AppState>,
) -> Result<crate::db::transactions::DeletePreview, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::transactions::get_delete_preview(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transaction_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or("Database is not unlocked. Please unlock it first.")?;
    let mut conn = db.get_conn().map_err(|e| e.to_string())?;

    crate::db::transactions::delete_transaction(&mut conn, &id).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::transactions::TransactionRecord;

    fn make_transaction_record(id: &str) -> TransactionRecord {
        TransactionRecord {
            id: id.to_string(),
            date: "2024-01-15".to_string(),
            description: "Test transaction".to_string(),
            category_id: Some("cat-123".to_string()),
            is_reconciled: false,
            deleted_at: None,
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
        }
    }

    fn make_posting_record(tx_id: &str, account_id: &str, amount: i64, seq: i32) -> PostingRecord {
        PostingRecord {
            id: format!("posting-{}", seq),
            transaction_id: tx_id.to_string(),
            account_id: account_id.to_string(),
            amount,
            sequence: seq,
            created_at: "2024-01-15T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_to_dto_conversion() {
        let tx_record = make_transaction_record("tx-123");
        let postings = vec![
            make_posting_record("tx-123", "acc-1", -1000, 0),
            make_posting_record("tx-123", "acc-2", 1000, 1),
        ];

        let tx_with_postings = TransactionWithPostings {
            transaction: tx_record,
            postings,
        };

        let dto = to_dto(tx_with_postings);

        assert_eq!(dto.id, "tx-123");
        assert_eq!(dto.date, "2024-01-15");
        assert_eq!(dto.description, "Test transaction");
        assert_eq!(dto.category_id, Some("cat-123".to_string()));
        assert!(!dto.is_reconciled);
        assert_eq!(dto.created_at, "2024-01-15T10:00:00Z");
        assert_eq!(dto.updated_at, "2024-01-15T10:00:00Z");
        assert_eq!(dto.postings.len(), 2);
    }

    #[test]
    fn test_posting_dto_conversion() {
        let record = make_posting_record("tx-123", "acc-1", -5000, 0);
        let dto = posting_record_to_dto(record);

        assert_eq!(dto.id, "posting-0");
        assert_eq!(dto.transaction_id, "tx-123");
        assert_eq!(dto.account_id, "acc-1");
        assert_eq!(dto.amount, -5000);
        assert_eq!(dto.sequence, 0);
        assert_eq!(dto.created_at, "2024-01-15T10:00:00Z");
    }

    #[test]
    fn test_to_dto_preserves_postings_order() {
        let tx_record = make_transaction_record("tx-order");
        let postings = vec![
            make_posting_record("tx-order", "acc-1", -100, 0),
            make_posting_record("tx-order", "acc-2", 50, 1),
            make_posting_record("tx-order", "acc-3", 50, 2),
        ];

        let tx_with_postings = TransactionWithPostings {
            transaction: tx_record,
            postings,
        };

        let dto = to_dto(tx_with_postings);

        assert_eq!(dto.postings[0].sequence, 0);
        assert_eq!(dto.postings[1].sequence, 1);
        assert_eq!(dto.postings[2].sequence, 2);
        assert_eq!(dto.postings[0].account_id, "acc-1");
        assert_eq!(dto.postings[1].account_id, "acc-2");
        assert_eq!(dto.postings[2].account_id, "acc-3");
    }

    #[test]
    fn test_to_dto_empty_postings() {
        let tx_record = make_transaction_record("tx-empty");
        let postings = vec![];

        let tx_with_postings = TransactionWithPostings {
            transaction: tx_record,
            postings,
        };

        let dto = to_dto(tx_with_postings);

        assert_eq!(dto.id, "tx-empty");
        assert!(dto.postings.is_empty());
    }

    #[test]
    fn test_to_dto_null_category_id() {
        let tx_record = TransactionRecord {
            id: "tx-nocat".to_string(),
            date: "2024-01-15".to_string(),
            description: "No category".to_string(),
            category_id: None,
            is_reconciled: false,
            deleted_at: None,
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
        };
        let postings = vec![make_posting_record("tx-nocat", "acc-1", -100, 0)];

        let tx_with_postings = TransactionWithPostings {
            transaction: tx_record,
            postings,
        };

        let dto = to_dto(tx_with_postings);

        assert_eq!(dto.category_id, None);
    }

    #[test]
    fn test_dto_camel_case_serialization() {
        let dto = TransactionDto {
            id: "tx-123".to_string(),
            date: "2024-01-15".to_string(),
            description: "Test".to_string(),
            category_id: None,
            is_reconciled: true,
            created_at: "2024-01-15T10:00:00Z".to_string(),
            updated_at: "2024-01-15T10:00:00Z".to_string(),
            postings: vec![],
        };

        let json = serde_json::to_string(&dto).unwrap();

        // Verify camelCase keys
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"date\""));
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"categoryId\"")); // camelCase
        assert!(json.contains("\"isReconciled\"")); // camelCase
        assert!(json.contains("\"createdAt\"")); // camelCase
        assert!(json.contains("\"updatedAt\"")); // camelCase
        assert!(json.contains("\"postings\""));
    }

    #[test]
    fn test_posting_dto_camel_case_serialization() {
        let dto = PostingDto {
            id: "posting-1".to_string(),
            transaction_id: "tx-1".to_string(),
            account_id: "acc-1".to_string(),
            amount: -1000,
            sequence: 0,
            created_at: "2024-01-15T10:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&dto).unwrap();

        // Verify camelCase keys
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"transactionId\"")); // camelCase
        assert!(json.contains("\"accountId\"")); // camelCase
        assert!(json.contains("\"amount\""));
        assert!(json.contains("\"sequence\""));
        assert!(json.contains("\"createdAt\"")); // camelCase
    }

    #[test]
    fn test_create_input_deserialization() {
        let json = r#"{
            "date": "2024-01-15",
            "description": "Grocery shopping",
            "categoryId": "cat-123",
            "postings": [
                {"accountId": "acc-1", "amount": "-500.00"},
                {"accountId": "acc-2", "amount": "500.00"}
            ]
        }"#;

        let input: CreateTransactionInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.date, "2024-01-15");
        assert_eq!(input.description, "Grocery shopping");
        assert_eq!(input.category_id, Some("cat-123".to_string()));
        assert_eq!(input.postings.len(), 2);
        assert_eq!(input.postings[0].account_id, "acc-1");
        assert_eq!(input.postings[0].amount, "-500.00");
        assert_eq!(input.postings[1].account_id, "acc-2");
        assert_eq!(input.postings[1].amount, "500.00");
    }

    #[test]
    fn test_create_input_null_category() {
        let json = r#"{
            "date": "2024-01-15",
            "description": "No category",
            "categoryId": null,
            "postings": [
                {"accountId": "acc-1", "amount": "-100.00"}
            ]
        }"#;

        let input: CreateTransactionInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.category_id, None);
    }

    #[test]
    fn test_create_input_missing_category() {
        let json = r#"{
            "date": "2024-01-15",
            "description": "Missing category",
            "postings": [
                {"accountId": "acc-1", "amount": "-100.00"}
            ]
        }"#;

        let input: CreateTransactionInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.category_id, None);
    }

    #[test]
    fn test_filter_deserialization() {
        let json = r#"{
            "accountId": "acc-123",
            "fromDate": "2024-01-01",
            "toDate": "2024-01-31"
        }"#;

        let filter: TransactionListFilter = serde_json::from_str(json).unwrap();

        assert_eq!(filter.account_id, Some("acc-123".to_string()));
        assert_eq!(filter.from_date, Some("2024-01-01".to_string()));
        assert_eq!(filter.to_date, Some("2024-01-31".to_string()));
    }

    #[test]
    fn test_filter_partial_fields() {
        let json = r#"{
            "accountId": "acc-123"
        }"#;

        let filter: TransactionListFilter = serde_json::from_str(json).unwrap();

        assert_eq!(filter.account_id, Some("acc-123".to_string()));
        assert_eq!(filter.from_date, None);
        assert_eq!(filter.to_date, None);
    }

    #[test]
    fn test_filter_empty_all_none() {
        let json = r#"{}"#;

        let filter: TransactionListFilter = serde_json::from_str(json).unwrap();

        assert_eq!(filter.account_id, None);
        assert_eq!(filter.from_date, None);
        assert_eq!(filter.to_date, None);
    }

    #[test]
    fn test_to_db_postings_converts_decimal_strings() {
        let postings = to_db_postings(vec![
            CreatePostingInput {
                account_id: "acc-1".to_string(),
                amount: "-35.50".to_string(),
            },
            CreatePostingInput {
                account_id: "acc-2".to_string(),
                amount: "35.50".to_string(),
            },
        ])
        .unwrap();

        assert_eq!(postings.len(), 2);
        assert_eq!(postings[0].account_id, "acc-1");
        assert_eq!(postings[0].amount, -3550);
        assert_eq!(postings[1].account_id, "acc-2");
        assert_eq!(postings[1].amount, 3550);
    }

    #[test]
    fn test_to_db_postings_rejects_invalid_amount() {
        let err = to_db_postings(vec![CreatePostingInput {
            account_id: "acc-1".to_string(),
            amount: "abc".to_string(),
        }])
        .expect_err("invalid amount string must fail");
        assert!(err.contains("errors.transaction.invalidAmount"));

        // Empty list is valid (unbalanced validation happens later)
        assert!(to_db_postings(vec![]).unwrap().is_empty());
    }
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::accounts::create_account_with_path;
    use crate::db::categories::{create_category, CategoryType};
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn mock_app(
        state: AppState,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                transaction_create,
                transaction_list,
                transaction_get,
                transaction_list_paginated,
                quick_add_transaction,
                transaction_detail,
                transaction_update,
                transaction_delete_preview,
                transaction_delete,
            ])
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        (app, webview)
    }

    fn invoke(
        webview: &tauri::WebviewWindow<tauri::test::MockRuntime>,
        cmd: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, serde_json::Value> {
        let req = tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        tauri::test::get_ipc_response(webview, req)
            .map(|b| b.deserialize::<serde_json::Value>().unwrap())
    }

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    fn seed_accounts(db: &Database) -> (String, String, String) {
        let conn = db.get_conn().unwrap();
        let cash = create_account_with_path(&conn, "Assets/Cash", "CNY", None).unwrap();
        let food_account = create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
        let food_category = create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();
        (cash, food_account, food_category)
    }

    #[test]
    fn test_transaction_commands_via_ipc() {
        let (_dir, db) = test_db();
        let (cash, food_account, food_category) = seed_accounts(&db);
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(Some(db)),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        // create with decimal string amounts
        let created = invoke(
            &webview,
            "transaction_create",
            serde_json::json!({
                "input": {
                    "date": "2024-01-15",
                    "description": "Lunch",
                    "categoryId": food_category,
                    "postings": [
                        { "accountId": cash, "amount": "-35.00" },
                        { "accountId": food_account, "amount": "35.00" }
                    ]
                }
            }),
        )
        .expect("transaction_create should succeed");
        let tx_id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["date"], serde_json::json!("2024-01-15"));
        assert_eq!(created["postings"].as_array().unwrap().len(), 2);
        assert_eq!(created["postings"][0]["amount"], serde_json::json!(-3500));

        // get by id
        let fetched = invoke(
            &webview,
            "transaction_get",
            serde_json::json!({ "id": tx_id }),
        )
        .expect("transaction_get should succeed");
        assert_eq!(fetched["description"], serde_json::json!("Lunch"));

        // get missing -> null
        let missing = invoke(
            &webview,
            "transaction_get",
            serde_json::json!({ "id": "missing" }),
        )
        .expect("transaction_get missing should return null");
        assert!(missing.is_null());

        // list with filter
        let list = invoke(
            &webview,
            "transaction_list",
            serde_json::json!({
                "filter": { "accountId": cash, "fromDate": "2024-01-01", "toDate": "2024-01-31" }
            }),
        )
        .expect("transaction_list should succeed");
        assert_eq!(list.as_array().unwrap().len(), 1);

        // list without filter
        let all = invoke(&webview, "transaction_list", serde_json::json!({}))
            .expect("transaction_list without filter should succeed");
        assert_eq!(all.as_array().unwrap().len(), 1);

        // paginated list
        let paginated = invoke(
            &webview,
            "transaction_list_paginated",
            serde_json::json!({
                "filter": {
                    "fromDate": "2024-01-01",
                    "toDate": "2024-01-31",
                    "minAmount": 0,
                    "maxAmount": 100000,
                    "accountId": cash,
                    "categoryId": food_category,
                    "descriptionQuery": "Lunch",
                    "page": 1,
                    "pageSize": 10,
                    "sortBy": "date",
                    "sortOrder": "desc"
                }
            }),
        )
        .expect("transaction_list_paginated should succeed");
        assert_eq!(paginated["items"].as_array().unwrap().len(), 1);
        assert_eq!(paginated["pagination"]["hasNext"], serde_json::json!(false));

        // paginated with no filter
        let paginated_all = invoke(
            &webview,
            "transaction_list_paginated",
            serde_json::json!({}),
        )
        .expect("transaction_list_paginated without filter should succeed");
        assert_eq!(paginated_all["items"].as_array().unwrap().len(), 1);

        // detail
        let detail = invoke(
            &webview,
            "transaction_detail",
            serde_json::json!({ "id": tx_id }),
        )
        .expect("transaction_detail should succeed");
        assert_eq!(detail["isBalanced"], serde_json::json!(true));
        assert_eq!(detail["postingCount"], serde_json::json!(2));

        // update description
        let updated = invoke(
            &webview,
            "transaction_update",
            serde_json::json!({
                "id": tx_id,
                "input": { "description": "Lunch updated", "date": "2024-01-16" }
            }),
        )
        .expect("transaction_update should succeed");
        assert_eq!(updated["description"], serde_json::json!("Lunch updated"));

        // update postings
        invoke(
            &webview,
            "transaction_update",
            serde_json::json!({
                "id": tx_id,
                "input": {
                    "postings": [
                        { "accountId": cash, "amount": "-50.00" },
                        { "accountId": food_account, "amount": "50.00" }
                    ]
                }
            }),
        )
        .expect("transaction_update postings should succeed");

        // delete preview
        let preview = invoke(
            &webview,
            "transaction_delete_preview",
            serde_json::json!({ "id": tx_id }),
        )
        .expect("transaction_delete_preview should succeed");
        assert_eq!(preview["postingCount"], serde_json::json!(2));

        // delete
        invoke(
            &webview,
            "transaction_delete",
            serde_json::json!({ "id": tx_id }),
        )
        .expect("transaction_delete should succeed");
        let list_after = invoke(&webview, "transaction_list", serde_json::json!({}))
            .expect("transaction_list after delete should succeed");
        assert!(list_after.as_array().unwrap().is_empty());
    }

    #[test]
    fn test_quick_add_via_ipc() {
        let (_dir, db) = test_db();
        let (cash, _food_account, food_category) = seed_accounts(&db);
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(Some(db)),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        let created = invoke(
            &webview,
            "quick_add_transaction",
            serde_json::json!({
                "input": {
                    "mode": "expense",
                    "amount": "35.00",
                    "sourceAccountId": cash,
                    "categoryId": food_category,
                    "description": "Quick lunch",
                    "date": "2024-02-01"
                }
            }),
        )
        .expect("quick_add_transaction should succeed");
        assert_eq!(created["description"], serde_json::json!("Quick lunch"));
        assert_eq!(created["postings"].as_array().unwrap().len(), 2);

        let err = invoke(
            &webview,
            "quick_add_transaction",
            serde_json::json!({
                "input": { "mode": "bogus", "amount": "1.00" }
            }),
        )
        .expect_err("invalid quick add mode must fail");
        assert!(err.as_str().unwrap().contains("invalidMode"));
    }

    #[test]
    fn test_transaction_create_invalid_amount() {
        let (_dir, db) = test_db();
        let (cash, food_account, _food_category) = seed_accounts(&db);
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(Some(db)),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        let err = invoke(
            &webview,
            "transaction_create",
            serde_json::json!({
                "input": {
                    "date": "2024-01-15",
                    "description": "Bad",
                    "categoryId": null,
                    "postings": [
                        { "accountId": cash, "amount": "abc" },
                        { "accountId": food_account, "amount": "10.00" }
                    ]
                }
            }),
        )
        .expect_err("invalid posting amount must fail");
        assert!(err.as_str().unwrap().contains("invalidAmount"));
    }

    #[test]
    fn test_transaction_commands_locked_db() {
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(None),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        let bodies: Vec<(&str, serde_json::Value)> = vec![
            (
                "transaction_create",
                serde_json::json!({
                    "input": {
                        "date": "2024-01-15",
                        "description": "x",
                        "categoryId": null,
                        "postings": []
                    }
                }),
            ),
            ("transaction_list", serde_json::json!({})),
            ("transaction_get", serde_json::json!({ "id": "x" })),
            ("transaction_list_paginated", serde_json::json!({})),
            (
                "quick_add_transaction",
                serde_json::json!({
                    "input": { "mode": "expense", "amount": "1.00" }
                }),
            ),
            ("transaction_detail", serde_json::json!({ "id": "x" })),
            ("transaction_update", serde_json::json!({ "id": "x", "input": {} })),
            ("transaction_delete_preview", serde_json::json!({ "id": "x" })),
            ("transaction_delete", serde_json::json!({ "id": "x" })),
        ];
        for (cmd, body) in bodies {
            let err = invoke(&webview, cmd, body)
                .expect_err("locked db must fail for every transaction command");
            assert!(
                err.as_str().unwrap().contains("not unlocked"),
                "unexpected error for {}: {}",
                cmd,
                err.as_str().unwrap()
            );
        }
    }
}
