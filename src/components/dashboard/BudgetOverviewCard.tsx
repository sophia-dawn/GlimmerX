import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { useBudgetStatuses } from "@/hooks/useBudgetStatuses";
import { formatAmount } from "@/utils/format";
import { ArrowRight, AlertTriangle } from "lucide-react";
import { cn } from "@/lib/utils";

const MAX_VISIBLE_BUDGETS = 4;

export function BudgetOverviewCard() {
  const { t } = useTranslation();
  const { data: budgets, isLoading, error } = useBudgetStatuses();

  if (error) {
    return (
      <Card elevation="raise">
        <CardHeader className="flex flex-row items-center justify-between">
          <CardTitle>{t("dashboard.budgetOverview")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  const budgetList = budgets ?? [];
  const hasMore = budgetList.length > MAX_VISIBLE_BUDGETS;
  const visibleBudgets = hasMore
    ? budgetList.slice(0, MAX_VISIBLE_BUDGETS)
    : budgetList;

  const getProgressPercentage = (spent: number, amount: number): number => {
    if (amount <= 0) return 0;
    const percentage = (spent / amount) * 100;
    return Math.min(percentage, 100);
  };

  return (
    <Card elevation="raise" className="flex min-h-[120px] flex-1 flex-col">
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t("dashboard.budgetOverview")}</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link to="/budgets">
            {t("dashboard.viewAllBudgets")}
            <ArrowRight className="ml-1 size-3.5" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-auto">
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="h-10 animate-pulse rounded-sm bg-muted" />
            ))}
          </div>
        ) : budgetList.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.noBudgetsSet")}
          </p>
        ) : (
          <div className="space-y-2.5">
            {visibleBudgets.map((budget) => {
              const percentage = getProgressPercentage(
                budget.spent,
                budget.available,
              );
              const isOverBudget = budget.overBudget;

              return (
                <div key={budget.id} className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5">
                      {budget.categoryIcon && (
                        <span className="text-sm">{budget.categoryIcon}</span>
                      )}
                      <span
                        className={cn(
                          "font-medium text-sm",
                          isOverBudget && "text-destructive",
                        )}
                      >
                        {budget.categoryName}
                      </span>
                      {isOverBudget && (
                        <AlertTriangle className="size-3.5 text-destructive" />
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-sm">
                      <span
                        className={cn(
                          "font-mono font-medium",
                          isOverBudget ? "text-destructive" : "text-expense",
                        )}
                      >
                        {formatAmount(budget.spent)}
                      </span>
                      <span className="text-muted-foreground">/</span>
                      <span className="font-mono font-medium">
                        {formatAmount(budget.amount)}
                      </span>
                    </div>
                  </div>
                  <div className="h-1 overflow-hidden bg-muted">
                    <div
                      className={cn(
                        "h-full transition-all",
                        isOverBudget ? "bg-destructive" : "bg-signal",
                      )}
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                  {isOverBudget && (
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">
                        {t("budgets.overBudget")}
                      </span>
                      <span className="font-mono text-destructive font-medium">
                        {formatAmount(budget.remaining)}
                      </span>
                    </div>
                  )}
                </div>
              );
            })}
            {hasMore && (
              <div className="pt-1 text-center text-xs text-muted-foreground">
                {t("dashboard.moreBudgets", {
                  n: budgetList.length - MAX_VISIBLE_BUDGETS,
                })}
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
