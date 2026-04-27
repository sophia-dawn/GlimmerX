import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { transactionListPaginated } from "@/utils/api";
import { formatAmount } from "@/utils/format";
import { formatDateLong } from "@/utils/date";
import { ArrowRight } from "lucide-react";
import type { TransactionListItem } from "@/types";
import { DASHBOARD_LIMITS } from "@/constants";

export function RecentTransactionsCard() {
  const { t } = useTranslation();
  const { data, isLoading, error } = useQuery({
    queryKey: ["recent-transactions"],
    queryFn: () =>
      transactionListPaginated({
        pageSize: DASHBOARD_LIMITS.recentTransactions,
        sortBy: "date",
        sortOrder: "desc",
      }),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });

  if (error) {
    return (
      <Card>
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

  // Expense accounts have negative amounts in postingsSummary (double-entry bookkeeping)
  const getTransactionDisplayInfo = (tx: TransactionListItem) => {
    const isExpense = tx.postingsSummary.some((p) => p.amount < 0);
    const amountClass = isExpense ? "text-red-600" : "text-green-600";
    return { amountClass };
  };

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t("dashboard.recentTransactions")}</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link to="/transactions">
            {t("dashboard.viewAll")}
            <ArrowRight className="ml-1 h-4 w-4" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-10 bg-muted rounded animate-pulse" />
            ))}
          </div>
        ) : transactions.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.noTransactions")}
          </p>
        ) : (
          <div className="space-y-3">
            {transactions.map((tx) => {
              const { amountClass } = getTransactionDisplayInfo(tx);

              return (
                <Link
                  key={tx.id}
                  to={`/transactions/${tx.id}`}
                  className="flex items-center justify-between p-3 rounded-lg bg-muted/50 hover:bg-muted transition-colors group"
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      {tx.categoryIcon && (
                        <span className="text-sm">{tx.categoryIcon}</span>
                      )}
                      <span className="font-medium truncate">
                        {tx.description || t("transactions.noDescription")}
                      </span>
                    </div>
                    <div className="flex items-center gap-2 text-muted-foreground text-xs mt-1">
                      <span>{formatDateLong(tx.date)}</span>
                      {tx.categoryName && (
                        <span className="truncate">{tx.categoryName}</span>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <span className={`font-medium ${amountClass}`}>
                      {formatAmount(tx.displayAmount)}
                    </span>
                    <ArrowRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                  </div>
                </Link>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
