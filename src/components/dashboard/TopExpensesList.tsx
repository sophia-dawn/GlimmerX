import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useTopExpenses } from "@/hooks/useTopExpenses";
import { formatAmount } from "@/utils/format";

/**
 * The largest expenses this month, as a tight list.
 *
 * Like the recent-transactions panel, this used to render one padded card per
 * row — ten panels to show ten numbers. A ranked list is a single column of
 * aligned amounts; boxing each row hid that the whole point is comparing them.
 */
export function TopExpensesList() {
  const { t } = useTranslation();
  const { data, isLoading, error } = useTopExpenses();

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>{t("dashboard.topExpenses")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  const expenses = data?.expenses ?? [];

  return (
    <Card className="flex min-h-[120px] flex-1 flex-col">
      <CardHeader>
        <CardTitle>{t("dashboard.topExpenses")}</CardTitle>
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-auto">
        {isLoading ? (
          <div className="space-y-1.5">
            {[...Array(6)].map((_, i) => (
              <div key={i} className="h-6 animate-pulse rounded-sm bg-muted" />
            ))}
          </div>
        ) : expenses.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.topExpensesEmpty")}
          </p>
        ) : (
          <div className="divide-y divide-border/60">
            {expenses.map((expense) => (
              <Link
                key={expense.posting_id}
                to={`/transactions/${expense.transaction_id}`}
                className="-mx-1 flex items-baseline gap-2 rounded-sm px-1 py-1.5 transition-colors hover:bg-muted/60"
              >
                <span className="shrink-0 whitespace-nowrap font-mono text-xs text-muted-foreground">
                  {expense.date.slice(5)}
                </span>
                <span
                  className="min-w-0 flex-1 truncate"
                  title={
                    expense.category_name
                      ? `${expense.description} · ${expense.category_name}`
                      : expense.description
                  }
                >
                  {expense.description || t("transactions.noDescription")}
                </span>
                <span className="shrink-0 font-mono text-expense">
                  {formatAmount(expense.amount)}
                </span>
              </Link>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
