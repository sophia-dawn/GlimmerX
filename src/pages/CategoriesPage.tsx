import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  categoryList,
  categoryCreate,
  categoryUpdate,
  categoryDelete,
  categoryDeletePreview,
} from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { translateErrorMessage } from "@/utils/errorTranslation";
import type {
  Category,
  CreateCategoryInput,
  UpdateCategoryInput,
  CategoryDeletePreview,
} from "@/types";
import { CategoryList } from "@/components/categories/CategoryList";
import { CategoryForm } from "@/components/categories/CategoryForm";
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

export function CategoriesPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [formOpen, setFormOpen] = useState(false);
  const [formMode, setFormMode] = useState<"create" | "edit">("create");
  const [editCategory, setEditCategory] = useState<Category | null>(null);
  const [addType, setAddType] = useState<"income" | "expense">("expense");
  const [deleteCategory, setDeleteCategory] = useState<Category | null>(null);
  const [deletePreview, setDeletePreview] =
    useState<CategoryDeletePreview | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const { data: categories, isLoading } = useQuery({
    queryKey: ["categoryList"],
    queryFn: () => categoryList(),
    ...QUERY_CONFIG.FINANCIAL,
  });

  const incomeCategories = categories?.filter((c) => c.type === "income") ?? [];
  const expenseCategories =
    categories?.filter((c) => c.type === "expense") ?? [];

  const handleAdd = (type: "income" | "expense") => {
    setFormMode("create");
    setEditCategory(null);
    setAddType(type);
    setFormOpen(true);
  };

  const handleEdit = (category: Category) => {
    setFormMode("edit");
    setEditCategory(category);
    setFormOpen(true);
  };

  const handleFormSubmit = async (
    data: CreateCategoryInput | UpdateCategoryInput,
  ) => {
    setIsSubmitting(true);
    try {
      if (formMode === "create") {
        await categoryCreate(data as CreateCategoryInput);
        toast.success(t("categories.created"));
      } else if (editCategory) {
        await categoryUpdate(editCategory.id, data as UpdateCategoryInput);
        toast.success(t("categories.updated"));
      }
      setFormOpen(false);
      await queryClient.refetchQueries({ queryKey: ["categoryList"] });
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteClick = async (category: Category) => {
    setDeleteCategory(category);
    try {
      const preview = await categoryDeletePreview(category.id);
      setDeletePreview(preview);
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
      setDeleteCategory(null);
    }
  };

  const handleDeleteConfirm = async () => {
    if (!deleteCategory) return;
    try {
      await categoryDelete(deleteCategory.id);
      toast.success(t("categories.deleted"));
      await queryClient.refetchQueries({ queryKey: ["categoryList"] });
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setDeleteCategory(null);
      setDeletePreview(null);
    }
  };

  return (
    <div className="h-full overflow-auto p-6">
      {isLoading && (
        <div className="py-12 text-center text-muted-foreground">
          {t("common.loading")}
        </div>
      )}

      {!isLoading && (
        <div className="grid gap-6 md:grid-cols-2">
          <CategoryList
            categories={incomeCategories}
            type="income"
            onEdit={handleEdit}
            onDelete={handleDeleteClick}
            onAdd={() => handleAdd("income")}
          />
          <CategoryList
            categories={expenseCategories}
            type="expense"
            onEdit={handleEdit}
            onDelete={handleDeleteClick}
            onAdd={() => handleAdd("expense")}
          />
        </div>
      )}

      <CategoryForm
        open={formOpen}
        onOpenChange={setFormOpen}
        mode={formMode}
        category={editCategory}
        onSubmit={handleFormSubmit}
        isLoading={isSubmitting}
        defaultType={addType}
      />

      <AlertDialog
        open={!!deleteCategory}
        onOpenChange={(open) => {
          if (!open) {
            setDeleteCategory(null);
            setDeletePreview(null);
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("categories.deleteConfirmTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {deletePreview && deletePreview.budgetCount > 0
                ? t("categories.deleteConfirmDescBudgetsBlock", {
                    name: deleteCategory?.name ?? "",
                    budgets: deletePreview.budgetCount,
                  })
                : deletePreview && deletePreview.transactionCount > 0
                  ? t("categories.deleteConfirmDescOnlyTransactions", {
                      name: deleteCategory?.name ?? "",
                      transactions: deletePreview.transactionCount,
                    })
                  : t("categories.deleteConfirmDesc", {
                      name: deleteCategory?.name ?? "",
                    })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={handleDeleteConfirm}
              disabled={deletePreview !== null && deletePreview.budgetCount > 0}
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
