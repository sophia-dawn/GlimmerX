//! Tests for every report query.

use super::common::{get_period_format, resolve_date_range};

use super::*;
use crate::db::accounts::{create_account_with_path, CreateAccountFullInput};
use crate::db::categories::{account_id_for_category, create_category, CategoryType};
use crate::db::transactions::{create_transaction, PostingInput};
use crate::utils::time::*;
use rusqlite::params;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    crate::db::init_schema(&conn).unwrap();
    conn
}

#[test]
fn test_resolve_date_range_current_month() {
    let filter = ReportFilter {
        date_range_preset: DateRangePreset::CurrentMonth,
        start_date: None,
        end_date: None,
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let (start, end) = resolve_date_range(&filter);
    assert!(start.len() == 10);
    assert!(end.len() == 10);
}

#[test]
fn test_resolve_date_range_custom() {
    let filter = ReportFilter {
        date_range_preset: DateRangePreset::Custom,
        start_date: Some("2024-01-01".to_string()),
        end_date: Some("2024-01-31".to_string()),
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let (start, end) = resolve_date_range(&filter);
    assert_eq!(start, "2024-01-01");
    assert_eq!(end, "2024-01-31");
}

#[test]
fn test_get_standard_report_empty_db() {
    let conn = setup_test_db();
    let filter = ReportFilter {
        date_range_preset: DateRangePreset::CurrentMonth,
        start_date: None,
        end_date: None,
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let result = get_standard_report(&conn, &filter);
    assert!(result.is_ok());
    let dto = result.unwrap();
    assert_eq!(dto.period_income, 0);
    assert_eq!(dto.period_expense, 0);
}

#[test]
fn test_get_trend_report_empty_db() {
    let conn = setup_test_db();
    let filter = ReportFilter {
        date_range_preset: DateRangePreset::CurrentMonth,
        start_date: None,
        end_date: None,
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let result = get_trend_report(&conn, &filter);
    assert!(result.is_ok());
    let dto = result.unwrap();
    assert_eq!(dto.granularity, "monthly");
    assert_eq!(dto.data_points.len(), 0);
}

#[test]
fn test_get_category_breakdown_report_empty_db() {
    let conn = setup_test_db();
    let filter = ReportFilter {
        date_range_preset: DateRangePreset::CurrentMonth,
        start_date: None,
        end_date: None,
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let result = get_category_breakdown_report(&conn, &filter, "expense");
    match result {
        Ok(dto) => {
            assert_eq!(dto.total_amount, 0);
            assert_eq!(dto.categories.len(), 0);
        }
        Err(e) => {
            panic!("get_category_breakdown_report returned error: {}", e);
        }
    }
}

struct SeededIds {
    cash: String,
    credit_card: String,
    food_account: String,
    food_category: String,
    transport_category: String,
}

fn net_worth_input() -> CreateAccountFullInput {
    CreateAccountFullInput {
        description: None,
        account_number: None,
        iban: None,
        is_active: true,
        include_net_worth: true,
    }
}

fn seed_data(conn: &Connection) -> SeededIds {
    let cash = create_account_with_path(conn, "Assets/Cash", Some(&net_worth_input())).unwrap();
    let credit_card =
        create_account_with_path(conn, "Liabilities/CreditCard", Some(&net_worth_input())).unwrap();

    // Every P&L account now belongs to a category, so the fixture takes the
    // account from the category instead of creating a second one next to it —
    // a standalone "Expenses/Food" account would carry the postings while the
    // category reports looked at an empty, different account.
    let salary_category = create_category(conn, "Salary", &CategoryType::Income, None).unwrap();
    let food_category = create_category(conn, "Food", &CategoryType::Expense, None).unwrap();
    let transport_category =
        create_category(conn, "Transport", &CategoryType::Expense, None).unwrap();
    let salary_account = account_id_for_category(conn, &salary_category).unwrap();
    let food_account = account_id_for_category(conn, &food_category).unwrap();
    let transport_account = account_id_for_category(conn, &transport_category).unwrap();

    // 2024-01-05 salary of 100000 cents (1000.00)
    create_transaction(
        conn,
        "2024-01-05",
        "Salary Jan",
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
    // 2024-01-10 lunch 3500
    create_transaction(
        conn,
        "2024-01-10",
        "Lunch",
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
    // 2024-01-20 metro 1200
    create_transaction(
        conn,
        "2024-01-20",
        "Metro",
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
    // 2024-01-25 credit card purchase 8000 (liability credit posting)
    create_transaction(
        conn,
        "2024-01-25",
        "Online",
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
    // 2024-02-10 dinner 5000
    create_transaction(
        conn,
        "2024-02-10",
        "Dinner",
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
    // Unbalanced transaction (single posting on cash) for audit coverage
    conn.execute(
            "INSERT INTO transactions (id, date, description, created_at, updated_at)
             VALUES ('tx-unbalanced', '2024-01-30', 'Broken', '2024-01-30T00:00:00+08:00', '2024-01-30T00:00:00+08:00')",
            [],
        )
        .unwrap();
    conn.execute(
        "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
             VALUES ('p-unbalanced', 'tx-unbalanced', ?1, 100, 0, '2024-01-30T00:00:00+08:00')",
        params![cash],
    )
    .unwrap();

    SeededIds {
        cash,
        credit_card,
        food_account,
        food_category,
        transport_category,
    }
}

fn custom_filter(start: &str, end: &str) -> ReportFilter {
    ReportFilter {
        date_range_preset: DateRangePreset::Custom,
        start_date: Some(start.to_string()),
        end_date: Some(end.to_string()),
        period_granularity: PeriodGranularity::Daily,
        account_ids: None,
        category_ids: None,
    }
}

#[test]
fn test_display_and_period_format() {
    assert_eq!(PeriodGranularity::Daily.to_string(), "daily");
    assert_eq!(PeriodGranularity::Weekly.to_string(), "weekly");
    assert_eq!(PeriodGranularity::Monthly.to_string(), "monthly");
    assert_eq!(PeriodGranularity::Yearly.to_string(), "yearly");
    assert_eq!(get_period_format(&PeriodGranularity::Daily), "%Y-%m-%d");
    assert_eq!(get_period_format(&PeriodGranularity::Weekly), "%Y-%W");
    assert_eq!(get_period_format(&PeriodGranularity::Monthly), "%Y-%m");
    assert_eq!(get_period_format(&PeriodGranularity::Yearly), "%Y");
}

#[test]
fn test_resolve_date_range_all_presets() {
    let presets = [
        DateRangePreset::CurrentMonth,
        DateRangePreset::LastMonth,
        DateRangePreset::CurrentYear,
        DateRangePreset::LastYear,
        DateRangePreset::Last3Months,
        DateRangePreset::Last6Months,
        DateRangePreset::Last12Months,
        DateRangePreset::Custom,
    ];
    for preset in presets {
        let filter = ReportFilter {
            date_range_preset: preset,
            start_date: None,
            end_date: None,
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        let (start, end) = resolve_date_range(&filter);
        assert!(start.len() == 10 && end.len() == 10);
    }
}

#[test]
fn test_get_standard_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_standard_report(&conn, &filter).unwrap();
    assert_eq!(dto.period_income, 100000);
    assert_eq!(dto.period_expense, 12700);
    assert_eq!(dto.prev_income, 0);
    assert_eq!(dto.prev_expense, 0);
    assert_eq!(dto.income_change_pct, 100.0);
    assert_eq!(dto.expense_change_pct, 100.0);
    assert_eq!(dto.net_worth_trend.len(), 5);
    assert_eq!(dto.net_worth_trend[0].net_worth, 100000);
    assert_eq!(dto.net_worth_trend[4].net_worth, 87400);
    assert_eq!(dto.account_changes.len(), 2);
    let cash_change = dto
        .account_changes
        .iter()
        .find(|c| c.account_id == seeded.cash)
        .unwrap();
    assert_eq!(cash_change.end_balance, 95400);
    let cc_change = dto
        .account_changes
        .iter()
        .find(|c| c.account_id == seeded.credit_card)
        .unwrap();
    assert_eq!(cc_change.end_balance, -8000);
}

#[test]
fn test_get_standard_report_empty_period_change_pct_zero() {
    let conn = setup_test_db();
    seed_data(&conn);

    // No data in this range -> both diffs are zero -> 0.0% change
    let filter = custom_filter("2025-01-01", "2025-01-31");
    let dto = get_standard_report(&conn, &filter).unwrap();
    assert_eq!(dto.income_change_pct, 0.0);
    assert_eq!(dto.expense_change_pct, 0.0);
}

#[test]
fn test_get_standard_report_account_filter() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let mut filter = custom_filter("2024-01-01", "2024-01-31");
    filter.account_ids = Some(vec![seeded.food_account.clone()]);
    let dto = get_standard_report(&conn, &filter).unwrap();
    assert_eq!(dto.period_income, 0);
    assert_eq!(dto.period_expense, 11500);
    assert_eq!(dto.account_changes.len(), 0);
}

#[test]
fn test_get_standard_report_all_presets() {
    let conn = setup_test_db();
    seed_data(&conn);
    for preset in [
        DateRangePreset::CurrentMonth,
        DateRangePreset::LastMonth,
        DateRangePreset::CurrentYear,
        DateRangePreset::LastYear,
        DateRangePreset::Last3Months,
        DateRangePreset::Last6Months,
        DateRangePreset::Last12Months,
        DateRangePreset::Custom,
    ] {
        let filter = ReportFilter {
            date_range_preset: preset,
            start_date: Some("2024-01-01".to_string()),
            end_date: Some("2024-01-31".to_string()),
            period_granularity: PeriodGranularity::Monthly,
            account_ids: None,
            category_ids: None,
        };
        assert!(get_standard_report(&conn, &filter).is_ok());
    }
}

#[test]
fn test_get_standard_report_current_month_change_pct() {
    let conn = setup_test_db();
    let (month_start, month_end) = current_month_bounds();
    let (prev_start, prev_end) = last_month_bounds();

    let cash = create_account_with_path(&conn, "Assets/Cash2", Some(&net_worth_input())).unwrap();
    let income = create_account_with_path(&conn, "Income/Salary2", None).unwrap();
    let expense = create_account_with_path(&conn, "Expenses/Food2", None).unwrap();

    // Previous month: income 50000, expense 10000
    create_transaction(
        &conn,
        &prev_start,
        "Prev Salary",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: 50000,
            },
            PostingInput {
                account_id: income.clone(),
                amount: -50000,
            },
        ],
    )
    .unwrap();
    create_transaction(
        &conn,
        &prev_end,
        "Prev Food",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -10000,
            },
            PostingInput {
                account_id: expense.clone(),
                amount: 10000,
            },
        ],
    )
    .unwrap();
    // Current month: income 60000, expense 12000
    create_transaction(
        &conn,
        &month_start,
        "Cur Salary",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: 60000,
            },
            PostingInput {
                account_id: income.clone(),
                amount: -60000,
            },
        ],
    )
    .unwrap();
    create_transaction(
        &conn,
        &month_end,
        "Cur Food",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -12000,
            },
            PostingInput {
                account_id: expense.clone(),
                amount: 12000,
            },
        ],
    )
    .unwrap();

    let filter = ReportFilter {
        date_range_preset: DateRangePreset::CurrentMonth,
        start_date: None,
        end_date: None,
        period_granularity: PeriodGranularity::Monthly,
        account_ids: None,
        category_ids: None,
    };
    let dto = get_standard_report(&conn, &filter).unwrap();
    assert_eq!(dto.period_income, 60000);
    assert_eq!(dto.period_expense, 12000);
    assert_eq!(dto.prev_income, 50000);
    assert_eq!(dto.prev_expense, 10000);
    assert!((dto.income_change_pct - 20.0).abs() < 0.001);
    assert!((dto.expense_change_pct - 20.0).abs() < 0.001);
}

#[test]
fn test_get_trend_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_trend_report(&conn, &filter).unwrap();
    assert_eq!(dto.granularity, "daily");
    assert_eq!(dto.data_points.len(), 5);
    assert_eq!(dto.total_income, 100000);
    assert_eq!(dto.total_expense, 12700);
    assert_eq!(dto.total_net, 87300);

    // Category filter + monthly granularity
    let mut filtered = custom_filter("2024-01-01", "2024-01-31");
    filtered.period_granularity = PeriodGranularity::Monthly;
    filtered.category_ids = Some(vec![seeded.food_category.clone()]);
    let dto = get_trend_report(&conn, &filtered).unwrap();
    assert_eq!(dto.data_points.len(), 1);
    assert_eq!(dto.data_points[0].period, "2024-01");
    assert_eq!(dto.total_income, 0);
    assert_eq!(dto.total_expense, 11500);

    // Weekly and yearly granularities
    let mut weekly = custom_filter("2024-01-01", "2024-01-31");
    weekly.period_granularity = PeriodGranularity::Weekly;
    assert!(get_trend_report(&conn, &weekly).is_ok());

    let mut yearly = custom_filter("2024-01-01", "2024-01-31");
    yearly.period_granularity = PeriodGranularity::Yearly;
    let dto = get_trend_report(&conn, &yearly).unwrap();
    assert_eq!(dto.data_points.len(), 1);
    assert_eq!(dto.data_points[0].period, "2024");
}

