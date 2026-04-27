/**
 * Dashboard 模块常量配置
 */

// 图表颜色
export const CHART_COLORS = {
  income: "#22c55e",
  expense: "#ef4444",
} as const;

// 分类饼图颜色数组
export const CATEGORY_COLORS = [
  "#3b82f6",
  "#22c55e",
  "#f59e0b",
  "#ef4444",
  "#8b5cf6",
  "#ec4899",
  "#14b8a6",
  "#f97316",
  "#6366f1",
  "#84cc16",
] as const;

// 分页限制
export const DASHBOARD_LIMITS = {
  topExpenses: 10,
  recentTransactions: 8,
} as const;

// 日期格式
export const DATE_FORMATS = {
  short: "MM-dd",
  full: "yyyy-MM-dd",
} as const;

// 默认货币
export const DEFAULT_CURRENCY = "CNY";
