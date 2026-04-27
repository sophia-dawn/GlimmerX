// ============================================================================
// Core domain types (from DESIGN.md Section 11)
// ============================================================================

export type AccountType =
  | "asset"
  | "liability"
  | "income"
  | "expense"
  | "equity";
export type BudgetPeriod = "monthly" | "weekly";

// ---------------------------------------------------------------------------
// Category
// ---------------------------------------------------------------------------

export interface Category {
  id: string;
  name: string;
  type: "income" | "expense";
  icon: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateCategoryInput {
  name: string;
  type: "income" | "expense";
  icon?: string | null;
}

export interface UpdateCategoryInput {
  name?: string;
  icon?: string | null;
}

export interface CategoryDeletePreview {
  budgetCount: number;
  transactionCount: number;
  canDelete: boolean;
}

// ---------------------------------------------------------------------------
// Budget
// ---------------------------------------------------------------------------

export interface Budget {
  id: string;
  categoryId: string;
  amount: number; // cents
  period: BudgetPeriod;
  rollover: boolean;
  createdAt?: string;
  updatedAt?: string;
}

export interface BudgetStatus extends Budget {
  categoryName: string;
  categoryIcon: string | null;
  spent: number; // cents
  remaining: number; // cents
  overBudget: boolean;
  rolloverAmount: number;
  available: number; // cents - amount + rolloverAmount
}

export interface CreateBudgetInput {
  categoryId: string;
  amount: number; // cents
  period: BudgetPeriod;
  rollover?: boolean;
}

export interface UpdateBudgetInput {
  amount?: number;
  period?: BudgetPeriod;
  rollover?: boolean;
}

// ---------------------------------------------------------------------------
// Rust-side response types
// ---------------------------------------------------------------------------

export interface DbInfo {
  path: string;
  label: string;
  createdAt: string;
}

export interface RecentDbEntry {
  path: string;
  label: string;
  lastOpened: string;
  exists: boolean;
}

// ---------------------------------------------------------------------------
// Command input types
// ---------------------------------------------------------------------------

/** Input for creating a new account using Beancount-style path. */
export interface CreateAccountInput {
  name: string; // full path like "资产/银行/招商银行"
  currency?: string;
  initial_balance?: string | number; // decimal string or cents
  initial_balance_date?: string; // ISO date for opening balance, defaults to today
  description?: string;
  account_number?: string;
  iban?: string;
  is_active?: boolean;
  include_net_worth?: boolean;
  meta?: Record<string, string>;
  equity_account_name?: string;
  opening_balance_name?: string;
}

/** Input for updating an existing account. */
export interface UpdateAccountInput {
  name?: string;
  description?: string;
  account_number?: string;
  initial_balance?: string;
  initial_balance_date?: string;
  iban?: string;
  is_active?: boolean;
  include_net_worth?: boolean;
  meta?: Record<string, string>;
}

// ---------------------------------------------------------------------------
// Account Transaction History
// ---------------------------------------------------------------------------

export interface AccountTransaction {
  id: string;
  date: string;
  description: string;
  category_id: string | null;
  amount: number; // cents, positive = debit, negative = credit
  created_at: string;
  updated_at: string;
}

// ---------------------------------------------------------------------------
// Rust DTO types (response shapes from Tauri commands)
// ---------------------------------------------------------------------------

export interface AccountDto {
  id: string;
  name: string;
  account_type: AccountType;
  currency: string;
  description: string;
  account_number: string | null;
  is_system: boolean;
  iban: string | null;
  is_active: boolean;
  include_net_worth: boolean;
  created_at: string;
  updated_at: string;
  meta: AccountMeta[];
  initial_balance?: number;
  initial_balance_date?: string;
}

export interface AccountMeta {
  id: string;
  account_id: string;
  key: string;
  value: string;
  created_at: string;
}

/** Flat group of accounts by type for display. */
export interface AccountGroup {
  type: AccountType;
  accounts: AccountDto[];
}

export interface AccountMetaSchema {
  valid_account_roles: string[];
  valid_liability_types: string[];
}

// ---------------------------------------------------------------------------
// Transaction DTO types (response shapes from Tauri commands)
// ---------------------------------------------------------------------------

export interface PostingDto {
  id: string;
  transactionId: string;
  accountId: string;
  amount: number; // cents
  sequence: number;
  createdAt: string;
}

export interface TransactionDto {
  id: string;
  date: string; // ISO 8601
  description: string;
  categoryId: string | null;
  isReconciled: boolean;
  createdAt: string;
  updatedAt: string;
  postings: PostingDto[];
}

// ---------------------------------------------------------------------------
// Transaction input types
// ---------------------------------------------------------------------------

export interface CreatePostingInput {
  accountId: string;
  amount: number; // cents
}

export interface CreateTransactionInput {
  date: string; // ISO 8601
  description: string;
  categoryId?: string | null;
  postings: CreatePostingInput[];
}

export interface TransactionListFilter {
  accountId?: string;
  fromDate?: string; // ISO 8601
  toDate?: string; // ISO 8601
}

export interface EnhancedTransactionFilter {
  fromDate?: string;
  toDate?: string;
  minAmount?: number; // cents
  maxAmount?: number; // cents
  accountId?: string;
  categoryId?: string;
  descriptionQuery?: string;
  page?: number;
  pageSize?: number;
  sortBy?: "date" | "amount" | "description";
  sortOrder?: "asc" | "desc";
}

export interface PostingSummary {
  accountName: string;
  amount: number; // cents
}

export interface TransactionListItem {
  id: string;
  date: string;
  description: string;
  categoryId: string | null;
  categoryName: string | null;
  categoryIcon: string | null;
  postingsSummary: PostingSummary[];
  displayAmount: number; // cents, always positive (debit total)
  createdAt: string;
  updatedAt: string;
}

export interface PaginationInfo {
  page: number;
  pageSize: number;
  totalCount: number;
  totalPages: number;
  hasNext: boolean;
  hasPrev: boolean;
}

export interface TransactionDateGroup {
  date: string;
  dateDisplay: string;
  items: TransactionListItem[];
  dayTotal: number; // cents, calculated by backend
}

export interface TransactionListResponse {
  items: TransactionListItem[];
  pagination: PaginationInfo;
  dateGroups: TransactionDateGroup[];
}

// ---------------------------------------------------------------------------
// Quick Add Types
// ---------------------------------------------------------------------------

export interface QuickAddInput {
  mode: "expense" | "income" | "transfer";
  amount: string; // Decimal string like "35.00"
  sourceAccountId?: string;
  destinationAccountId?: string;
  categoryId?: string;
  description?: string;
  date?: string; // ISO 8601
}

// ---------------------------------------------------------------------------
// Transaction Detail Types
// ---------------------------------------------------------------------------

export interface TransactionDetail {
  id: string;
  date: string;
  description: string;
  categoryId: string | null;
  categoryName: string | null;
  postings: PostingDetail[];
  createdAt: string;
  updatedAt: string;
  isBalanced: boolean;
  postingCount: number;
  debitTotal: number;
  creditTotal: number;
}

export interface PostingDetail {
  id: string;
  transactionId: string;
  accountId: string;
  accountName: string;
  accountType: string;
  amount: number;
  amountDisplay: string;
  isDebit: boolean;
  sequence: number;
  createdAt: string;
}

// ---------------------------------------------------------------------------
// Update Transaction Input
// ---------------------------------------------------------------------------

export interface UpdateTransactionInput {
  date?: string;
  description?: string;
  categoryId?: string | null;
  postings?: CreatePostingInput[];
}

// ---------------------------------------------------------------------------
// Transaction Delete Preview
// ---------------------------------------------------------------------------

export interface TransactionDeletePreview {
  transactionId: string;
  description: string;
  date: string;
  postingCount: number;
  canDelete: boolean;
  warningMessage: string | null;
}

export interface AccountBalanceItem {
  id: string;
  balance: number; // cents
}

export * from "./dashboard";
export * from "./report";
export * from "./export";