#[test]
fn test_get_category_breakdown_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_category_breakdown_report(&conn, &filter, "expense").unwrap();
    assert_eq!(dto.total_amount, 12700);
    assert_eq!(dto.categories.len(), 2);
    let food = dto
        .categories
        .iter()
        .find(|c| c.category_name == "Food")
        .unwrap();
    assert_eq!(food.amount, 11500);
    assert!((food.percentage - 11500.0 / 12700.0 * 100.0).abs() < 0.001);
    assert_eq!(food.transaction_count, 2);

    let income_dto = get_category_breakdown_report(&conn, &filter, "income").unwrap();
    assert_eq!(income_dto.total_amount, 100000);
    assert_eq!(income_dto.categories.len(), 1);
    assert_eq!(income_dto.categories[0].category_name, "Salary");
    assert_eq!(income_dto.categories[0].percentage, 100.0);

    // Category filter
    let mut filtered = custom_filter("2024-01-01", "2024-01-31");
    filtered.category_ids = Some(vec![seeded.transport_category.clone()]);
    let dto = get_category_breakdown_report(&conn, &filtered, "expense").unwrap();
    assert_eq!(dto.total_amount, 1200);
    assert_eq!(dto.categories.len(), 1);
}

#[test]
fn test_get_balance_sheet_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let dto = get_balance_sheet_report(&conn, "2024-01-31").unwrap();
    assert_eq!(dto.snapshot_date, "2024-01-31");
    assert_eq!(dto.assets.len(), 1);
    assert_eq!(dto.assets[0].account_id, seeded.cash);
    assert_eq!(dto.assets[0].balance, 95400);
    assert_eq!(dto.liabilities.len(), 1);
    assert_eq!(dto.liabilities[0].account_id, seeded.credit_card);
    assert_eq!(dto.liabilities[0].balance, -8000);
    assert_eq!(dto.total_assets, 95400);
    assert_eq!(dto.total_liabilities, -8000);
    assert_eq!(dto.net_worth, 87400);
}

