import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useTopExpenses } from "@/hooks/useTopExpenses";
import { formatAmount } from "@/utils/format";
import { formatDateShort } from "@/utils/date";
import { ArrowRight } from "lucide-react";

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
    <Card>
      <CardHeader>
        <CardTitle>{t("dashboard.topExpenses")}</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(5)].map((_, i) => (
              <div key={i} className="h-10 bg-muted rounded animate-pulse" />
            ))}
          </div>
        ) : expenses.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.topExpensesEmpty")}
          </p>
        ) : (
          <div className="space-y-3">
            {expenses.map((expense) => (
              <Link
                key={expense.transaction_id}
                to={`/transactions/${expense.transaction_id}`}
                className="flex items-center justify-between p-3 rounded-lg bg-muted/50 hover:bg-muted transition-colors group"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    {expense.category_icon && (
                      <span className="text-sm">{expense.category_icon}</span>
                    )}
                    <span className="font-medium truncate">
                      {expense.description || t("transactions.noDescription")}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 text-muted-foreground text-xs mt-1">
                    <span>{formatDateShort(expense.date)}</span>
                    {expense.category_name && (
                      <span className="truncate">{expense.category_name}</span>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <span className="text-red-600 font-medium">
                    {formatAmount(expense.amount)}
                  </span>
                  <ArrowRight className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity" />
                </div>
              </Link>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
