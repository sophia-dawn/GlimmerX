import { useTranslation } from "react-i18next";
import { Plus, Edit, Trash2 } from "lucide-react";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  CardAction,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import type { Category } from "@/types";

interface CategoryListProps {
  categories: Category[];
  type: "income" | "expense";
  onEdit: (category: Category) => void;
  onDelete: (category: Category) => void;
  onAdd: () => void;
}

export function CategoryList({
  categories,
  type,
  onEdit,
  onDelete,
  onAdd,
}: CategoryListProps) {
  const { t } = useTranslation();

  const title =
    type === "income"
      ? t("categories.incomeTitle")
      : t("categories.expenseTitle");

  const emptyMessage =
    type === "income"
      ? t("categories.emptyIncome")
      : t("categories.emptyExpense");

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardAction>
          <Button variant="outline" size="sm" onClick={onAdd}>
            <Plus className="h-4 w-4" />
            {t("categories.addRoot")}
          </Button>
        </CardAction>
      </CardHeader>
      <CardContent>
        {categories.length === 0 ? (
          <p className="text-muted-foreground py-4">{emptyMessage}</p>
        ) : (
          <div className="space-y-2">
            {categories.map((category) => (
              <div
                key={category.id}
                className="flex items-center justify-between rounded-lg border p-3 hover:bg-accent/50 transition-colors"
              >
                <div className="flex items-center gap-3">
                  {category.icon && (
                    <span className="text-lg">{category.icon}</span>
                  )}
                  <span className="font-medium">{category.name}</span>
                </div>
                <div className="flex gap-1">
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8"
                    onClick={() => onEdit(category)}
                  >
                    <Edit className="h-4 w-4" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-8 w-8 text-destructive hover:text-destructive"
                    onClick={() => onDelete(category)}
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
