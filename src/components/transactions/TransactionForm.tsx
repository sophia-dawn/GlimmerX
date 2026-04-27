import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import type {
  TransactionDto,
  CreateTransactionInput,
  AccountDto,
  Category,
} from "@/types";
import { formatAmount } from "@/utils/format";
import { todayLocalDate } from "@/utils/date";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DatePicker } from "@/components/ui/date-picker";
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
import {
  Plus,
  Trash2,
  AlertCircle,
  HelpCircle,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

// Special value for "no category" - Radix UI Select.Item cannot have empty string as value
const NO_CATEGORY_VALUE = "__none__";
// Backend marker to explicitly clear category_id on update (partial update semantics)
const CLEAR_CATEGORY_MARKER = "__clear__";

interface PostingFormValue {
  accountId: string;
  amount: string;
}

interface TransactionFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  mode: "create" | "edit";
  transaction?: TransactionDto | null;
  accounts: AccountDto[];
  categories: Category[];
  onSubmit: (data: CreateTransactionInput) => void;
  isLoading?: boolean;
}

function decimalToCents(decimal: string): number | null {
  const value = parseFloat(decimal);
  if (isNaN(value)) return null;
  const cents = Math.round(value * 100);
  if (cents === 0 && value !== 0) return null;
  return cents;
}

function hasExcessPrecision(decimal: string): boolean {
  if (/e/i.test(decimal)) return true;
  const parts = decimal.split(".");
  if (parts.length === 1) return false;
  const decimalPart = parts[1];
  if (decimalPart === undefined) return false;
  return decimalPart.length > 2;
}

function hasZeroCents(decimal: string): boolean {
  const cents = decimalToCents(decimal);
  return cents === null && decimal.trim().length > 0;
}

