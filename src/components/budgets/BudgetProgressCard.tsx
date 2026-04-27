import { useTranslation } from "react-i18next";
import { AlertTriangle, CheckCircle } from "lucide-react";
import { cn } from "@/lib/utils";
import { formatAmount } from "@/utils/format";
import type { BudgetStatus } from "@/types";

interface BudgetProgressCardProps {
  budget: BudgetStatus;
  onEdit: (budget: BudgetStatus) => void;
  onDelete: (budget: BudgetStatus) => void;
}

export function BudgetProgressCard({
  budget,
  onEdit,
  onDelete,
}: BudgetProgressCardProps) {
  const { t } = useTranslation();

  const percentage =
    budget.available > 0
      ? Math.min(100, Math.round((budget.spent / budget.available) * 100))
      : 0;

  const isOverBudget = budget.overBudget;

  return (
    <div
      className={cn(
        "rounded-lg border p-4 transition-colors",
        isOverBudget
          ? "border-destructive/50 bg-destructive/5"
          : "border-border bg-card",
      )}
    >
      {/* Header: Category info and status icon */}
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-2">
          {budget.categoryIcon && (
            <span className="text-lg">{budget.categoryIcon}</span>
          )}
          <div>
            <h3 className="font-medium">{budget.categoryName}</h3>
            <p className="text-xs text-muted-foreground">
              {t(`budgets.period.${budget.period}`)}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {isOverBudget ? (
            <AlertTriangle className="h-4 w-4 text-destructive" />
          ) : (
            <CheckCircle className="h-4 w-4 text-green-500" />
          )}
          <div className="flex gap-1">
            <button
              onClick={() => onEdit(budget)}
              className="text-xs text-muted-foreground hover:text-foreground"
            >
              {t("common.edit")}
            </button>
            <button
              onClick={() => onDelete(budget)}
              className="text-xs text-muted-foreground hover:text-destructive"
            >
              {t("common.delete")}
            </button>
          </div>
        </div>
      </div>

      {/* Progress bar */}
      <div className="mb-2">
        <div className="h-2 rounded-full bg-muted overflow-hidden">
          <div
            className={cn(
              "h-full rounded-full transition-all",
              isOverBudget ? "bg-destructive" : "bg-primary",
            )}
            style={{ width: `${percentage}%` }}
          />
        </div>
      </div>

      {/* Amount details */}
      <div className="flex justify-between text-sm">
        <div>
          <span className="text-muted-foreground">{t("budgets.spent")}: </span>
          <span className={cn(isOverBudget && "text-destructive font-medium")}>
            {formatAmount(budget.spent)}
          </span>
        </div>
        <div>
          <span className="text-muted-foreground">{t("budgets.limit")}: </span>
          <span>{formatAmount(budget.amount)}</span>
        </div>
      </div>

      {/* Remaining amount */}
      <div className="mt-1 text-sm">
        <span className="text-muted-foreground">
          {t("budgets.remaining")}:{" "}
        </span>
        <span
          className={cn(
            budget.remaining < 0 ? "text-destructive" : "text-green-600",
          )}
        >
          {formatAmount(budget.remaining)}
        </span>
      </div>
    </div>
  );
}
