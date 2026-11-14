import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { transactionListPaginated } from "@/utils/api";
import { formatAmount, formatAmountWithSign } from "@/utils/format";
import { ArrowRight } from "lucide-react";
import type { TransactionListItem } from "@/types";
import { DASHBOARD_LIMITS } from "@/constants";
import { cn } from "@/lib/utils";

/**
 * Recent transactions as a table.
 *
 * Each row used to be a card of its own — three lines of padding, a rounded
 * panel and a background tint per transaction — so eight entries filled the
 * column and no two rows aligned with each other. A table shows three times as
 * many in the same space and, more to the point, puts the amounts in a column:
 * the eye compares magnitudes instead of re-finding the number on every card.
 *
 * The amount is signed by cash flow rather than by `displayAmount`, which is a
 * debit total and always positive — printing it would render every expense as if
 * it were a receipt. Moving money between the user's own accounts nets to zero
 * and is shown unsigned and neutral, because none of it left the household.
 */
export function RecentTransactionsCard() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data, isLoading, error } = useQuery({
    queryKey: ["recent-transactions"],
    queryFn: () =>
      transactionListPaginated({
        pageSize: DASHBOARD_LIMITS.recentTransactions,
        sortBy: "date",
        sortOrder: "desc",
      }),
  });

  if (error) {
    return (
      <Card elevation="raise">
        <CardHeader>
          <CardTitle>{t("dashboard.recentTransactions")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  const transactions = data?.items ?? [];

  const amountOf = (tx: TransactionListItem) =>
    tx.flowAmount === 0
      ? formatAmount(tx.displayAmount)
      : formatAmountWithSign(tx.flowAmount);

  const colorOf = (tx: TransactionListItem) =>
    tx.flowAmount > 0
      ? "text-income"
      : tx.flowAmount < 0
        ? "text-expense"
        : "text-muted-foreground";

  return (
    <Card elevation="raise" className="flex min-h-0 flex-1 flex-col">
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t("dashboard.recentTransactions")}</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link to="/transactions">
            {t("dashboard.viewAll")}
            <ArrowRight className="ml-1 size-3.5" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-auto">
        {isLoading ? (
          <div className="space-y-1.5">
            {[...Array(8)].map((_, i) => (
              <div key={i} className="h-7 animate-pulse rounded-sm bg-muted" />
            ))}
          </div>
        ) : transactions.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.noTransactions")}
          </p>
        ) : (
          <table className="w-full border-collapse text-sm">
            {/* The header sticks: the pane scrolls, the column names do not. It
                carries the rule itself because a sticky `<tr>` border scrolls
                away with the row. */}
            <thead>
              <tr className="text-xs text-muted-foreground">
                <th className="sticky top-0 z-10 w-px border-b border-border bg-card py-1.5 pr-2 text-left font-normal">
                  {t("transactions.date")}
                </th>
                <th className="sticky top-0 z-10 w-full border-b border-border bg-card py-1.5 pr-2 text-left font-normal">
                  {t("transactions.description")}
                </th>
                <th className="sticky top-0 z-10 w-px border-b border-border bg-card py-1.5 pr-2 text-left font-normal">
                  {t("transactions.category")}
                </th>
                <th className="sticky top-0 z-10 w-px border-b border-border bg-card py-1.5 text-right font-normal">
                  {t("transactions.amount")}
                </th>
              </tr>
            </thead>
            <tbody>
              {transactions.map((tx) => {
                const go = () => navigate(`/transactions/${tx.id}`);
                return (
                  <tr
                    key={tx.id}
                    role="link"
                    tabIndex={0}
                    onClick={go}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" || event.key === " ") {
                        event.preventDefault();
                        go();
                      }
                    }}
                    className="cursor-pointer border-b border-border/60 transition-colors last:border-0 hover:bg-muted/60 focus-visible:bg-muted/60 focus-visible:outline-none"
                  >
                    <td className="w-px whitespace-nowrap py-2 pr-2 font-mono text-xs text-muted-foreground">
                      {tx.date.slice(5)}
                    </td>
                    <td className="w-full max-w-0 truncate py-2 pr-2">
                      {tx.description || t("transactions.noDescription")}
                    </td>
                    <td className="w-px max-w-[5.5rem] truncate whitespace-nowrap py-2 pr-2 text-xs text-muted-foreground">
                      {tx.categoryNames.length > 0
                        ? tx.categoryNames.join(t("common.listSeparator"))
                        : "—"}
                    </td>
                    <td
                      className={cn(
                        "w-px whitespace-nowrap py-2 text-right font-mono",
                        colorOf(tx),
                      )}
                    >
                      {amountOf(tx)}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </CardContent>
    </Card>
  );
}