#[test]
fn test_get_account_transactions_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 10).unwrap();
    assert_eq!(dto.account_id, seeded.cash);
    assert_eq!(dto.account_name, "Cash");
    assert_eq!(dto.total_count, 4);
    assert_eq!(dto.total_pages, 1);
    assert_eq!(dto.transactions.len(), 4);
    assert_eq!(dto.total_inflow, 100100);
    assert_eq!(dto.total_outflow, 4700);
    assert_eq!(dto.net_change, 95400);

    let salary = dto
        .transactions
        .iter()
        .find(|t| t.description == "Salary Jan")
        .unwrap();
    assert_eq!(salary.amount, 100000);
    assert!(salary.is_inflow);
    assert_eq!(salary.counter_account_name.as_deref(), Some("Salary"));
    assert_eq!(salary.category_name.as_deref(), Some("Salary"));
}

#[test]
fn test_get_account_transactions_report_pagination() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 2).unwrap();
    assert_eq!(dto.total_pages, 2);
    assert_eq!(dto.transactions.len(), 2);

    // page_size 0 -> division by zero handled with fallback page count
    let dto = get_account_transactions_report(&conn, &seeded.cash, &filter, 1, 0).unwrap();
    assert_eq!(dto.total_pages, 1);
    assert_eq!(dto.transactions.len(), 0);
}

