import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type {
  Category,
  CreateCategoryInput,
  UpdateCategoryInput,
} from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { IconPicker } from "@/components/categories/IconPicker";

interface CategoryFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: "create" | "edit";
  category?: Category | null;
  onSubmit: (data: CreateCategoryInput | UpdateCategoryInput) => void;
  isLoading?: boolean;
  defaultType?: "income" | "expense";
}

interface FormValues {
  name: string;
  categoryType: "income" | "expense";
  icon: string | null;
}

export function CategoryForm({
  open,
  onOpenChange,
  mode,
  category,
  onSubmit,
  isLoading = false,
  defaultType = "expense",
}: CategoryFormProps) {
  const { t } = useTranslation();
  const isEditing = mode === "edit";

  const [formValues, setFormValues] = useState<FormValues>({
    name: "",
    categoryType: "expense",
    icon: null,
  });
  // 追踪原始图标值，用于判断是否需要发送 icon 字段
  const [originalIcon, setOriginalIcon] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;

    if (isEditing && category) {
      const initialIcon = category.icon ?? null;
      setFormValues({
        name: category.name,
        categoryType: category.type,
        icon: initialIcon,
      });
      setOriginalIcon(initialIcon);
    } else {
      setFormValues({
        name: "",
        categoryType: defaultType,
        icon: null,
      });
      setOriginalIcon(null);
    }
  }, [open, isEditing, category, defaultType]);

  const nameTrimmed = formValues.name.trim();
  const canSave = nameTrimmed.length > 0;

  const handleSubmit = () => {
    if (!canSave) return;

    const data: CreateCategoryInput | UpdateCategoryInput = isEditing
      ? {
          name: nameTrimmed,
          icon:
            formValues.icon !== originalIcon
              ? formValues.icon?.trim() || null
              : undefined,
        }
      : {
          name: nameTrimmed,
          type: formValues.categoryType,
          icon: formValues.icon?.trim() || undefined,
        };

    onSubmit(data);
  };

  const dialogTitle = isEditing
    ? t("categories.editCategory")
    : t("categories.newCategory");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[40%] sm:min-w-[360px]">
        <DialogHeader>
          <DialogTitle>{dialogTitle}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="category-name">{t("categories.name")}</Label>
            <Input
              id="category-name"
              value={formValues.name}
              onChange={(e) =>
                setFormValues((prev) => ({ ...prev, name: e.target.value }))
              }
              placeholder={t("categories.namePlaceholder")}
            />
          </div>

          {!isEditing && (
            <div className="space-y-2">
              <Label htmlFor="category-type">{t("categories.type")}</Label>
              <Select
                value={formValues.categoryType}
                onValueChange={(value) =>
                  setFormValues((prev) => ({
                    ...prev,
                    categoryType: value as "income" | "expense",
                  }))
                }
              >
                <SelectTrigger id="category-type" className="w-full">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="income">
                    {t("categories.typeIncome")}
                  </SelectItem>
                  <SelectItem value="expense">
                    {t("categories.typeExpense")}
                  </SelectItem>
                </SelectContent>
              </Select>
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="category-icon">{t("categories.icon")}</Label>
            <IconPicker
              value={formValues.icon ?? ""}
              onChange={(icon) => setFormValues((prev) => ({ ...prev, icon }))}
            />
          </div>
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={!canSave || isLoading}>
            {isLoading ? t("common.processing") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
