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
      <Card>
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
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t("dashboard.budgetOverview")}</CardTitle>
        <Button variant="ghost" size="sm" asChild>
          <Link to="/budgets">
            {t("dashboard.viewAllBudgets")}
            <ArrowRight className="ml-1 h-4 w-4" />
          </Link>
        </Button>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="h-12 bg-muted rounded animate-pulse" />
            ))}
          </div>
        ) : budgetList.length === 0 ? (
          <p className="text-muted-foreground text-sm">
            {t("dashboard.noBudgetsSet")}
          </p>
        ) : (
          <div className="space-y-4">
            {visibleBudgets.map((budget) => {
              const percentage = getProgressPercentage(
                budget.spent,
                budget.available,
              );
              const isOverBudget = budget.overBudget;

              return (
                <div key={budget.id} className="space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      {budget.categoryIcon && (
                        <span className="text-base">{budget.categoryIcon}</span>
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
                        <AlertTriangle className="h-4 w-4 text-destructive" />
                      )}
                    </div>
                    <div className="flex items-center gap-2 text-sm">
                      <span
                        className={cn(
                          "font-medium",
                          isOverBudget ? "text-destructive" : "text-red-600",
                        )}
                      >
                        {formatAmount(budget.spent)}
                      </span>
                      <span className="text-muted-foreground">/</span>
                      <span className="font-medium">
                        {formatAmount(budget.amount)}
                      </span>
                    </div>
                  </div>
                  <div className="h-2 rounded-full bg-muted overflow-hidden">
                    <div
                      className={cn(
                        "h-full rounded-full transition-all",
                        isOverBudget ? "bg-destructive" : "bg-primary",
                      )}
                      style={{ width: `${percentage}%` }}
                    />
                  </div>
                  {isOverBudget && (
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">
                        {t("budgets.overBudget")}
                      </span>
                      <span className="text-destructive font-medium">
                        {formatAmount(budget.remaining)}
                      </span>
                    </div>
                  )}
                </div>
              );
            })}
            {hasMore && (
              <div className="pt-2 text-muted-foreground text-xs text-center">
                +{budgetList.length - MAX_VISIBLE_BUDGETS} more
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
