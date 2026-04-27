import { useTranslation } from "react-i18next";
import { BudgetProgressCard } from "./BudgetProgressCard";
import type { BudgetStatus } from "@/types";

interface BudgetListProps {
  budgets: BudgetStatus[];
  onEdit: (budget: BudgetStatus) => void;
  onDelete: (budget: BudgetStatus) => void;
}

export function BudgetList({ budgets, onEdit, onDelete }: BudgetListProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <h2 className="text-lg font-semibold">{t("budgets.expenseBudgets")}</h2>

      {budgets.length === 0 ? (
        <div className="py-12 text-center text-muted-foreground">
          {t("budgets.noBudgets")}
        </div>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {budgets.map((budget) => (
            <BudgetProgressCard
              key={budget.id}
              budget={budget}
              onEdit={onEdit}
              onDelete={onDelete}
            />
          ))}
        </div>
      )}
    </div>
  );
}