#[test]
fn test_get_account_balance_trend_report_with_data() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_account_balance_trend_report(&conn, &seeded.cash, &filter).unwrap();
    assert_eq!(dto.account_name, "Cash");
    assert_eq!(dto.granularity, "daily");
    assert_eq!(dto.data_points.len(), 4);
    assert_eq!(dto.data_points[0].balance, 100000);
    assert_eq!(dto.data_points[3].balance, 95400);

    let mut monthly = custom_filter("2024-01-01", "2024-01-31");
    monthly.period_granularity = PeriodGranularity::Monthly;
    let dto = get_account_balance_trend_report(&conn, &seeded.cash, &monthly).unwrap();
    assert_eq!(dto.data_points.len(), 1);
    assert_eq!(dto.data_points[0].period, "2024-01");
    assert_eq!(dto.data_points[0].balance, 95400);

    // Unknown account -> error
    assert!(get_account_balance_trend_report(&conn, "missing", &filter).is_err());
}

/// The closing balance of a period must come from the posting that is last *by
/// date*, not the one inserted last. Back-dating is normal (recording a January
/// expense while it is already February), and it puts insertion order and date
/// order out of step — reading the balance at `MAX(rowid)` then reported a
/// January close that ignored the very transaction the user had just entered.
#[test]
fn test_balance_trend_reads_the_close_by_date_not_by_insert_order() {
    let conn = setup_test_db();
    let cash = create_account_with_path(&conn, "Assets/Cash", Some(&net_worth_input())).unwrap();
    let food_category = create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();
    let food_account = account_id_for_category(&conn, &food_category).unwrap();

    // Insert the later transaction first, so rowid order contradicts date order.
    create_transaction(
        &conn,
        "2024-01-25",
        "Late entry",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -1000,
            },
            PostingInput {
                account_id: food_account.clone(),
                amount: 1000,
            },
        ],
    )
    .unwrap();
    create_transaction(
        &conn,
        "2024-01-10",
        "Back-dated",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -500,
            },
            PostingInput {
                account_id: food_account.clone(),
                amount: 500,
            },
        ],
    )
    .unwrap();

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_account_balance_trend_report(&conn, &cash, &filter).unwrap();
    assert_eq!(dto.data_points.len(), 2);
    assert_eq!(dto.data_points[0].period_start, "2024-01-10");
    assert_eq!(dto.data_points[0].balance, -500);
    assert_eq!(dto.data_points[1].period_start, "2024-01-25");
    assert_eq!(dto.data_points[1].balance, -1500);

    let mut monthly = custom_filter("2024-01-01", "2024-01-31");
    monthly.period_granularity = PeriodGranularity::Monthly;
    let dto = get_account_balance_trend_report(&conn, &cash, &monthly).unwrap();
    assert_eq!(dto.data_points.len(), 1);
    assert_eq!(
        dto.data_points[0].balance, -1500,
        "the month closes on the last date, not the last insert"
    );
}

