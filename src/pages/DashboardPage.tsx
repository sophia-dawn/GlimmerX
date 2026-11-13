import {
  DashboardStatusBar,
  MonthlyIncomeExpenseChart,
  CategoryBreakdownChart,
  TopExpensesList,
  RecentTransactionsCard,
  AccountBalanceList,
  BudgetOverviewCard,
} from "@/components/dashboard";

/**
 * A dashboard, not a column of cards.
 *
 * The page used to stack eight sections vertically — four summary cards, three
 * health cards, then five more panels — which meant two and a half screens of
 * scrolling for what one screen could show. The four figures worth seeing first
 * now share a status strip, and everything else is arranged in three columns by
 * how often it gets read: charts on the left, the transaction table in the
 * middle, budgets and balances on the right.
 *
 * Columns are sized by content weight rather than equally (`1.2fr / 1fr /
 * 0.9fr`): a time axis needs horizontal room, the table needs room for four
 * columns, and the side panels are lists that read fine narrow.
 *
 * The greeting is gone on purpose. The header already says which page this is;
 * a second heading saying "welcome" above four numbers only delays them.
 */
export function DashboardPage() {
  return (
    <div className="flex flex-col gap-3">
      <DashboardStatusBar />

      <div className="grid gap-3 lg:grid-cols-2 xl:grid-cols-[1.2fr_1fr_0.9fr]">
        <div className="flex min-w-0 flex-col gap-3">
          <MonthlyIncomeExpenseChart />
          <CategoryBreakdownChart />
        </div>

        <div className="flex min-w-0 flex-col gap-3">
          <RecentTransactionsCard />
        </div>

        <div className="flex min-w-0 flex-col gap-3">
          <BudgetOverviewCard />
          <AccountBalanceList />
          <TopExpensesList />
        </div>
      </div>
    </div>
  );
}
