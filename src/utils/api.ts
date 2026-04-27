import { invoke } from "@tauri-apps/api/core";
import type {
  AccountDto,
  AccountMeta,
  AccountMetaSchema,
  AccountTransaction,
  Budget,
  BudgetStatus,
  Category,
  CategoryDeletePreview,
  CreateAccountInput,
  CreateBudgetInput,
  CreateCategoryInput,
  CreateTransactionInput,
  DashboardSummary,
  DbInfo,
  EnhancedTransactionFilter,
  ExportResult,
  ImportResult,
  MonthlyChartData,
  CategoryBreakdownData,
  TopExpensesData,
  QuickAddInput,
  RecentDbEntry,
  TransactionDeletePreview,
  TransactionDetail,
  TransactionDto,
  TransactionListFilter,
  TransactionListResponse,
  UpdateAccountInput,
  UpdateBudgetInput,
  UpdateCategoryInput,
  UpdateTransactionInput,
} from "@/types";
import type { AccountBalanceItem } from "@/types";
import type {
  ReportFilter,
  StandardReportDto,
  BudgetExecutionReportDto,
  CategoryBreakdownReportDto,
  BalanceSheetReportDto,
  TrendReportDto,
  MonthComparisonReportDto,
  YearSummaryReportDto,
  AccountTransactionsReportDto,
  AccountBalanceTrendReportDto,
  AuditReportDto,
} from "@/types/report";

export type { RecentDbEntry };