export function TransactionForm({
  open,
  onOpenChange,
  mode,
  transaction,
  accounts,
  categories,
  onSubmit,
  isLoading = false,
}: TransactionFormProps) {
  const { t } = useTranslation();
  const isEditing = mode === "edit";

  const [date, setDate] = useState("");
  const [description, setDescription] = useState("");
  const [categoryId, setCategoryId] = useState<string>(NO_CATEGORY_VALUE);
  const [postings, setPostings] = useState<PostingFormValue[]>([]);
  const [precisionWarnings, setPrecisionWarnings] = useState<Set<number>>(
    new Set(),
  );
  const [zeroCentsWarnings, setZeroCentsWarnings] = useState<Set<number>>(
    new Set(),
  );
  const [showHelp, setShowHelp] = useState(false);

  useEffect(() => {
    if (!open) {
      setPrecisionWarnings(new Set());
      setZeroCentsWarnings(new Set());
      return;
    }

    if (isEditing && transaction) {
      setDate(transaction.date);
      setDescription(transaction.description || "");
      setCategoryId(transaction.categoryId ?? NO_CATEGORY_VALUE);
      // Convert amount from cents to decimal string for display
      setPostings(
        transaction.postings.map((p) => ({
          accountId: p.accountId,
          amount: (p.amount / 100).toFixed(2),
        })),
      );
    } else {
      setDate(todayLocalDate());
      setDescription("");
      setCategoryId(NO_CATEGORY_VALUE);
      setPostings([
        { accountId: "", amount: "" },
        { accountId: "", amount: "" },
      ]);
    }
  }, [open, isEditing, transaction]);

  const totalCents = postings.reduce((sum, p) => {
    const cents = decimalToCents(p.amount);
    return cents === null ? sum : sum + cents;
  }, 0);
  const isBalanced = totalCents === 0;

  const hasValidDate = date.trim().length > 0;
  const hasMinPostings = postings.length >= 2;
  const allPostingsHaveAccount = postings.every(
    (p) => p.accountId.trim().length > 0,
  );
  const allPostingsHaveValidAmount = postings.every((p) => {
    const cents = decimalToCents(p.amount);
    return cents !== null && cents !== 0;
  });
  const hasDuplicateAccounts = (() => {
    const accountIds = postings
      .filter((p) => p.accountId.trim().length > 0)
      .map((p) => p.accountId);
    const uniqueIds = new Set(accountIds);
    return accountIds.length !== uniqueIds.size;
  })();

  const canSubmit =
    hasValidDate &&
    hasMinPostings &&
    allPostingsHaveAccount &&
    allPostingsHaveValidAmount &&
    isBalanced &&
    !hasDuplicateAccounts &&
    !isLoading;

  const addPosting = () => {
    setPostings((prev) => [...prev, { accountId: "", amount: "" }]);
  };

  const removePosting = (index: number) => {
    setPostings((prev) => {
      if (prev.length <= 2) return prev;
      const next = [...prev];
      next.splice(index, 1);
      return next;
    });
  };

  const updatePosting = (
    index: number,
    field: keyof PostingFormValue,
    value: string,
  ) => {
    setPostings((prev) => {
      const next = [...prev];
      const existing = next[index];
      if (existing) {
        next[index] = { ...existing, [field]: value };
      }
      return next;
    });

    if (field === "amount") {
      setPrecisionWarnings((prev) => {
        const next = new Set(prev);
        if (hasExcessPrecision(value)) {
          next.add(index);
        } else {
          next.delete(index);
        }
        return next;
      });
      setZeroCentsWarnings((prev) => {
        const next = new Set(prev);
        if (hasZeroCents(value)) {
          next.add(index);
        } else {
          next.delete(index);
        }
        return next;
      });
    }
  };

  const handleSubmit = () => {
    if (!canSubmit) return;

    const data: CreateTransactionInput = {
      date: date.trim(),
      description: description.trim(),
      categoryId:
        categoryId === NO_CATEGORY_VALUE
          ? isEditing
            ? CLEAR_CATEGORY_MARKER
            : null
          : categoryId,
      postings: postings.map((p) => ({
        accountId: p.accountId,
        amount: decimalToCents(p.amount) ?? 0,
      })),
    };

    onSubmit(data);
  };

  const dialogTitle = isEditing
    ? t("common.edit")
    : t("transactions.formTitle");

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[40%] sm:min-w-[420px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{dialogTitle}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-2">
            <Label>{t("transactions.date")}</Label>
            <DatePicker
              value={date}
              onChange={setDate}
              placeholder={t("transactions.date")}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="transaction-description">
              {t("transactions.description")}
            </Label>
            <Input
              id="transaction-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t("transactions.descriptionPlaceholder")}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="transaction-category">
              {t("transactions.category")}
              <span className="text-muted-foreground text-xs ml-1">
                ({t("transactions.categoryOptional")})
              </span>
            </Label>
            <Select value={categoryId} onValueChange={setCategoryId}>
              <SelectTrigger id="transaction-category">
                <SelectValue placeholder={t("transactions.selectCategory")} />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_CATEGORY_VALUE}>
                  {t("transactions.noCategory")}
                </SelectItem>
                {categories.map((category) => (
                  <SelectItem key={category.id} value={category.id}>
                    {category.icon ? `${category.icon} ` : ""}
                    {category.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>{t("transactions.postings")}</Label>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setShowHelp(!showHelp)}
                className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground"
              >
                <HelpCircle className="h-3 w-3 mr-1" />
                {t("transactions.help.toggle")}
                {showHelp ? (
                  <ChevronUp className="h-3 w-3 ml-1" />
                ) : (
                  <ChevronDown className="h-3 w-3 ml-1" />
                )}
              </Button>
            </div>

            {showHelp && (
              <div className="p-3 rounded-md border bg-muted/50 text-sm space-y-2">
                <p className="font-medium">{t("transactions.help.title")}</p>
                <ul className="list-disc list-inside space-y-1 text-muted-foreground">
                  <li>{t("transactions.help.rule1")}</li>
                  <li>{t("transactions.help.rule2")}</li>
                  <li>{t("transactions.help.rule3")}</li>
                </ul>
                <div className="mt-2 p-2 rounded bg-background/50">
                  <p className="font-medium text-xs mb-1">
                    {t("transactions.help.exampleTitle")}
                  </p>
                  <div className="text-xs space-y-1 text-muted-foreground">
                    <p>{t("transactions.help.example1")}</p>
                    <p>{t("transactions.help.example2")}</p>
                  </div>
                </div>
              </div>
            )}

            {postings.map((posting, index) => (
              <div
                key={index}
                className="flex items-center gap-2 p-2 border rounded-md bg-muted/30"
              >
                <Select
                  value={posting.accountId}
                  onValueChange={(value) =>
                    updatePosting(index, "accountId", value)
                  }
                >
                  <SelectTrigger className="flex-1">
                    <SelectValue
                      placeholder={t("transactions.selectAccount")}
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {accounts.map((account) => (
                      <SelectItem key={account.id} value={account.id}>
                        {account.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>

                <Input
                  type="number"
                  step="0.01"
                  placeholder={t("transactions.amount")}
                  value={posting.amount}
                  onChange={(e) =>
                    updatePosting(index, "amount", e.target.value)
                  }
                  className={cn(
                    "w-[120px] tabular-nums",
                    precisionWarnings.has(index) && "border-yellow-500",
                    zeroCentsWarnings.has(index) && "border-red-500",
                  )}
                />
                {precisionWarnings.has(index) && (
                  <span className="text-xs text-yellow-600 ml-1 whitespace-nowrap">
                    {t("transactions.precisionWarning")}
                  </span>
                )}
                {zeroCentsWarnings.has(index) && (
                  <span className="text-xs text-red-600 ml-1 whitespace-nowrap">
                    {t("errors.transaction.amountTooSmall")}
                  </span>
                )}

                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  onClick={() => removePosting(index)}
                  disabled={postings.length <= 2}
                  className="shrink-0"
                  aria-label={t("common.remove")}
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              </div>
            ))}

            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={addPosting}
              className="w-full"
            >
              <Plus className="h-4 w-4 mr-1" />
              {t("transactions.addPosting")}
            </Button>
          </div>

          <div className="flex items-center justify-between p-3 rounded-md border bg-card">
            <span className="text-sm text-muted-foreground">
              {t("transactions.balanceCheck")}
            </span>
            <span
              className={cn(
                "text-sm font-medium",
                isBalanced ? "text-green-600" : "text-red-600",
              )}
            >
              {isBalanced
                ? t("transactions.balanced")
                : t("transactions.unbalanced", {
                    diff: formatAmount(Math.abs(totalCents)),
                  })}
            </span>
          </div>

          {hasDuplicateAccounts && (
            <div className="flex items-center gap-2 p-3 rounded-md border bg-destructive/10 text-destructive">
              <AlertCircle className="h-4 w-4" />
              <span className="text-sm">
                {t("transactions.errors.duplicateAccount")}
              </span>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button onClick={handleSubmit} disabled={!canSubmit}>
            {isLoading ? t("common.processing") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
