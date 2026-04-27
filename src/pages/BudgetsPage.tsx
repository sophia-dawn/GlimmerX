import { useState, useEffect, useMemo, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Plus } from "lucide-react";
import {
  budgetListStatuses,
  budgetCreate,
  budgetUpdate,
  budgetDelete,
} from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { todayLocalDate } from "@/utils/date";
import type {
  BudgetStatus,
  CreateBudgetInput,
  UpdateBudgetInput,
} from "@/types";
import { BudgetList } from "@/components/budgets/BudgetList";
import { BudgetForm } from "@/components/budgets/BudgetForm";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { useHeaderActions } from "@/contexts/HeaderContext";

export function BudgetsPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { setActions } = useHeaderActions();

  // Get current year/month from date
  const today = new Date(todayLocalDate());
  const currentYear = today.getFullYear();
  const currentMonth = today.getMonth() + 1;

  // Dialog state
  const [formOpen, setFormOpen] = useState(false);
  const [formMode, setFormMode] = useState<"create" | "edit">("create");
  const [editBudget, setEditBudget] = useState<BudgetStatus | null>(null);
  const [deleteBudget, setDeleteBudget] = useState<BudgetStatus | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Handlers
  const handleAdd = useCallback(() => {
    setFormMode("create");
    setEditBudget(null);
    setFormOpen(true);
  }, []);

  const headerActions = useMemo(
    () => (
      <Button size="sm" onClick={handleAdd}>
        <Plus className="h-4 w-4 mr-1" />
        {t("budgets.addBudget")}
      </Button>
    ),
    [t, handleAdd],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  // Fetch budget statuses for current month
  const { data: budgets, isLoading } = useQuery({
    queryKey: ["budgetStatuses", currentYear, currentMonth],
    queryFn: () => budgetListStatuses(currentYear, currentMonth),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });

  const handleEdit = (budget: BudgetStatus) => {
    setFormMode("edit");
    setEditBudget(budget);
    setFormOpen(true);
  };

  const handleDeleteClick = (budget: BudgetStatus) => {
    setDeleteBudget(budget);
  };

  const handleFormSubmit = async (
    data: CreateBudgetInput | UpdateBudgetInput,
  ) => {
    setIsSubmitting(true);
    try {
      if (formMode === "create") {
        await budgetCreate(data as CreateBudgetInput);
        toast.success(t("budgets.created"));
      } else if (editBudget) {
        await budgetUpdate(editBudget.id, data as UpdateBudgetInput);
        toast.success(t("budgets.updated"));
      }
      setFormOpen(false);
      await queryClient.refetchQueries({
        queryKey: ["budgetStatuses", currentYear, currentMonth],
      });
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteConfirm = async () => {
    if (!deleteBudget) return;
    try {
      await budgetDelete(deleteBudget.id);
      toast.success(t("budgets.deleted"));
      await queryClient.refetchQueries({
        queryKey: ["budgetStatuses", currentYear, currentMonth],
      });
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setDeleteBudget(null);
    }
  };

  return (
    <div className="h-full overflow-auto p-6">
      {/* Loading state */}
      {isLoading && (
        <div className="py-12 text-center text-muted-foreground">
          {t("common.loading")}
        </div>
      )}

      {/* Budget list */}
      {!isLoading && budgets && (
        <BudgetList
          budgets={budgets}
          onEdit={handleEdit}
          onDelete={handleDeleteClick}
        />
      )}

      {/* Create/Edit form dialog */}
      <BudgetForm
        open={formOpen}
        onOpenChange={setFormOpen}
        mode={formMode}
        budget={editBudget}
        onSubmit={handleFormSubmit}
        isLoading={isSubmitting}
      />

      {/* Delete confirmation dialog */}
      <AlertDialog
        open={!!deleteBudget}
        onOpenChange={(open) => {
          if (!open) setDeleteBudget(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("budgets.deleteConfirmTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("budgets.deleteConfirmDesc", {
                category: deleteBudget?.categoryName ?? "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteConfirm}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t("common.delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
