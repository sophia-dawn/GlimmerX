import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { categoryList } from "@/utils/api";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import type {
  BudgetStatus,
  CreateBudgetInput,
  UpdateBudgetInput,
  Category,
  BudgetPeriod,
} from "@/types";

interface BudgetFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: "create" | "edit";
  budget?: BudgetStatus | null;
  onSubmit: (data: CreateBudgetInput | UpdateBudgetInput) => Promise<void>;
  isLoading: boolean;
}

export function BudgetForm({
  open,
  onOpenChange,
  mode,
  budget,
  onSubmit,
  isLoading,
}: BudgetFormProps) {
  const { t } = useTranslation();

  // Form state
  const [categoryId, setCategoryId] = useState<string>("");
  const [amount, setAmount] = useState<string>("");
  const [period, setPeriod] = useState<BudgetPeriod>("monthly");
  const [rollover, setRollover] = useState<boolean>(false);

  // Fetch expense categories
  // NOTE: Per AGENTS.md, use no caching for all queries
  const { data: categories } = useQuery({
    queryKey: ["categoryList", "expense"],
    queryFn: () => categoryList("expense"),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });

  // Pre-fill form in edit mode
  useEffect(() => {
    if (!open) return;

    if (mode === "edit" && budget) {
      setCategoryId(budget.categoryId);
      // NOTE: This is display formatting (cents → yuan for input field), NOT financial calculation.
      // All monetary calculations (spent, remaining, overBudget) are computed by Rust backend.
      setAmount(String(budget.amount / 100));
      setPeriod(budget.period);
      setRollover(budget.rollover);
    } else if (mode === "create") {
      setCategoryId("");
      setAmount("");
      setPeriod("monthly");
      setRollover(false);
    }
  }, [mode, budget, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    const amountCents = Math.round(parseFloat(amount) * 100);
    if (amountCents <= 0) return;

    if (mode === "create" && !categoryId) return;

    if (mode === "create") {
      await onSubmit({
        categoryId,
        amount: amountCents,
        period,
        rollover,
      });
    } else {
      await onSubmit({
        amount: amountCents,
        period,
        rollover,
      });
    }
  };

  // Filter expense categories
  const availableCategories = categories?.filter(
    (cat: Category) => cat.type === "expense",
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[425px]">
        <DialogHeader>
          <DialogTitle>
            {mode === "create"
              ? t("budgets.createBudget")
              : t("budgets.editBudget")}
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Category select (only in create mode) */}
          {mode === "create" && (
            <div className="space-y-2">
              <Label htmlFor="category">{t("budgets.category")}</Label>
              <Select value={categoryId} onValueChange={setCategoryId}>
                <SelectTrigger id="category">
                  <SelectValue placeholder={t("budgets.selectCategory")} />
                </SelectTrigger>
                <SelectContent>
                  {availableCategories?.map((cat: Category) => (
                    <SelectItem key={cat.id} value={cat.id}>
                      {cat.icon && <span className="mr-1">{cat.icon}</span>}
                      {cat.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {/* Category display in edit mode */}
          {mode === "edit" && budget && (
            <div className="space-y-2">
              <Label>{t("budgets.category")}</Label>
              <div className="flex items-center gap-2 p-2 border rounded-md bg-muted/50">
                {budget.categoryIcon && (
                  <span className="text-lg">{budget.categoryIcon}</span>
                )}
                <span>{budget.categoryName}</span>
              </div>
            </div>
          )}

          {/* Amount input */}
          <div className="space-y-2">
            <Label htmlFor="amount">{t("budgets.amount")}</Label>
            <Input
              id="amount"
              type="number"
              step="0.01"
              min="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              placeholder="0.00"
              required
            />
          </div>

          {/* Period select */}
          <div className="space-y-2">
            <Label htmlFor="period">{t("budgets.periodLabel")}</Label>
            <Select
              value={period}
              onValueChange={(v) => setPeriod(v as BudgetPeriod)}
            >
              <SelectTrigger id="period">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="monthly">
                  {t("budgets.period.monthly")}
                </SelectItem>
                <SelectItem value="weekly">
                  {t("budgets.period.weekly")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* Rollover switch */}
          <div className="flex items-center justify-between">
            <Label htmlFor="rollover">{t("budgets.rollover")}</Label>
            <Switch
              id="rollover"
              checked={rollover}
              onCheckedChange={setRollover}
            />
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              {t("common.cancel")}
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? t("common.processing") : t("common.save")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
