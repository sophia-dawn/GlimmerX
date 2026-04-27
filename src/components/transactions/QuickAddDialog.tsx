import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import type { QuickAddInput } from "@/types";
import { accountList, categoryList, quickAddTransaction } from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { translateErrorMessage } from "@/utils/errorTranslation";
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
  ArrowDownCircle,
  ArrowUpCircle,
  ArrowRightCircle,
  HelpCircle,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

const NO_CATEGORY_VALUE = "__none__";

type TransactionMode = "expense" | "income" | "transfer";

interface QuickAddDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function QuickAddDialog({
  open,
  onOpenChange,
  onSuccess,
}: QuickAddDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [mode, setMode] = useState<TransactionMode>("expense");
  const [amount, setAmount] = useState("");
  const [sourceAccountId, setSourceAccountId] = useState("");
  const [destinationAccountId, setDestinationAccountId] = useState("");
  const [categoryId, setCategoryId] = useState<string>(NO_CATEGORY_VALUE);
  const [description, setDescription] = useState("");
  const [date, setDate] = useState("");
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [showHelp, setShowHelp] = useState(false);

  // Clear account state when mode changes (not just on dialog open)
  useEffect(() => {
    if (!open) return;
    // Clear the opposite side's account when mode changes
    if (mode === "expense") {
      setDestinationAccountId("");
    } else if (mode === "income") {
      setSourceAccountId("");
    }
    // Reset category when switching to transfer (no category needed)
    if (mode === "transfer") {
      setCategoryId(NO_CATEGORY_VALUE);
    }
  }, [mode, open]);

  const { data: accounts = [] } = useQuery({
    queryKey: ["accounts"],
    queryFn: accountList,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const { data: categories = [] } = useQuery({
    queryKey: ["categories"],
    queryFn: () => categoryList(),
    ...QUERY_CONFIG.FINANCIAL,
  });

  const assetAccounts = accounts.filter(
    (a) => a.account_type === "asset" && a.is_active,
  );

  const expenseCategories = categories.filter((c) => c.type === "expense");
  const incomeCategories = categories.filter((c) => c.type === "income");

  useEffect(() => {
    if (!open) {
      setSubmitError(null);
      return;
    }

    setMode("expense");
    setAmount("");
    setSourceAccountId("");
    setDestinationAccountId("");
    setCategoryId(NO_CATEGORY_VALUE);
    setDescription("");
    setDate(todayLocalDate());
    setSubmitError(null);
  }, [open]);

  const mutation = useMutation({
    mutationFn: (input: QuickAddInput) => quickAddTransaction(input),
    onSuccess: () => {
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === "transactionListPaginated" ||
          query.queryKey[0] === "transactionDetail",
      });
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      onOpenChange(false);
      onSuccess?.();
    },
    onError: (err) => {
      setSubmitError(translateErrorMessage(err, t));
    },
  });

  const parsedAmount = parseFloat(amount);
  const hasValidAmount = !isNaN(parsedAmount) && parsedAmount > 0;
  const hasSourceAccount =
    mode === "expense" || mode === "transfer"
      ? sourceAccountId.length > 0
      : true;
  const hasDestinationAccount =
    mode === "income" || mode === "transfer"
      ? destinationAccountId.length > 0
      : true;
  const accountsAreDifferent =
    mode === "transfer" ? sourceAccountId !== destinationAccountId : true;
  const hasValidDate = date.trim().length > 0;

  const canSubmit =
    hasValidAmount &&
    hasSourceAccount &&
    hasDestinationAccount &&
    accountsAreDifferent &&
    hasValidDate &&
    !mutation.isPending;

  const handleSubmit = () => {
    if (!canSubmit) return;

    const input: QuickAddInput = {
      mode,
      amount: parsedAmount.toFixed(2),
      sourceAccountId:
        mode === "expense" || mode === "transfer" ? sourceAccountId : undefined,
      destinationAccountId:
        mode === "income" || mode === "transfer"
          ? destinationAccountId
          : undefined,
      categoryId: categoryId === NO_CATEGORY_VALUE ? undefined : categoryId,
      description: description.trim() || undefined,
      date,
    };

    mutation.mutate(input);
  };

  const modeButtons: Array<{
    mode: TransactionMode;
    icon: React.ReactNode;
    label: string;
    colorClass: string;
  }> = [
    {
      mode: "expense",
      icon: <ArrowDownCircle className="h-4 w-4" />,
      label: t("quickAdd.expense"),
      colorClass: "text-red-500",
    },
    {
      mode: "income",
      icon: <ArrowUpCircle className="h-4 w-4" />,
      label: t("quickAdd.income"),
      colorClass: "text-green-500",
    },
    {
      mode: "transfer",
      icon: <ArrowRightCircle className="h-4 w-4" />,
      label: t("quickAdd.transfer"),
      colorClass: "text-blue-500",
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t("quickAdd.title")}</DialogTitle>
        </DialogHeader>

        <div className="space-y-4">
          <div className="flex gap-2">
            {modeButtons.map((btn) => (
              <Button
                key={btn.mode}
                type="button"
                variant={mode === btn.mode ? "default" : "outline"}
                size="sm"
                onClick={() => setMode(btn.mode)}
                className={cn(
                  "flex-1 gap-1",
                  mode === btn.mode &&
                    btn.mode === "expense" &&
                    "bg-red-500 hover:bg-red-600",
                  mode === btn.mode &&
                    btn.mode === "income" &&
                    "bg-green-500 hover:bg-green-600",
                  mode === btn.mode &&
                    btn.mode === "transfer" &&
                    "bg-blue-500 hover:bg-blue-600",
                  mode !== btn.mode && btn.colorClass,
                )}
              >
                {btn.icon}
                {btn.label}
              </Button>
            ))}
          </div>

          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">
              {mode === "expense" && t(`quickAdd.help.expenseTitle`)}
              {mode === "income" && t(`quickAdd.help.incomeTitle`)}
              {mode === "transfer" && t(`quickAdd.help.transferTitle`)}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setShowHelp(!showHelp)}
              className="h-6 px-2 text-xs text-muted-foreground hover:text-foreground"
            >
              <HelpCircle className="h-3 w-3 mr-1" />
              {t("quickAdd.help.toggle")}
              {showHelp ? (
                <ChevronUp className="h-3 w-3 ml-1" />
              ) : (
                <ChevronDown className="h-3 w-3 ml-1" />
              )}
            </Button>
          </div>

          {showHelp && (
            <div className="p-3 rounded-md border bg-muted/50 text-sm">
              {mode === "expense" && (
                <p className="text-muted-foreground">
                  {t("quickAdd.help.expenseDesc")}
                </p>
              )}
              {mode === "income" && (
                <p className="text-muted-foreground">
                  {t("quickAdd.help.incomeDesc")}
                </p>
              )}
              {mode === "transfer" && (
                <p className="text-muted-foreground">
                  {t("quickAdd.help.transferDesc")}
                </p>
              )}
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="quick-add-amount">{t("transactions.amount")}</Label>
            <Input
              id="quick-add-amount"
              type="number"
              step="0.01"
              min="0"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="tabular-nums"
            />
          </div>

          {mode === "expense" && (
            <div className="space-y-2">
              <Label htmlFor="source-account">
                {t("quickAdd.sourceAccount")}
              </Label>
              <Select
                value={sourceAccountId}
                onValueChange={setSourceAccountId}
              >
                <SelectTrigger id="source-account">
                  <SelectValue placeholder={t("transactions.selectAccount")} />
                </SelectTrigger>
                <SelectContent>
                  {assetAccounts.map((account) => (
                    <SelectItem key={account.id} value={account.id}>
                      {account.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {mode === "expense" && (
            <div className="space-y-2">
              <Label htmlFor="expense-category">
                {t("transactions.category")}
                <span className="text-muted-foreground text-xs ml-1">
                  ({t("transactions.categoryOptional")})
                </span>
              </Label>
              <Select value={categoryId} onValueChange={setCategoryId}>
                <SelectTrigger id="expense-category">
                  <SelectValue placeholder={t("transactions.selectCategory")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NO_CATEGORY_VALUE}>
                    {t("transactions.noCategory")}
                  </SelectItem>
                  {expenseCategories.map((category) => (
                    <SelectItem key={category.id} value={category.id}>
                      {category.icon ? `${category.icon} ` : ""}
                      {category.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {mode === "income" && (
            <div className="space-y-2">
              <Label htmlFor="destination-account">
                {t("quickAdd.destinationAccount")}
              </Label>
              <Select
                value={destinationAccountId}
                onValueChange={setDestinationAccountId}
              >
                <SelectTrigger id="destination-account">
                  <SelectValue placeholder={t("transactions.selectAccount")} />
                </SelectTrigger>
                <SelectContent>
                  {assetAccounts.map((account) => (
                    <SelectItem key={account.id} value={account.id}>
                      {account.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {mode === "income" && (
            <div className="space-y-2">
              <Label htmlFor="income-category">
                {t("transactions.category")}
                <span className="text-muted-foreground text-xs ml-1">
                  ({t("transactions.categoryOptional")})
                </span>
              </Label>
              <Select value={categoryId} onValueChange={setCategoryId}>
                <SelectTrigger id="income-category">
                  <SelectValue placeholder={t("transactions.selectCategory")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={NO_CATEGORY_VALUE}>
                    {t("transactions.noCategory")}
                  </SelectItem>
                  {incomeCategories.map((category) => (
                    <SelectItem key={category.id} value={category.id}>
                      {category.icon ? `${category.icon} ` : ""}
                      {category.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {mode === "transfer" && (
            <div className="space-y-2">
              <Label htmlFor="transfer-source">
                {t("quickAdd.sourceAccount")}
              </Label>
              <Select
                value={sourceAccountId}
                onValueChange={setSourceAccountId}
              >
                <SelectTrigger id="transfer-source">
                  <SelectValue placeholder={t("transactions.selectAccount")} />
                </SelectTrigger>
                <SelectContent>
                  {assetAccounts.map((account) => (
                    <SelectItem key={account.id} value={account.id}>
                      {account.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}

          {mode === "transfer" && (
            <div className="space-y-2">
              <Label htmlFor="transfer-destination">
                {t("quickAdd.destinationAccount")}
              </Label>
              <Select
                value={destinationAccountId}
                onValueChange={setDestinationAccountId}
              >
                <SelectTrigger id="transfer-destination">
                  <SelectValue placeholder={t("transactions.selectAccount")} />
                </SelectTrigger>
                <SelectContent>
                  {assetAccounts
                    .filter((a) => a.id !== sourceAccountId)
                    .map((account) => (
                      <SelectItem key={account.id} value={account.id}>
                        {account.name}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              {sourceAccountId === destinationAccountId &&
                destinationAccountId.length > 0 && (
                  <p className="text-xs text-destructive">
                    {t("quickAdd.sameAccountError")}
                  </p>
                )}
            </div>
          )}

          <div className="space-y-2">
            <Label htmlFor="quick-add-description">
              {t("transactions.description")}
              <span className="text-muted-foreground text-xs ml-1">
                ({t("common.optional")})
              </span>
            </Label>
            <Input
              id="quick-add-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={t("transactions.descriptionPlaceholder")}
            />
          </div>

          <div className="space-y-2">
            <Label>{t("transactions.date")}</Label>
            <DatePicker
              value={date}
              onChange={setDate}
              placeholder={t("transactions.date")}
            />
          </div>

          {submitError && (
            <p className="text-sm text-destructive">{submitError}</p>
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
            {mutation.isPending ? t("common.processing") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
