//! Reporting queries: standard overview, comparisons, trends, balance sheet,
//! account ledgers and the integrity audit.
//!
//! Each area lives in its own submodule; everything public is re-exported here
//! so callers keep using `crate::db::reports::*`.

mod account;
mod audit;
mod common;
mod comparison;
mod standard;
mod trend;
mod types;

#[cfg(test)]
mod tests;

pub use account::{get_account_balance_trend_report, get_account_transactions_report};
pub use audit::get_audit_report;
pub use comparison::get_month_comparison;
pub use standard::get_standard_report;
pub use trend::{
    get_balance_sheet_report, get_category_breakdown_report, get_trend_report,
    get_year_summary_report,
};
// `db` is a private module, so re-exports only used through field types look
// unused to rustc even though `commands::reports` depends on them.
#[allow(unused_imports)]
pub use types::{
    AccountBalanceTrendReportDto, AccountChange, AccountTransactionItem,
    AccountTransactionsReportDto, AccountUsageStat, AuditReportDto, BalanceCheckResult,
    BalancePoint, BalanceSheetItem, BalanceSheetReportDto, CategoryAmount, CategoryBreakdownItem,
    CategoryBreakdownReportDto, CategoryCheckResult, CategoryComparisonItem, DateRangePreset,
    MonthComparisonReportDto, MonthlySummary, NetWorthPoint, PeriodGranularity, ReportFilter,
    StandardReportDto, TrendPoint, TrendReportDto, UnbalancedTransaction, UncategorizedTransaction,
    YearSummaryReportDto,
};
