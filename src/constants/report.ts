/**
 * Report catalogue and filter vocabulary.
 *
 * Six reports, each answering a question the previous nine did not: the old list
 * had four overlapping ways to look at cash flow (standard / trend / month
 * comparison / year summary) and two overlapping ways to look at one account
 * (transactions / balance trend), which is why the reports page read as a pile
 * rather than an index. What is left is one entry per question, and the
 * questions are ordered the way someone reads their own finances: what came in
 * and went out, where it went, what it did to net worth, what is owed, one
 * account in detail, and what the plan was.
 */

export const REPORT_IDS = [
  "cashflow",
  "categories",
  "netWorth",
  "balanceSheet",
  "accounts",
  "budgets",
] as const;

export type ReportId = (typeof REPORT_IDS)[number];

/**
 * Which date control a report needs. Not every report takes a range: a balance
 * sheet is a snapshot and a budget is a month, and offering a range where the
 * data is a point makes the filter look broken.
 */
export type ReportDateControl = "range" | "snapshot" | "month";

export interface ReportDefinition {
  id: ReportId;
  nameKey: string;
  /** Shown under the report title: what the numbers count, in one line. */
  scopeKey: string;
  dateControl: ReportDateControl;
}

export const REPORT_LIST = [
  {
    id: "cashflow",
    nameKey: "reports.names.cashflow",
    scopeKey: "reports.scope.cashflow",
    dateControl: "range",
  },
  {
    id: "categories",
    nameKey: "reports.names.categories",
    scopeKey: "reports.scope.categories",
    dateControl: "range",
  },
  {
    id: "netWorth",
    nameKey: "reports.names.netWorth",
    scopeKey: "reports.scope.netWorth",
    dateControl: "range",
  },
  {
    id: "balanceSheet",
    nameKey: "reports.names.balanceSheet",
    scopeKey: "reports.scope.balanceSheet",
    dateControl: "snapshot",
  },
  {
    id: "accounts",
    nameKey: "reports.names.accounts",
    scopeKey: "reports.scope.accounts",
    dateControl: "range",
  },
  {
    id: "budgets",
    nameKey: "reports.names.budgets",
    scopeKey: "reports.scope.budgets",
    dateControl: "month",
  },
] as const satisfies readonly ReportDefinition[];

export const DEFAULT_REPORT: ReportId = "cashflow";

export function findReport(id: string | null): ReportDefinition {
  return REPORT_LIST.find((report) => report.id === id) ?? REPORT_LIST[0];
}

// 日期范围预设
export const DATE_RANGE_PRESETS = [
  { id: "currentMonth", labelKey: "reports.dateRange.currentMonth" },
  { id: "lastMonth", labelKey: "reports.dateRange.lastMonth" },
  { id: "last3Months", labelKey: "reports.dateRange.last3Months" },
  { id: "last6Months", labelKey: "reports.dateRange.last6Months" },
  { id: "last12Months", labelKey: "reports.dateRange.last12Months" },
  { id: "currentYear", labelKey: "reports.dateRange.currentYear" },
  { id: "lastYear", labelKey: "reports.dateRange.lastYear" },
  { id: "custom", labelKey: "reports.dateRange.custom" },
] as const;

// 时间粒度
export const GRANULARITY_OPTIONS = [
  { id: "daily", labelKey: "reports.granularity.daily" },
  { id: "weekly", labelKey: "reports.granularity.weekly" },
  { id: "monthly", labelKey: "reports.granularity.monthly" },
  { id: "yearly", labelKey: "reports.granularity.yearly" },
] as const;

// 对比基期：跟谁比
export const COMPARE_OPTIONS = [
  { id: "previousPeriod", labelKey: "reports.compare.previousPeriod" },
  { id: "lastYear", labelKey: "reports.compare.lastYear" },
  { id: "none", labelKey: "reports.compare.none" },
] as const;
