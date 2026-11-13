/**
 * Report 模块常量配置
 */

// 报表列表
export const REPORT_LIST = [
  { id: "standard", nameKey: "reports.names.standard", group: "P0" },
  { id: "category", nameKey: "reports.names.category", group: "P0" },
  { id: "balanceSheet", nameKey: "reports.names.balanceSheet", group: "P0" },
  { id: "trend", nameKey: "reports.names.trend", group: "P0" },
  {
    id: "monthComparison",
    nameKey: "reports.names.monthComparison",
    group: "P1",
  },
  { id: "yearSummary", nameKey: "reports.names.yearSummary", group: "P1" },
  {
    id: "accountTransactions",
    nameKey: "reports.names.accountTransactions",
    group: "P1",
  },
  {
    id: "accountBalanceTrend",
    nameKey: "reports.names.accountBalanceTrend",
    group: "P1",
  },
  { id: "audit", nameKey: "reports.names.audit", group: "P2" },
] as const;

// 报表分组
export const REPORT_GROUP_LABELS = {
  P0: "reports.groups.p0",
  P1: "reports.groups.p1",
  P2: "reports.groups.p2",
} as const;

// 日期范围预设
export const DATE_RANGE_PRESETS = [
  { id: "currentMonth", labelKey: "reports.dateRange.currentMonth" },
  { id: "lastMonth", labelKey: "reports.dateRange.lastMonth" },
  { id: "currentYear", labelKey: "reports.dateRange.currentYear" },
  { id: "lastYear", labelKey: "reports.dateRange.lastYear" },
  { id: "last3Months", labelKey: "reports.dateRange.last3Months" },
  { id: "last6Months", labelKey: "reports.dateRange.last6Months" },
  { id: "last12Months", labelKey: "reports.dateRange.last12Months" },
  { id: "custom", labelKey: "reports.dateRange.custom" },
] as const;

// 时间粒度
export const GRANULARITY_OPTIONS = [
  { id: "daily", labelKey: "reports.granularity.daily" },
  { id: "weekly", labelKey: "reports.granularity.weekly" },
  { id: "monthly", labelKey: "reports.granularity.monthly" },
  { id: "yearly", labelKey: "reports.granularity.yearly" },
] as const;