#[test]
fn test_get_audit_report_with_data() {
    let conn = setup_test_db();
    seed_data(&conn);

    let dto = get_audit_report(&conn).unwrap();
    assert_eq!(dto.balance_check.total_transactions, 6);
    assert_eq!(dto.balance_check.balanced_count, 5);
    assert_eq!(dto.balance_check.unbalanced_count, 1);
    assert!(!dto.balance_check.unbalanced_has_more);
    assert_eq!(dto.balance_check.unbalanced_transactions.len(), 1);
    assert_eq!(
        dto.balance_check.unbalanced_transactions[0].transaction_id,
        "tx-unbalanced"
    );
    assert_eq!(dto.balance_check.unbalanced_transactions[0].sum, 100);

    assert_eq!(dto.category_check.uncategorized_transactions, 1);
    assert_eq!(dto.category_check.uncategorized_list.len(), 1);
    assert!(!dto.category_check.uncategorized_has_more);

    assert_eq!(dto.account_usage.len(), 5);
    let cash_stat = dto
        .account_usage
        .iter()
        .find(|s| s.account_name == "Cash")
        .unwrap();
    assert_eq!(cash_stat.posting_count, 5);
    assert_eq!(cash_stat.total_debit, 100100);
    assert_eq!(cash_stat.total_credit, 9700);
}

