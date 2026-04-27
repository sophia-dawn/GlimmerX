export interface ExportResult {
  transactionCount: number;
  postingCount: number;
}

export interface ImportError {
  rowNumber: number;
  transactionId: string;
  message: string;
}

export interface ImportResult {
  importedCount: number;
  skippedCount: number;
  errorCount: number;
  createdAccounts: string[];
  errors: ImportError[];
}
