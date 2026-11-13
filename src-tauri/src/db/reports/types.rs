//! Report filter types and DTOs shared by every report query.

use serde::{Deserialize, Serialize};

/// Filter parameters for generating reports.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportFilter {
    pub date_range_preset: DateRangePreset,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub period_granularity: PeriodGranularity,
    pub account_ids: Option<Vec<String>>,
    pub category_ids: Option<Vec<String>>,
}

/// Preset date ranges for report filtering.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum DateRangePreset {
    CurrentMonth,
    LastMonth,
    CurrentYear,
    LastYear,
    Last3Months,
    Last6Months,
    Last12Months,
    Custom,
}

/// Granularity for aggregating report data by time period.
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum PeriodGranularity {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl std::fmt::Display for PeriodGranularity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PeriodGranularity::Daily => write!(f, "daily"),
            PeriodGranularity::Weekly => write!(f, "weekly"),
            PeriodGranularity::Monthly => write!(f, "monthly"),
            PeriodGranularity::Yearly => write!(f, "yearly"),
        }
    }
}

/// Standard overview report with period comparison.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardReportDto {
    pub period_income: i64,
    pub period_expense: i64,
    pub prev_income: i64,
    pub prev_expense: i64,
    pub income_change_pct: f64,
    pub expense_change_pct: f64,
    pub net_worth_trend: Vec<NetWorthPoint>,
    pub account_changes: Vec<AccountChange>,
}

/// Single point in the net worth trend over time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetWorthPoint {
    pub date: String,
    pub net_worth: i64,
}

/// Account balance change over the report period.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountChange {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub start_balance: i64,
    pub end_balance: i64,
    pub change: i64,
}

/// Category spending breakdown report.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownReportDto {
    pub total_amount: i64,
    pub categories: Vec<CategoryBreakdownItem>,
}

/// Single category breakdown with spending details.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownItem {
    pub category_id: String,
    pub category_name: String,
    pub amount: i64,
    pub percentage: f64,
    pub transaction_count: i64,
}

/// Balance sheet report showing assets, liabilities, and net worth.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheetReportDto {
    pub snapshot_date: String,
    pub assets: Vec<BalanceSheetItem>,
    pub liabilities: Vec<BalanceSheetItem>,
    pub total_assets: i64,
    pub total_liabilities: i64,
    pub net_worth: i64,
}

/// Single account entry in the balance sheet.
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalanceSheetItem {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub balance: i64,
}

/// Trend report showing income/expense patterns over time.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendReportDto {
    pub granularity: String,
    pub data_points: Vec<TrendPoint>,
    pub total_income: i64,
    pub total_expense: i64,
    pub total_net: i64,
}

/// Single data point in a trend report.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendPoint {
    pub period: String,
    pub period_start: String,
    pub period_end: String,
    pub income: i64,
    pub expense: i64,
    pub net: i64,
}

/// Month comparison report comparing two months side-by-side.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthComparisonReportDto {
    pub month1: String,
    pub month2: String,
    pub month1_income: i64,
    pub month1_expense: i64,
    pub month2_income: i64,
    pub month2_expense: i64,
    pub income_diff: i64,
    pub expense_diff: i64,
    pub income_change_pct: f64,
    pub expense_change_pct: f64,
    pub category_comparison: Vec<CategoryComparisonItem>,
}

/// Single category comparison between two months.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryComparisonItem {
    pub category_id: String,
    pub category_name: String,
    pub month1_amount: i64,
    pub month2_amount: i64,
    pub diff: i64,
    pub change_pct: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YearSummaryReportDto {
    pub year: i32,
    pub total_income: i64,
    pub total_expense: i64,
    pub net: i64,
    pub monthly_breakdown: Vec<MonthlySummary>,
    pub top_income_categories: Vec<CategoryAmount>,
    pub top_expense_categories: Vec<CategoryAmount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySummary {
    pub month: u32,
    pub income: i64,
    pub expense: i64,
    pub net: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAmount {
    pub category_id: String,
    pub category_name: String,
    pub amount: i64,
    pub percentage: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditReportDto {
    pub balance_check: BalanceCheckResult,
    pub category_check: CategoryCheckResult,
    pub account_usage: Vec<AccountUsageStat>,
    pub generated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceCheckResult {
    pub total_transactions: u32,
    pub balanced_count: u32,
    pub unbalanced_count: u32,
    pub unbalanced_has_more: bool,
    pub unbalanced_transactions: Vec<UnbalancedTransaction>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbalancedTransaction {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub sum: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryCheckResult {
    pub uncategorized_transactions: u32,
    pub uncategorized_has_more: bool,
    pub uncategorized_list: Vec<UncategorizedTransaction>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UncategorizedTransaction {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUsageStat {
    pub account_id: String,
    pub account_name: String,
    pub account_type: String,
    pub posting_count: u32,
    pub total_debit: i64,
    pub total_credit: i64,
    pub last_transaction_date: Option<String>,
}

/// Account transactions report DTOs
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionsReportDto {
    pub account_id: String,
    pub account_name: String,
    pub start_date: String,
    pub end_date: String,
    pub transactions: Vec<AccountTransactionItem>,
    pub total_inflow: i64,
    pub total_outflow: i64,
    pub net_change: i64,
    pub total_count: u32,
    pub total_pages: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountTransactionItem {
    pub transaction_id: String,
    pub date: String,
    pub description: String,
    pub category_name: Option<String>,
    pub amount: i64,
    pub is_inflow: bool,
    pub counter_account_name: Option<String>,
}

/// Account balance trend report DTOs
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceTrendReportDto {
    pub account_id: String,
    pub account_name: String,
    pub granularity: String,
    pub data_points: Vec<BalancePoint>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalancePoint {
    pub period: String,
    pub period_start: String,
    pub period_end: String,
    pub balance: i64,
}