#[test]
fn test_get_audit_report_more_than_limit() {
    let conn = setup_test_db();
    let seeded = seed_data(&conn);

    let mut tx_stmt = conn
            .prepare(
                "INSERT INTO transactions (id, date, description, created_at, updated_at)
                 VALUES (?1, '2024-03-01', 'Broken', '2024-03-01T00:00:00+08:00', '2024-03-01T00:00:00+08:00')",
            )
            .unwrap();
    let mut posting_stmt = conn
        .prepare(
            "INSERT INTO postings (id, transaction_id, account_id, amount, sequence, created_at)
                 VALUES (?1, ?2, ?3, 100, 0, '2024-03-01T00:00:00+08:00')",
        )
        .unwrap();
    for i in 0..105 {
        let tx_id = format!("tx-unbalanced-{}", i);
        tx_stmt.execute(params![&tx_id]).unwrap();
        posting_stmt
            .execute(params![format!("p-{}", i), &tx_id, &seeded.cash])
            .unwrap();
    }

    let dto = get_audit_report(&conn).unwrap();
    assert!(dto.balance_check.unbalanced_has_more);
    assert_eq!(dto.balance_check.unbalanced_transactions.len(), 100);
    assert_eq!(dto.balance_check.unbalanced_count, 106);
    assert!(dto.category_check.uncategorized_has_more);
    assert_eq!(dto.category_check.uncategorized_list.len(), 100);
    assert_eq!(dto.category_check.uncategorized_transactions, 106);
}

/// A split transaction must be spread over its categories, not credited to one.
#[test]
fn test_split_transaction_splits_across_categories() {
    let conn = setup_test_db();
    let cash = create_account_with_path(&conn, "Assets/Cash", Some(&net_worth_input())).unwrap();
    let food_category = create_category(&conn, "Food", &CategoryType::Expense, None).unwrap();
    let transport_category =
        create_category(&conn, "Transport", &CategoryType::Expense, None).unwrap();
    let food = account_id_for_category(&conn, &food_category).unwrap();
    let transport = account_id_for_category(&conn, &transport_category).unwrap();

    // One payment, two categories.
    create_transaction(
        &conn,
        "2024-01-15",
        "Lunch and taxi",
        &[
            PostingInput {
                account_id: cash.clone(),
                amount: -30000,
            },
            PostingInput {
                account_id: food,
                amount: 10000,
            },
            PostingInput {
                account_id: transport,
                amount: 20000,
            },
        ],
    )
    .unwrap();

    let filter = custom_filter("2024-01-01", "2024-01-31");
    let dto = get_category_breakdown_report(&conn, &filter, "expense").unwrap();
    assert_eq!(dto.total_amount, 30000);
    assert_eq!(dto.categories.len(), 2);

    let food_item = dto
        .categories
        .iter()
        .find(|c| c.category_name == "Food")
        .unwrap();
    assert_eq!(food_item.amount, 10000);
    assert_eq!(food_item.transaction_count, 1);

    let transport_item = dto
        .categories
        .iter()
        .find(|c| c.category_name == "Transport")
        .unwrap();
    assert_eq!(transport_item.amount, 20000);

    // The trend report attributes the same split by category filter.
    let mut by_category = custom_filter("2024-01-01", "2024-01-31");
    by_category.period_granularity = PeriodGranularity::Monthly;
    by_category.category_ids = Some(vec![transport_category.clone()]);
    let trend = get_trend_report(&conn, &by_category).unwrap();
    assert_eq!(trend.total_expense, 20000);
    assert_eq!(trend.total_income, 0);
}
