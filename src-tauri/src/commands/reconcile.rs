//! Reconciliation commands.
//!
//! The totals are computed on the backend on every selection change: the
//! frontend may render amounts but never adds them up.

use tauri::State;

use crate::db::reconcile::{AutoMatchResult, ReconcilePreview, ReconciliationRecord};
use crate::utils::money::parse_amount_to_cents;
use crate::AppState;

/// Totals shown under the ticking list.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileTotalsDto {
    /// Balance already reconciled before this run.
    pub opening_balance: i64,
    /// Sum of the currently ticked postings.
    pub selected_total: i64,
    /// `opening_balance + selected_total`.
    pub expected_balance: i64,
    /// `expected_balance - closing_balance`; zero means it balances.
    pub difference: i64,
    pub balanced: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileCreateInput {
    pub account_id: String,
    pub statement_date: String,
    /// Decimal amount string, e.g. "-1234.56".
    pub closing_balance: String,
    pub posting_ids: Vec<String>,
}

fn with_conn<T>(
    state: &State<'_, AppState>,
    f: impl FnOnce(&rusqlite::Connection) -> Result<T, crate::db::AppError>,
) -> Result<T, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or(crate::errors::message("errors.databaseLocked"))?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;
    f(&conn).map_err(|e| e.to_string())
}

/// Parse the statement balance, treating an empty field as zero.
///
/// `reconcile_totals` and `reconcile_create` must agree: when only the totals
/// call was lenient, the screen could show a balanced statement that the submit
/// then rejected, with no way to tell which one was telling the truth.
fn parse_closing(raw: &str) -> Result<i64, crate::db::AppError> {
    if raw.trim().is_empty() {
        Ok(0)
    } else {
        parse_amount_to_cents(raw)
    }
}

#[tauri::command]
pub async fn reconcile_preview(
    account_id: String,
    statement_date: String,
    state: State<'_, AppState>,
) -> Result<ReconcilePreview, String> {
    with_conn(&state, |conn| {
        crate::db::reconcile::preview(conn, &account_id, &statement_date)
    })
}

#[tauri::command]
pub async fn reconcile_auto_match(
    account_id: String,
    statement_date: String,
    amounts: Vec<String>,
    state: State<'_, AppState>,
) -> Result<AutoMatchResult, String> {
    with_conn(&state, |conn| {
        let candidates = crate::db::reconcile::candidates(conn, &account_id, &statement_date)?;
        let mut targets = Vec::with_capacity(amounts.len());
        for raw in &amounts {
            targets.push(parse_amount_to_cents(raw)?);
        }
        Ok(crate::db::reconcile::auto_match(
            &candidates,
            &targets,
            &statement_date,
        ))
    })
}

