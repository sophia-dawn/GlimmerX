import { useTranslation } from "react-i18next";
import {
  DashboardSummaryCards,
  FinancialHealthCards,
  MonthlyIncomeExpenseChart,
  CategoryBreakdownChart,
  TopExpensesList,
  RecentTransactionsCard,
  AccountBalanceList,
  BudgetOverviewCard,
} from "@/components/dashboard";

export function DashboardPage() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6 p-6">
      {/* Header */}
      <div>
        <h2 className="text-2xl font-bold tracking-tight">
          {t("dashboard.welcome")}
        </h2>
        <p className="text-muted-foreground">{t("dashboard.subtitle")}</p>
      </div>

      {/* 收支汇总 (4 cards) */}
      <DashboardSummaryCards />

      {/* 财务健康 (3 cards: assets/liabilities/net worth) */}
      <FinancialHealthCards />

      {/* 预算概览 */}
      <BudgetOverviewCard />

      {/* 月度收支图 */}
      <MonthlyIncomeExpenseChart />

      {/* 分类收支 & Top 10 (双列布局) */}
      <div className="grid gap-4 md:grid-cols-2">
        <CategoryBreakdownChart />
        <TopExpensesList />
      </div>

      {/* 最近交易 */}
      <RecentTransactionsCard />

      {/* 账户余额列表 */}
      <AccountBalanceList />
    </div>
  );
}
