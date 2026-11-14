import { useEffect, useMemo } from "react";
import { Printer } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useHeaderActions } from "@/contexts/HeaderContext";
import { useReportQuery } from "@/hooks/useReportQuery";
import type { ReportId } from "@/constants/report";
import { ReportNav } from "@/components/reports/ReportNav";
import { ReportFilterBar } from "@/components/reports/ReportFilterBar";
import { CashflowReport } from "@/components/reports/CashflowReport";
import { CategoryBreakdownReport } from "@/components/reports/CategoryBreakdownReport";
import { NetWorthAttributionReport } from "@/components/reports/NetWorthAttributionReport";
import { BalanceSheetReport } from "@/components/reports/BalanceSheetReport";
import { AccountDetailReport } from "@/components/reports/AccountDetailReport";
import { BudgetExecutionReport } from "@/components/reports/BudgetExecutionReport";

/**
 * Reports.
 *
 * Two columns: the catalogue on the left, one report on the right, one filter
 * bar on top. The old page put the report picker in the window header and let
 * each report own its filters, so a report change could silently change the
 * period and the header said the opposite of what the body was showing.
 *
 * The selected report and the period both live in the URL, which is what makes
 * the back button work and what makes a report linkable.
 */
function renderReport(
  id: ReportId,
  controller: ReturnType<typeof useReportQuery>,
) {
  switch (id) {
    case "categories":
      return <CategoryBreakdownReport controller={controller} />;
    case "netWorth":
      return <NetWorthAttributionReport controller={controller} />;
    case "balanceSheet":
      return <BalanceSheetReport controller={controller} />;
    case "accounts":
      return <AccountDetailReport controller={controller} />;
    case "budgets":
      return <BudgetExecutionReport controller={controller} />;
    case "cashflow":
    default:
      return <CashflowReport controller={controller} />;
  }
}

export function ReportsPage() {
  const { t } = useTranslation();
  const { setActions } = useHeaderActions();
  const controller = useReportQuery();

  const headerActions = useMemo(
    () => (
      <Button variant="outline" size="sm" onClick={() => window.print()}>
        <Printer className="mr-1.5 size-3.5" strokeWidth={1.75} />
        {t("reports.common.print")}
      </Button>
    ),
    [t],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  return (
    <div className="flex h-full min-h-0 overflow-hidden">
      <ReportNav
        selected={controller.query.report}
        onSelect={controller.selectReport}
      />
      <div
        data-print="page"
        className="flex min-h-0 flex-1 flex-col gap-3 overflow-auto px-4 py-4"
      >
        <ReportFilterBar controller={controller} />
        <div className="pb-6">
          {renderReport(controller.query.report, controller)}
        </div>
      </div>
    </div>
  );
}
