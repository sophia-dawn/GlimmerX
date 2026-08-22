use tauri::State;

use crate::db::reports::{
    self, AccountBalanceTrendReportDto, AccountTransactionsReportDto, AuditReportDto,
    BalanceSheetReportDto, CategoryBreakdownReportDto, MonthComparisonReportDto, ReportFilter,
    StandardReportDto, TrendReportDto, YearSummaryReportDto,
};
use crate::AppState;

#[tauri::command]
pub async fn report_standard(
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<StandardReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_standard_report(&conn, &filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_month_comparison(
    month1: String,
    month2: String,
    state: State<'_, AppState>,
) -> Result<MonthComparisonReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_month_comparison(&conn, &month1, &month2).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_category_breakdown(
    filter: ReportFilter,
    income_or_expense: String,
    state: State<'_, AppState>,
) -> Result<CategoryBreakdownReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_category_breakdown_report(&conn, &filter, &income_or_expense)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_balance_sheet(
    snapshot_date: String,
    state: State<'_, AppState>,
) -> Result<BalanceSheetReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_balance_sheet_report(&conn, &snapshot_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_trend(
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<TrendReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_trend_report(&conn, &filter).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_year_summary(
    year: i32,
    state: State<'_, AppState>,
) -> Result<YearSummaryReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_year_summary_report(&conn, year).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_account_transactions(
    account_id: String,
    filter: ReportFilter,
    page: u32,
    page_size: u32,
    state: State<'_, AppState>,
) -> Result<AccountTransactionsReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_account_transactions_report(&conn, &account_id, &filter, page, page_size)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_account_balance_trend(
    account_id: String,
    filter: ReportFilter,
    state: State<'_, AppState>,
) -> Result<AccountBalanceTrendReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_account_balance_trend_report(&conn, &account_id, &filter)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn report_audit(state: State<'_, AppState>) -> Result<AuditReportDto, String> {
    let db_state = state.database.lock().map_err(|e| e.to_string())?;
    let db = db_state
        .as_ref()
        .ok_or_else(|| "Database is not unlocked. Please unlock it first.".to_string())?;
    let conn = db.get_conn().map_err(|e| e.to_string())?;

    reports::get_audit_report(&conn).map_err(|e| e.to_string())
}

#[cfg(test)]
mod ipc_tests {
    use super::*;
    use crate::db::accounts::{create_account_with_path, CreateAccountFullInput};
    use crate::db::categories::{create_category, CategoryType};
    use crate::db::transactions::{create_transaction, PostingInput};
    use crate::db::{Database, RecentDbs};
    use std::sync::Mutex;
    use tempfile::TempDir;

    const ALL_COMMANDS: &[&str] = &[
        "report_standard",
        "report_month_comparison",
        "report_category_breakdown",
        "report_balance_sheet",
        "report_trend",
        "report_year_summary",
        "report_account_transactions",
        "report_account_balance_trend",
        "report_audit",
    ];

    fn mock_app(
        state: AppState,
    ) -> (
        tauri::App<tauri::test::MockRuntime>,
        tauri::WebviewWindow<tauri::test::MockRuntime>,
    ) {
        let app = tauri::test::mock_builder()
            .manage(state)
            .invoke_handler(tauri::generate_handler![
                report_standard,
                report_month_comparison,
                report_category_breakdown,
                report_balance_sheet,
                report_trend,
                report_year_summary,
                report_account_transactions,
                report_account_balance_trend,
                report_audit,
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

    fn filter_json() -> serde_json::Value {
        serde_json::json!({
            "dateRangePreset": "custom",
            "startDate": "2024-01-01",
            "endDate": "2024-01-31",
            "periodGranularity": "daily",
            "accountIds": null,
            "categoryIds": null
        })
    }

    fn seed_data(db: &Database) -> (String, String) {
        let conn = db.get_conn().unwrap();
        let cash = create_account_with_path(
            &conn,
            "Assets/Cash",
            "CNY",
            Some(&CreateAccountFullInput {
                description: None,
                account_number: None,
                iban: None,
                is_active: true,
                include_net_worth: true,
            }),
        )
        .unwrap();
        let credit_card =
            create_account_with_path(&conn, "Liabilities/CreditCard", "CNY", None).unwrap();
        let salary_account = create_account_with_path(&conn, "Income/Salary", "CNY", None).unwrap();
        let food_account = create_account_with_path(&conn, "Expenses/Food", "CNY", None).unwrap();
        let transport_account =
            create_account_with_path(&conn, "Expenses/Transport", "CNY", None).unwrap();

        let salary_category =
            create_category(&conn, "Salary", &CategoryType::Income, None).unwrap();
        let food_category = create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();

        create_transaction(
            &conn,
            "2024-01-05",
            "Salary Jan",
            Some(&salary_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: 100000,
                },
                PostingInput {
                    account_id: salary_account.clone(),
                    amount: -100000,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-01-10",
            "Lunch",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -3500,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 3500,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-01-20",
            "Metro",
            None,
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -1200,
                },
                PostingInput {
                    account_id: transport_account.clone(),
                    amount: 1200,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-01-25",
            "Online",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: credit_card.clone(),
                    amount: -8000,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 8000,
                },
            ],
        )
        .unwrap();
        create_transaction(
            &conn,
            "2024-02-10",
            "Dinner",
            Some(&food_category),
            &[
                PostingInput {
                    account_id: cash.clone(),
                    amount: -5000,
                },
                PostingInput {
                    account_id: food_account.clone(),
                    amount: 5000,
                },
            ],
        )
        .unwrap();
        (cash, credit_card)
    }

    fn test_db() -> (TempDir, Database) {
        let dir = TempDir::new().unwrap();
        let db = Database::create(&dir.path().join("test.db"), "secret").unwrap();
        (dir, db)
    }

    #[test]
    fn test_report_commands_via_ipc() {
        let (_dir, db) = test_db();
        let (cash, _credit_card) = seed_data(&db);
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(Some(db)),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        let standard = invoke(
            &webview,
            "report_standard",
            serde_json::json!({ "filter": filter_json() }),
        )
        .expect("report_standard should succeed");
        assert_eq!(standard["periodIncome"], serde_json::json!(100000));
        assert_eq!(standard["periodExpense"], serde_json::json!(12700));
        assert_eq!(standard["accountChanges"].as_array().unwrap().len(), 2);

        let comparison = invoke(
            &webview,
            "report_month_comparison",
            serde_json::json!({ "month1": "2024-01", "month2": "2024-02" }),
        )
        .expect("report_month_comparison should succeed");
        assert_eq!(comparison["month1Income"], serde_json::json!(100000));
        assert_eq!(comparison["month1Expense"], serde_json::json!(12700));
        assert_eq!(comparison["month2Expense"], serde_json::json!(5000));

        let breakdown = invoke(
            &webview,
            "report_category_breakdown",
            serde_json::json!({
                "filter": filter_json(),
                "incomeOrExpense": "expense"
            }),
        )
        .expect("report_category_breakdown should succeed");
        assert_eq!(breakdown["totalAmount"], serde_json::json!(11500));
        assert_eq!(breakdown["categories"].as_array().unwrap().len(), 1);

        let balance_sheet = invoke(
            &webview,
            "report_balance_sheet",
            serde_json::json!({ "snapshotDate": "2024-01-31" }),
        )
        .expect("report_balance_sheet should succeed");
        assert_eq!(balance_sheet["totalAssets"], serde_json::json!(95300));
        assert_eq!(balance_sheet["totalLiabilities"], serde_json::json!(-8000));
        assert_eq!(balance_sheet["netWorth"], serde_json::json!(87300));

        let trend = invoke(
            &webview,
            "report_trend",
            serde_json::json!({ "filter": filter_json() }),
        )
        .expect("report_trend should succeed");
        assert_eq!(trend["totalIncome"], serde_json::json!(100000));
        assert_eq!(trend["totalExpense"], serde_json::json!(12700));

        let year_summary = invoke(
            &webview,
            "report_year_summary",
            serde_json::json!({ "year": 2024 }),
        )
        .expect("report_year_summary should succeed");
        assert_eq!(year_summary["totalIncome"], serde_json::json!(100000));
        assert_eq!(year_summary["totalExpense"], serde_json::json!(17700));

        let account_txns = invoke(
            &webview,
            "report_account_transactions",
            serde_json::json!({
                "accountId": cash,
                "filter": filter_json(),
                "page": 1,
                "pageSize": 10
            }),
        )
        .expect("report_account_transactions should succeed");
        assert_eq!(account_txns["totalCount"], serde_json::json!(3));
        assert_eq!(account_txns["totalInflow"], serde_json::json!(100000));

        let balance_trend = invoke(
            &webview,
            "report_account_balance_trend",
            serde_json::json!({
                "accountId": cash,
                "filter": filter_json()
            }),
        )
        .expect("report_account_balance_trend should succeed");
        assert_eq!(balance_trend["granularity"], serde_json::json!("daily"));
        assert_eq!(balance_trend["dataPoints"].as_array().unwrap().len(), 3);

        let audit = invoke(&webview, "report_audit", serde_json::json!({}))
            .expect("report_audit should succeed");
        assert_eq!(
            audit["balanceCheck"]["totalTransactions"],
            serde_json::json!(5)
        );
        assert_eq!(
            audit["balanceCheck"]["unbalancedCount"],
            serde_json::json!(0)
        );
    }

    #[test]
    fn test_report_commands_locked_db() {
        let (_app, webview) = mock_app(AppState {
            database: Mutex::new(None),
            recent_dbs: Mutex::new(RecentDbs::empty()),
        });

        let bodies: Vec<(&str, serde_json::Value)> = vec![
            (
                "report_standard",
                serde_json::json!({ "filter": filter_json() }),
            ),
            (
                "report_month_comparison",
                serde_json::json!({ "month1": "2024-01", "month2": "2024-02" }),
            ),
            (
                "report_category_breakdown",
                serde_json::json!({ "filter": filter_json(), "incomeOrExpense": "expense" }),
            ),
            (
                "report_balance_sheet",
                serde_json::json!({ "snapshotDate": "2024-01-31" }),
            ),
            (
                "report_trend",
                serde_json::json!({ "filter": filter_json() }),
            ),
            ("report_year_summary", serde_json::json!({ "year": 2024 })),
            (
                "report_account_transactions",
                serde_json::json!({
                    "accountId": "missing",
                    "filter": filter_json(),
                    "page": 1,
                    "pageSize": 10
                }),
            ),
            (
                "report_account_balance_trend",
                serde_json::json!({ "accountId": "missing", "filter": filter_json() }),
            ),
            ("report_audit", serde_json::json!({})),
        ];
        assert_eq!(bodies.len(), ALL_COMMANDS.len());
        for (cmd, body) in bodies {
            let err = invoke(&webview, cmd, body)
                .expect_err("locked db must fail for every report command");
            assert!(
                err.as_str().unwrap().contains("not unlocked"),
                "unexpected error for {}: {}",
                cmd,
                err.as_str().unwrap()
            );
        }
    }
}
