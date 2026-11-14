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
 * The grid fills the viewport instead of growing with its content, and each
 * pane scrolls internally. That is what keeps the three columns ending on the
 * same line: card heights used to be decided by content, so a column with two
 * charts was 200 px taller than a column with one table and the page looked
 * unfinished. `h-full` is what makes the column heights definite, and it has to
 * stay on the page root — `min-h-full` would let the columns collapse back to
 * their content.
 *
 * The greeting is gone on purpose. The header already says which page this is;
 * a second heading saying "welcome" above four numbers only delays them.
 */
export function DashboardPage() {
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <DashboardStatusBar />

      <div
        data-print="panes"
        className="grid min-h-0 flex-1 gap-3 overflow-auto lg:grid-cols-2 xl:grid-cols-[1.2fr_1fr_0.9fr]"
      >
        <div className="flex min-h-0 min-w-0 flex-col gap-3">
          <MonthlyIncomeExpenseChart />
          <CategoryBreakdownChart />
        </div>

        <div className="flex min-h-0 min-w-0 flex-col gap-3">
          <RecentTransactionsCard />
        </div>

        <div className="flex min-h-0 min-w-0 flex-col gap-3">
          <BudgetOverviewCard />
          <AccountBalanceList />
          <TopExpensesList />
        </div>
      </div>
    </div>
  );
}
