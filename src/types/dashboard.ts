// ============================================================================
// Dashboard types (matching Rust backend structs)
// ============================================================================

export interface DashboardSummary {
  month_income: number; // cents
  month_expense: number; // cents
  month_start: string;
  month_end: string;
  year_income: number; // cents
  year_expense: number; // cents
  year_start: string;
  year_end: string;
  total_assets: number; // cents
  total_liabilities: number; // cents
  net_worth: number; // cents
  calculated_at: string;
}

export interface DailyIncomeExpense {
  date: string;
  income: number; // cents
  expense: number; // cents
  has_transactions: boolean;
}

export interface MonthlyChartData {
  year: number;
  month: number;
  days: DailyIncomeExpense[];
  month_total_income: number; // cents
  month_total_expense: number; // cents
}

export interface CategoryAmount {
  category_id: string;
  category_name: string;
  icon: string | null;
  amount: number; // cents
  percentage: number;
}

export interface CategoryBreakdownData {
  year: number;
  month: number;
  category_type: "income" | "expense";
  categories: CategoryAmount[];
  total_amount: number; // cents
}

export interface TopExpenseItem {
  transaction_id: string;
  date: string;
  description: string;
  amount: number; // cents
  category_id: string | null;
  category_name: string | null;
  category_icon: string | null;
}

export interface TopExpensesData {
  year: number;
  month: number;
  expenses: TopExpenseItem[];
}