#[tauri::command]
pub async fn reconcile_totals(
    account_id: String,
    statement_date: String,
    closing_balance: String,
    posting_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ReconcileTotalsDto, String> {
    with_conn(&state, |conn| {
        let closing = parse_closing(&closing_balance)?;
        crate::db::reconcile::totals(conn, &account_id, &statement_date, &posting_ids, closing).map(
            |t| ReconcileTotalsDto {
                opening_balance: t.opening_balance,
                selected_total: t.selected_total,
                expected_balance: t.expected_balance,
                difference: t.difference,
                balanced: t.balanced,
            },
        )
    })
}

#[tauri::command]
pub async fn reconcile_create(
    input: ReconcileCreateInput,
    state: State<'_, AppState>,
) -> Result<String, String> {
    with_conn(&state, |conn| {
        let closing = parse_closing(&input.closing_balance)?;
        crate::db::reconcile::create_reconciliation(
            conn,
            &input.account_id,
            &input.statement_date,
            closing,
            &input.posting_ids,
        )
    })
}

#[tauri::command]
pub async fn reconcile_list(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ReconciliationRecord>, String> {
    with_conn(&state, |conn| {
        crate::db::reconcile::list_reconciliations(conn, &account_id)
    })
}

#[tauri::command]
pub async fn reconcile_undo(id: String, state: State<'_, AppState>) -> Result<(), String> {
    with_conn(&state, |conn| {
        crate::db::reconcile::undo_reconciliation(conn, &id)
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    fn mock_app_with_db(
        db: Database,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(AppState {
                database: Mutex::new(Some(db)),
                recent_dbs: Mutex::new(RecentDbs::empty()),
            })
            .invoke_handler(tauri::generate_handler![
                reconcile_preview,
                reconcile_auto_match,
                reconcile_totals,
                reconcile_create,
                reconcile_list,
                reconcile_undo,
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

    struct Seeded {
        account_id: String,
        posting_ids: Vec<String>,
    }

    /// Create two accounts and one balanced two-posting transaction per entry.
    /// Returns the balance-sheet account and its posting ids, in date order.
    fn seed(db: &Database, entries: &[(&str, i64)]) -> Seeded {
        let conn = db.get_conn().unwrap();
        let stamp = crate::utils::time::now_rfc3339();
        let make_account = |name: &str, kind: &str| -> String {
            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO accounts (id, name, type, currency, description,
                                       is_system, is_active, include_net_worth,
                                       created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'CNY', '', 0, 1, 0, ?4, ?4)",
                rusqlite::params![&id, name, kind, &stamp],
            )
            .unwrap();
            id
        };
        let account_id = make_account("银行", "asset");
        let counter = make_account("餐饮", "expense");

        let mut posting_ids = Vec::new();
        for (date, amount) in entries {
            let tx_id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO transactions (id, date, description, created_at, updated_at)
                 VALUES (?1, ?2, 'test', ?3, ?3)",
                rusqlite::params![&tx_id, date, &stamp],
            )
            .unwrap();
            for (account, value, sequence) in [(&account_id, *amount, 0), (&counter, -*amount, 1)] {
                let posting_id = uuid::Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO postings
                         (id, transaction_id, account_id, amount, sequence, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    rusqlite::params![&posting_id, &tx_id, account, value, sequence, &stamp],
                )
                .unwrap();
                if sequence == 0 {
                    posting_ids.push(posting_id);
                }
            }
        }
        Seeded {
            account_id,
            posting_ids,
        }
    }

    #[test]
    fn test_reconcile_roundtrip_via_ipc() {
        let (_dir, db) = test_db();
        let seeded = seed(
            &db,
            &[
                ("2024-01-03", 10_000),
                ("2024-01-05", 20_000),
                ("2024-01-20", 500),
            ],
        );
        let (_app, webview) = mock_app_with_db(db);

        // The 20th is past the statement date, so it must not be offered.
        let preview = invoke(
            &webview,
            "reconcile_preview",
            serde_json::json!({
                "accountId": seeded.account_id,
                "statementDate": "2024-01-10",
            }),
        )
        .expect("preview should succeed");
        assert_eq!(preview["openingBalance"], serde_json::json!(0));
        assert_eq!(preview["candidates"].as_array().unwrap().len(), 2);

        let matched = invoke(
            &webview,
            "reconcile_auto_match",
            serde_json::json!({
                "accountId": seeded.account_id,
                "statementDate": "2024-01-10",
                "amounts": ["100.00", "200.00"],
            }),
        )
        .expect("auto match should succeed");
        assert_eq!(matched["unmatchedTargets"], serde_json::json!(0));
        assert_eq!(matched["matched"].as_array().unwrap().len(), 2);

        let selection = serde_json::json!([seeded.posting_ids[0], seeded.posting_ids[1]]);
        let totals = invoke(
            &webview,
            "reconcile_totals",
            serde_json::json!({
                "accountId": seeded.account_id,
                "statementDate": "2024-01-10",
                "closingBalance": "300.00",
                "postingIds": selection,
            }),
        )
        .expect("totals should succeed");
        assert_eq!(totals["expectedBalance"], serde_json::json!(30_000));
        assert_eq!(totals["difference"], serde_json::json!(0));
        assert_eq!(totals["balanced"], serde_json::json!(true));

        // An empty closing balance is treated as zero, not as a parse error.
        let unchecked = invoke(
            &webview,
            "reconcile_totals",
            serde_json::json!({
                "accountId": seeded.account_id,
                "statementDate": "2024-01-10",
                "closingBalance": "",
                "postingIds": selection,
            }),
        )
        .expect("totals should succeed");
        assert_eq!(unchecked["difference"], serde_json::json!(30_000));
        assert_eq!(unchecked["balanced"], serde_json::json!(false));

        let created = invoke(
            &webview,
            "reconcile_create",
            serde_json::json!({
                "input": {
                    "accountId": seeded.account_id,
                    "statementDate": "2024-01-10",
                    "closingBalance": "300.00",
                    "postingIds": selection,
                }
            }),
        )
        .expect("create should succeed");
        assert!(!created.as_str().unwrap().is_empty());

        let history: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "reconcile_list",
                serde_json::json!({ "accountId": seeded.account_id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0]["closingBalance"], serde_json::json!(30_000));
        assert_eq!(history[0]["openingBalance"], serde_json::json!(0));

        // A second run on the same statement date is rejected.
        let err = invoke(
            &webview,
            "reconcile_create",
            serde_json::json!({
                "input": {
                    "accountId": seeded.account_id,
                    "statementDate": "2024-01-10",
                    "closingBalance": "300.00",
                    "postingIds": selection,
                }
            }),
        )
        .expect_err("duplicate statement date must fail");
        assert!(err.as_str().unwrap().contains("对账单"));

        invoke(
            &webview,
            "reconcile_undo",
            serde_json::json!({ "id": history[0]["id"] }),
        )
        .expect("undo should succeed");
        let history: Vec<serde_json::Value> = serde_json::from_value(
            invoke(
                &webview,
                "reconcile_list",
                serde_json::json!({ "accountId": seeded.account_id }),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(history.is_empty());

        // Undoing twice surfaces the missing-record error.
        let err = invoke(
            &webview,
            "reconcile_undo",
            serde_json::json!({ "id": "gone" }),
        )
        .expect_err("undoing a missing reconciliation must fail");
        assert!(!err.as_str().unwrap().is_empty());
    }

    #[test]
    fn test_reconcile_rejects_stale_or_foreign_selection() {
        let (_dir, db) = test_db();
        let seeded = seed(&db, &[("2024-01-03", 10_000)]);
        let (_app, webview) = mock_app_with_db(db);

        // An empty selection is refused before anything is written.
        let err = invoke(
            &webview,
            "reconcile_create",
            serde_json::json!({
                "input": {
                    "accountId": seeded.account_id,
                    "statementDate": "2024-01-10",
                    "closingBalance": "0",
                    "postingIds": [],
                }
            }),
        )
        .expect_err("empty selection must fail");
        assert!(err.as_str().unwrap().contains("至少"));

        // A posting id that is not open for this account cannot slip in.
        let err = invoke(
            &webview,
            "reconcile_totals",
            serde_json::json!({
                "accountId": seeded.account_id,
                "statementDate": "2024-01-10",
                "closingBalance": "0",
                "postingIds": ["not-a-posting"],
            }),
        )
        .expect_err("unknown posting must fail");
        assert!(!err.as_str().unwrap().is_empty());

        // Unknown accounts are refused up front rather than returning an
        // empty candidate list that looks like a fully reconciled account.
        let err = invoke(
            &webview,
            "reconcile_preview",
            serde_json::json!({ "accountId": "missing", "statementDate": "2024-01-10" }),
        )
        .expect_err("unknown account must fail");
        assert!(err.as_str().unwrap().contains("账户"));
    }
}
