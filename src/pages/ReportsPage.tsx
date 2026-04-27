import { useState, useEffect, useMemo, useCallback } from "react";
import {
  StandardReport,
  CategoryBreakdownReport,
  BalanceSheetReport,
  TrendReport,
  MonthComparisonReport,
  YearSummaryReport,
  AccountTransactionsReport,
  AccountBalanceTrend,
  AuditReport,
  ReportTypeSelector,
} from "@/components/reports";
import { useHeaderActions } from "@/contexts/HeaderContext";

export function ReportsPage() {
  const { setActions } = useHeaderActions();
  const [selectedReport, setSelectedReport] = useState("standard");

  const handleSelect = useCallback((id: string) => {
    setSelectedReport(id);
  }, []);

  const headerActions = useMemo(
    () => (
      <ReportTypeSelector selected={selectedReport} onSelect={handleSelect} />
    ),
    [selectedReport, handleSelect],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex-1 p-6 overflow-auto">
        {selectedReport === "standard" && <StandardReport />}
        {selectedReport === "category" && <CategoryBreakdownReport />}
        {selectedReport === "balanceSheet" && <BalanceSheetReport />}
        {selectedReport === "trend" && <TrendReport />}
        {selectedReport === "monthComparison" && <MonthComparisonReport />}
        {selectedReport === "yearSummary" && <YearSummaryReport />}
        {selectedReport === "accountTransactions" && (
          <AccountTransactionsReport />
        )}
        {selectedReport === "accountBalanceTrend" && <AccountBalanceTrend />}
        {selectedReport === "audit" && <AuditReport />}
      </div>
    </div>
  );
}
