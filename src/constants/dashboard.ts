/**
 * Dashboard 模块常量配置
 */

// 分页限制
export const DASHBOARD_LIMITS = {
  topExpenses: 10,
  // Fills the pane on a 900 px-tall window; the table scrolls on anything
  // shorter, which is the point of a pane.
  recentTransactions: 20,
} as const;

// 日期格式
export const DATE_FORMATS = {
  short: "MM-dd",
  full: "yyyy-MM-dd",
} as const;