/**
 * Typed wrapper around Tauri's invoke() with uniform error handling.
 */
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  const startTime = Date.now();

  // Filter sensitive fields from logging
  const SENSITIVE_FIELDS = ["password", "oldPassword", "newPassword"];
  const safeArgs = args
    ? {
        ...args,
        ...Object.fromEntries(
          SENSITIVE_FIELDS.filter((k) => k in args).map((k) => [
            k,
            "[REDACTED]",
          ]),
        ),
      }
    : args;

  console.log(`[invoke] ${command} start, args:`, safeArgs);
  try {
    const result = await invoke<T>(command, args);
    const elapsed = Date.now() - startTime;
    console.log(`[invoke] ${command} success (${elapsed}ms)`);
    return result;
  } catch (err: unknown) {
    const elapsed = Date.now() - startTime;
    console.error(`[invoke] ${command} failed (${elapsed}ms):`, err, safeArgs);
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Database commands
// ---------------------------------------------------------------------------

/**
 * Create a new encrypted database at the given path.
 */
export async function dbCreate(
  path: string,
  password: string,
): Promise<DbInfo> {
  return invokeCommand<DbInfo>("db_create", { path, password });
}

/**
 * Unlock an existing database at the given path with the given password.
 */
export async function dbUnlock(
  path: string,
  password: string,
): Promise<DbInfo> {
  return invokeCommand<DbInfo>("db_unlock", { path, password });
}

/**
 * Change the database password.
 */
export async function dbChangePassword(
  oldPassword: string,
  newPassword: string,
): Promise<void> {
  return invokeCommand<void>("db_change_password", {
    oldPassword,
    newPassword,
  });
}

/**
 * Check if a database file exists at the given path.
 */
export async function dbCheckExists(path: string): Promise<boolean> {
  return invokeCommand<boolean>("db_check_exists", { path });
}

/**
 * Check if any known database exists in the recent list.
 */
export async function dbCheckAnyExists(): Promise<boolean> {
  return invokeCommand<boolean>("db_check_any_exists");
}

/**
 * List recently opened databases.
 */
export async function dbListRecent(): Promise<RecentDbEntry[]> {
  return invokeCommand<RecentDbEntry[]>("db_list_recent");
}

/**
 * Remove a database from the recent list.
 */
export async function dbRemoveRecent(path: string): Promise<void> {
  return invokeCommand<void>("db_remove_recent", { path });
}

/**
 * Lock the database (clear the connection from application state).
 */
export async function dbLock(): Promise<void> {
  return invokeCommand<void>("db_lock");
}

export async function dbIsUnlocked(): Promise<boolean> {
  return invokeCommand<boolean>("db_is_unlocked");
}

export async function dbPing(): Promise<boolean> {
  return invokeCommand<boolean>("db_ping");
}

// ---------------------------------------------------------------------------
// Account commands
// ---------------------------------------------------------------------------

/**
 * Create a new account with optional initial balance.
 */
export async function accountCreate(
  input: CreateAccountInput,
): Promise<AccountDto> {
  return invokeCommand<AccountDto>("account_create", { input });
}

/**
 * Get all accounts as a flat list.
 */
export async function accountList(): Promise<AccountDto[]> {
  return invokeCommand<AccountDto[]>("account_list");
}

/**
 * Update an existing account.
 */
export async function accountUpdate(
  id: string,
  input: UpdateAccountInput,
): Promise<AccountDto> {
  const result = await invokeCommand<AccountDto>("account_update", {
    id,
    input,
  });
  return result;
}

/**
 * Permanently delete an account with no transactions.
 */
export async function accountDelete(id: string): Promise<void> {
  return invokeCommand<void>("account_delete", { id });
}

/**
 * Get the current balance for an account.
 */
export async function accountBalance(id: string): Promise<number> {
  return invokeCommand<number>("account_balance", { id });
}

/**
 * Get balances for multiple accounts in a single request.
 */
export async function accountBalancesBatch(
  ids: string[],
): Promise<AccountBalanceItem[]> {
  return invokeCommand<AccountBalanceItem[]>("account_balances_batch", { ids });
}

/**
 * Transfer amount between two accounts.
 */
export async function accountTransfer(
  fromId: string,
  toId: string,
  amount: number,
  description: string,
): Promise<string> {
  return invokeCommand<string>("account_transfer", {
    fromId,
    toId,
    amount,
    description,
  });
}

/**
 * Batch create multiple accounts in a single backend call.
 * Parents are automatically deduplicated.
 */
export async function accountBatchCreate(
  inputs: CreateAccountInput[],
): Promise<AccountDto[]> {
  return invokeCommand<AccountDto[]>("account_batch_create", { inputs });
}

/**
 * Get transaction history for an account with optional date range filtering.
 */
export async function accountTransactions(
  id: string,
  fromDate?: string,
  toDate?: string,
): Promise<AccountTransaction[]> {
  const args: Record<string, unknown> = { id };
  if (fromDate) args.fromDate = fromDate;
  if (toDate) args.toDate = toDate;
  return invokeCommand<AccountTransaction[]>("account_transactions", args);
}

// ---------------------------------------------------------------------------
// Account meta commands
// ---------------------------------------------------------------------------

/**
 * Get all metadata entries for an account.
 */
export async function accountMetaGet(
  accountId: string,
): Promise<AccountMeta[]> {
  return invokeCommand<AccountMeta[]>("account_meta_get", { accountId });
}

/**
 * Set a single metadata key-value pair for an account.
 */
export async function accountMetaSet(
  accountId: string,
  key: string,
  value: string,
): Promise<void> {
  return invokeCommand<void>("account_meta_set", { accountId, key, value });
}

/**
 * Batch set multiple metadata key-value pairs for an account.
 */
export async function accountMetaBatchSet(
  accountId: string,
  metas: { key: string; value: string }[],
): Promise<void> {
  return invokeCommand<void>("account_meta_batch_set", { accountId, metas });
}

export async function accountMetaSchema(): Promise<AccountMetaSchema> {
  return invokeCommand<AccountMetaSchema>("account_meta_schema");
}

// ---------------------------------------------------------------------------
// Category commands
// ---------------------------------------------------------------------------

export async function categoryList(
  type?: "income" | "expense",
): Promise<Category[]> {
  return invokeCommand<Category[]>("category_list", { type });
}

export async function categoryCreate(
  input: CreateCategoryInput,
): Promise<Category> {
  return invokeCommand<Category>("category_create", { input });
}

export async function categoryUpdate(
  id: string,
  input: UpdateCategoryInput,
): Promise<Category> {
  return invokeCommand<Category>("category_update", { id, input });
}

export async function categoryDelete(
  id: string,
  cascade: boolean = false,
): Promise<void> {
  return invokeCommand<void>("category_delete", { id, cascade });
}

export async function categoryDeletePreview(
  id: string,
): Promise<CategoryDeletePreview> {
  return invokeCommand<CategoryDeletePreview>("category_delete_preview", {
    id,
  });
}

// ---------------------------------------------------------------------------
// Transaction commands
// ---------------------------------------------------------------------------

export async function transactionCreate(
  input: CreateTransactionInput,
): Promise<TransactionDto> {
  return invokeCommand<TransactionDto>("transaction_create", { input });
}

export async function transactionList(
  filter?: TransactionListFilter,
): Promise<TransactionDto[]> {
  return invokeCommand<TransactionDto[]>("transaction_list", { filter });
}

export async function transactionGet(
  id: string,
): Promise<TransactionDto | null> {
  return invokeCommand<TransactionDto | null>("transaction_get", { id });
}

export async function transactionListPaginated(
  filter?: EnhancedTransactionFilter,
): Promise<TransactionListResponse> {
  return invokeCommand<TransactionListResponse>("transaction_list_paginated", {
    filter,
  });
}

// ---------------------------------------------------------------------------
// Quick Add Transaction
// ---------------------------------------------------------------------------

export async function quickAddTransaction(
  input: QuickAddInput,
): Promise<TransactionDto> {
  return invokeCommand<TransactionDto>("quick_add_transaction", { input });
}

// ---------------------------------------------------------------------------
// Transaction Detail
// ---------------------------------------------------------------------------

export async function transactionDetail(
  id: string,
): Promise<TransactionDetail> {
  return invokeCommand<TransactionDetail>("transaction_detail", { id });
}

// ---------------------------------------------------------------------------
// Transaction Update
// ---------------------------------------------------------------------------

export async function transactionUpdate(
  id: string,
  input: UpdateTransactionInput,
): Promise<TransactionDetail> {
  return invokeCommand<TransactionDetail>("transaction_update", { id, input });
}

// ---------------------------------------------------------------------------
// Transaction Delete
// ---------------------------------------------------------------------------

export async function transactionDeletePreview(
  id: string,
): Promise<TransactionDeletePreview> {
  return invokeCommand<TransactionDeletePreview>("transaction_delete_preview", {
    id,
  });
}

export async function transactionDelete(id: string): Promise<void> {
  return invokeCommand<void>("transaction_delete", { id });
}

// ---------------------------------------------------------------------------
// Dashboard commands
// ---------------------------------------------------------------------------

export async function dashboardSummary(params?: {
  fromDate?: string;
  toDate?: string;
}): Promise<DashboardSummary> {
  return invokeCommand<DashboardSummary>("dashboard_summary", params);
}

export async function dashboardMonthlyChart(params?: {
  year?: number;
  month?: number;
}): Promise<MonthlyChartData> {
  return invokeCommand<MonthlyChartData>("dashboard_monthly_chart", params);
}

export async function dashboardCategoryBreakdown(params?: {
  year?: number;
  month?: number;
  categoryType?: "income" | "expense";
}): Promise<CategoryBreakdownData> {
  return invokeCommand<CategoryBreakdownData>(
    "dashboard_category_breakdown",
    params,
  );
}

export async function dashboardTopExpenses(params?: {
  year?: number;
  month?: number;
  limit?: number;
}): Promise<TopExpensesData> {
  return invokeCommand<TopExpensesData>("dashboard_top_expenses", params);
}

// ---------------------------------------------------------------------------
// Budget commands
// ---------------------------------------------------------------------------

export async function budgetList(): Promise<Budget[]> {
  return invokeCommand<Budget[]>("budget_list");
}

export async function budgetListStatuses(
  year: number,
  month: number,
): Promise<BudgetStatus[]> {
  return invokeCommand<BudgetStatus[]>("budget_list_statuses", { year, month });
}

export async function budgetCreate(input: CreateBudgetInput): Promise<Budget> {
  return invokeCommand<Budget>("budget_create", { input });
}

export async function budgetUpdate(
  id: string,
  input: UpdateBudgetInput,
): Promise<Budget> {
  return invokeCommand<Budget>("budget_update", { id, input });
}

export async function budgetDelete(id: string): Promise<void> {
  return invokeCommand<void>("budget_delete", { id });
}

// ---------------------------------------------------------------------------
// Report commands
// ---------------------------------------------------------------------------

export async function reportStandard(
  filter: ReportFilter,
): Promise<StandardReportDto> {
  return invokeCommand<StandardReportDto>("report_standard", { filter });
}

export async function reportBudgetExecution(
  filter: ReportFilter,
  budgetIds?: string[],
): Promise<BudgetExecutionReportDto> {
  return invokeCommand<BudgetExecutionReportDto>("report_budget_execution", {
    filter,
    budgetIds,
  });
}

export async function reportCategoryBreakdown(
  filter: ReportFilter,
  incomeOrExpense: "income" | "expense",
): Promise<CategoryBreakdownReportDto> {
  return invokeCommand<CategoryBreakdownReportDto>(
    "report_category_breakdown",
    {
      filter,
      incomeOrExpense,
    },
  );
}

export async function reportBalanceSheet(
  snapshotDate: string,
): Promise<BalanceSheetReportDto> {
  return invokeCommand<BalanceSheetReportDto>("report_balance_sheet", {
    snapshotDate,
  });
}

export async function reportTrend(
  filter: ReportFilter,
): Promise<TrendReportDto> {
  return invokeCommand<TrendReportDto>("report_trend", { filter });
}

export async function reportMonthComparison(
  month1: string,
  month2: string,
): Promise<MonthComparisonReportDto> {
  return invokeCommand<MonthComparisonReportDto>("report_month_comparison", {
    month1,
    month2,
  });
}

export async function reportYearSummary(
  year: number,
): Promise<YearSummaryReportDto> {
  return invokeCommand<YearSummaryReportDto>("report_year_summary", { year });
}

export async function reportAccountTransactions(
  accountId: string,
  filter: ReportFilter,
  page?: number,
  pageSize?: number,
): Promise<AccountTransactionsReportDto> {
  return invokeCommand<AccountTransactionsReportDto>(
    "report_account_transactions",
    {
      accountId,
      filter,
      page: page ?? 1,
      pageSize: pageSize ?? 50,
    },
  );
}

export async function reportAccountBalanceTrend(
  accountId: string,
  filter: ReportFilter,
): Promise<AccountBalanceTrendReportDto> {
  return invokeCommand<AccountBalanceTrendReportDto>(
    "report_account_balance_trend",
    {
      accountId,
      filter,
    },
  );
}

export async function reportAudit(): Promise<AuditReportDto> {
  return invokeCommand<AuditReportDto>("report_audit");
}

// ---------------------------------------------------------------------------
// Data Export Commands
// ---------------------------------------------------------------------------

/**
 * Backup the current database to a specified path.
 */
export async function dbBackup(backupPath: string): Promise<void> {
  return invokeCommand<void>("db_backup", { backupPath });
}

/**
 * Export transactions to CSV format.
 */
export async function exportTransactionsCsv(
  outputPath: string,
  startDate?: string,
  endDate?: string,
): Promise<ExportResult> {
  return invokeCommand<ExportResult>("export_transactions_csv", {
    outputPath,
    startDate: startDate || null,
    endDate: endDate || null,
  });
}

/**
 * Export transactions to Beancount format.
 */
export async function exportTransactionsBeancount(
  outputPath: string,
  startDate?: string,
  endDate?: string,
): Promise<ExportResult> {
  return invokeCommand<ExportResult>("export_transactions_beancount", {
    outputPath,
    startDate: startDate || null,
    endDate: endDate || null,
  });
}

/**
 * Import transactions from CSV format.
 */
export async function importTransactionsCsv(
  inputPath: string,
  options: {
    createMissingAccounts: boolean;
    skipDuplicates: boolean;
  },
): Promise<ImportResult> {
  return invokeCommand<ImportResult>("import_transactions_csv", {
    inputPath,
    createMissingAccounts: options.createMissingAccounts,
    skipDuplicates: options.skipDuplicates,
  });
}
