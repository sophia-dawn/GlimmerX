// ============================================================================
// Report types (matching Rust backend structs with camelCase)
// ============================================================================

export type DateRangePreset =
  | "currentMonth"
  | "lastMonth"
  | "currentYear"
  | "lastYear"
  | "last3Months"
  | "last6Months"
  | "last12Months"
  | "custom";

export type PeriodGranularity = "daily" | "weekly" | "monthly" | "yearly";

export interface ReportFilter {
  dateRangePreset: DateRangePreset;
  startDate?: string;
  endDate?: string;
  periodGranularity: PeriodGranularity;
  accountIds?: string[];
  categoryIds?: string[];
}

export interface NetWorthPoint {
  date: string;
  netWorth: number;
}

export interface AccountChange {
  accountId: string;
  accountName: string;
  accountType: string;
  startBalance: number;
  endBalance: number;
  change: number;
}

export interface StandardReportDto {
  periodIncome: number;
  periodExpense: number;
  prevIncome: number;
  prevExpense: number;
  incomeChangePct: number;
  expenseChangePct: number;
  netWorthTrend: NetWorthPoint[];
  accountChanges: AccountChange[];
}

export interface BudgetExecutionItem {
  budgetId: string;
  categoryId: string;
  categoryName: string; // Used as display name
  budgetedAmount: number;
  spentAmount: number;
  remainingAmount: number;
  percentage: number;
  isOverBudget: boolean;
}

export interface BudgetExecutionReportDto {
  totalBudgeted: number;
  totalSpent: number;
  totalRemaining: number;
  overBudgetCount: number;
  budgetItems: BudgetExecutionItem[];
}

export interface CategoryBreakdownItem {
  categoryId: string;
  categoryName: string;
  amount: number;
  percentage: number;
  transactionCount: number;
}

export interface CategoryBreakdownReportDto {
  totalAmount: number;
  categories: CategoryBreakdownItem[];
}

export interface BalanceSheetItem {
  accountId: string;
  accountName: string;
  accountType: string;
  balance: number;
}

export interface BalanceSheetReportDto {
  snapshotDate: string;
  assets: BalanceSheetItem[];
  liabilities: BalanceSheetItem[];
  totalAssets: number;
  totalLiabilities: number;
  netWorth: number;
}

export interface TrendPoint {
  period: string;
  periodStart: string;
  periodEnd: string;
  income: number;
  expense: number;
  net: number;
}

export interface TrendReportDto {
  granularity: string;
  dataPoints: TrendPoint[];
  totalIncome: number;
  totalExpense: number;
  totalNet: number;
}

export interface CategoryComparisonItem {
  categoryId: string;
  categoryName: string;
  month1Amount: number;
  month2Amount: number;
  diff: number;
  changePct: number; // -1.0 indicates "new category" (no previous month data)
}

export interface MonthComparisonReportDto {
  month1: string;
  month2: string;
  month1Income: number;
  month1Expense: number;
  month2Income: number;
  month2Expense: number;
  incomeDiff: number;
  expenseDiff: number;
  incomeChangePct: number;
  expenseChangePct: number;
  categoryComparison: CategoryComparisonItem[];
}

export interface MonthlySummary {
  month: number;
  income: number;
  expense: number;
  net: number;
}

export interface ReportCategoryAmount {
  categoryId: string;
  categoryName: string;
  amount: number;
  percentage: number;
}

export interface YearSummaryReportDto {
  year: number;
  totalIncome: number;
  totalExpense: number;
  net: number;
  monthlyBreakdown: MonthlySummary[];
  topIncomeCategories: ReportCategoryAmount[];
  topExpenseCategories: ReportCategoryAmount[];
}

export interface AccountTransactionItem {
  transactionId: string;
  date: string;
  description: string;
  categoryName?: string;
  amount: number;
  isInflow: boolean;
  counterAccountName?: string;
}

export interface AccountTransactionsReportDto {
  accountId: string;
  accountName: string;
  startDate: string;
  endDate: string;
  transactions: AccountTransactionItem[];
  totalInflow: number;
  totalOutflow: number;
  netChange: number;
  totalCount: number;
  totalPages: number;
}

export interface BalancePoint {
  period: string;
  periodStart: string;
  periodEnd: string;
  balance: number;
}

export interface AccountBalanceTrendReportDto {
  accountId: string;
  accountName: string;
  granularity: string;
  dataPoints: BalancePoint[];
}

export interface UnbalancedTransaction {
  transactionId: string;
  date: string;
  description: string;
  sum: number;
}

export interface BalanceCheckResult {
  totalTransactions: number;
  balancedCount: number;
  unbalancedCount: number;
  unbalancedHasMore?: boolean;
  unbalancedTransactions: UnbalancedTransaction[];
}

export interface UncategorizedTransaction {
  transactionId: string;
  date: string;
  description: string;
}

export interface CategoryCheckResult {
  uncategorizedTransactions: number;
  uncategorizedHasMore?: boolean;
  uncategorizedList: UncategorizedTransaction[];
}

export interface AccountUsageStat {
  accountId: string;
  accountName: string;
  accountType: string;
  postingCount: number;
  totalDebit: number;
  totalCredit: number;
  lastTransactionDate?: string;
}

export interface AuditReportDto {
  balanceCheck: BalanceCheckResult;
  categoryCheck: CategoryCheckResult;
  accountUsage: AccountUsageStat[];
  generatedAt: string;
}
