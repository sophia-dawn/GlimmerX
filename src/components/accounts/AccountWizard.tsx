import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import {
  ChevronRight,
  Wallet,
  CreditCard,
  TrendingUp,
  TrendingDown,
  Scale,
} from "lucide-react";
import type { AccountType } from "@/types";
import { accountCreate } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { todayLocalDate } from "@/utils/date";
import { CURRENCY_SYMBOLS } from "@/utils/format";
import { AccountFields, type AccountFieldValues } from "./AccountFields";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AccountWizardProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess: () => void;
  /** If provided, the wizard skips step 1 and pre-fills the type. */
  initialType?: AccountType | null;
}

interface WizardState {
  step: 1 | 2 | 3;
  accountType: AccountType | null;
  initialBalance: string;
  initialBalanceDate: string;
  currency: string;
  formValues: AccountFieldValues;
}

// Account type metadata (icons only — labels come from i18n)
const ACCOUNT_TYPE_ICONS: Record<AccountType, typeof Wallet> = {
  asset: Wallet,
  liability: CreditCard,
  income: TrendingUp,
  expense: TrendingDown,
  equity: Scale,
};

const ACCOUNT_TYPE_KEYS = Object.keys(ACCOUNT_TYPE_ICONS) as AccountType[];

// Equity is system-managed — users cannot create equity accounts.
const WIZARD_ACCOUNT_TYPE_KEYS = ACCOUNT_TYPE_KEYS.filter(
  (t) => t !== "equity",
);

// English type labels for DB account paths (data layer, not display)

const COMMON_CURRENCIES = [
  "CNY",
  "USD",
  "EUR",
  "GBP",
  "JPY",
  "HKD",
  "KRW",
  "TWD",
] as const;

const emptyFormValues: AccountFieldValues = {
  name: "",
  description: "",
  account_number: "",
  iban: "",
  is_active: true,
  include_net_worth: true,
  meta: {},
};

const initialState: WizardState = {
  step: 1,
  accountType: null,
  initialBalance: "0",
  initialBalanceDate: "",
  currency: "CNY",
  formValues: { ...emptyFormValues },
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AccountWizard({
  open,
  onOpenChange,
  onSuccess,
  initialType,
}: AccountWizardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [state, setState] = useState<WizardState>(initialState);
  const [loading, setLoading] = useState(false);

  // Load parent options when opening
  useEffect(() => {
    if (open) {
      setState({
        step: initialType != null ? 2 : 1,
        accountType: initialType ?? null,
        initialBalance: "0",
        initialBalanceDate: todayLocalDate(),
        currency: "CNY",
        formValues: { ...emptyFormValues },
      });
    }
  }, [open, initialType]);

  const handleNext = () => {
    if (state.step === 1) {
      if (!state.accountType) return;
      setState((prev) => ({ ...prev, step: 2 }));
    } else if (state.step === 2) {
      if (!state.formValues.name.trim()) {
        toast.error(t("accounts.wizard.enterAccountName"));
        return;
      }
      setState((prev) => ({ ...prev, step: 3 }));
    }
  };

  const handleBack = () => {
    if (state.step > 1) {
      setState((prev) => ({ ...prev, step: (prev.step - 1) as 1 | 2 | 3 }));
    }
  };

  const handleAddAndContinue = async () => {
    await createAccount(false);
  };

  const handleFinish = async () => {
    await createAccount(true);
  };

  const createAccount = async (isFinish: boolean) => {
    if (!state.accountType || !state.formValues.name.trim()) {
      toast.error(t("accounts.wizard.enterAccountName"));
      return;
    }

    setLoading(true);
    try {
      // Prepend account type to name for flat model path format
      const accountPath = `${state.accountType}/${state.formValues.name.trim()}`;

      // Send raw decimal string — backend handles conversion
      const initialBalanceDecimal =
        state.accountType === "asset" || state.accountType === "liability"
          ? state.initialBalance || undefined
          : undefined;

      // Build meta from form meta values (non-empty strings only)
      const metaPayload: Record<string, string> = {};
      for (const [key, value] of Object.entries(state.formValues.meta)) {
        if (value !== "") {
          metaPayload[key] = value;
        }
      }
      const hasMeta = Object.keys(metaPayload).length > 0;

      await accountCreate({
        name: accountPath,
        currency: state.currency,
        initial_balance: initialBalanceDecimal,
        initial_balance_date: state.initialBalanceDate || undefined,
        description: state.formValues.description.trim() || undefined,
        account_number: state.formValues.account_number.trim() || undefined,
        equity_account_name: "Equity",
        opening_balance_name: "Opening Balances",
        iban: state.formValues.iban.trim() || undefined,
        is_active: state.formValues.is_active,
        include_net_worth: state.formValues.include_net_worth,
        meta: hasMeta ? metaPayload : undefined,
      });

      toast.success(t("accounts.accountCreated"));

      // Reset form for next account
      setState({
        ...initialState,
        step: initialType != null ? 2 : 1,
        accountType: initialType ?? null,
        formValues: { ...emptyFormValues },
        initialBalanceDate: todayLocalDate(),
      });

      onSuccess();
      queryClient.invalidateQueries();
      if (isFinish) {
        onOpenChange(false);
      }
    } catch (err: unknown) {
      console.error("[AccountWizard] createAccount error:", err);
      toast.error(translateErrorMessage(err, t));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="sm:max-w-[40%] sm:min-w-[360px] max-h-[90vh] overflow-y-auto"
        onInteractOutside={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>{t("accounts.wizard.title")}</DialogTitle>
          <DialogDescription>
            {initialType
              ? t("accounts.wizard.descWithType", {
                  type: t(`accounts.types.${initialType}`),
                })
              : t("accounts.wizard.desc")}
          </DialogDescription>
        </DialogHeader>

        {/* Step indicator */}
        {state.step > 1 && (
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <span className={state.step >= 1 ? "" : "opacity-50"}>
              {t("accounts.wizard.stepType")}
            </span>
            <ChevronRight className="h-3 w-3" />
            <span className={state.step >= 2 ? "" : "opacity-50"}>
              {t("accounts.wizard.stepInfo")}
            </span>
            <ChevronRight className="h-3 w-3" />
            <span className={state.step >= 3 ? "" : "opacity-50"}>
              {t("accounts.wizard.stepConfirm")}
            </span>
          </div>
        )}

        {/* ── Step 1: Choose account type ───────────── */}
        {state.step === 1 && (
          <div className="space-y-3 py-2">
            <p className="text-sm text-muted-foreground">
              {t("accounts.wizard.selectTypeDesc")}
            </p>
            <div className="grid grid-cols-2 gap-2">
              {WIZARD_ACCOUNT_TYPE_KEYS.map((key) => {
                const Icon = ACCOUNT_TYPE_ICONS[key];
                const isSelected = state.accountType === key;
                return (
                  <button
                    key={key}
                    type="button"
                    onClick={() =>
                      setState((prev) => ({
                        ...prev,
                        accountType: key,
                      }))
                    }
                    className={`flex flex-col items-start gap-1 rounded-lg border p-3 text-left transition-colors ${
                      isSelected
                        ? "border-primary bg-primary/5"
                        : "hover:border-muted-foreground/50"
                    }`}
                  >
                    <div className="flex items-center gap-2">
                      <Icon className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-medium">
                        {t(`accounts.types.${key}`)}
                      </span>
                    </div>
                    <span className="text-xs text-muted-foreground">
                      {t(`accounts.typeDescriptions.${key}`)}
                    </span>
                  </button>
                );
              })}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => onOpenChange(false)}
              >
                {t("common.cancel")}
              </Button>
              <Button onClick={handleNext} disabled={!state.accountType}>
                {t("common.next")}
              </Button>
            </DialogFooter>
          </div>
        )}

        {/* ── Step 2: Enter account info ────────────── */}
        {state.step === 2 && state.accountType && (
          <div className="space-y-4 py-2">
            {/* Shared fields */}
            <AccountFields
              mode="create"
              accountType={state.accountType}
              values={state.formValues}
              onChange={(formValues) =>
                setState((prev) => ({ ...prev, formValues }))
              }
            />

            {/* Initial balance for asset/liability */}
            {(state.accountType === "asset" ||
              state.accountType === "liability") && (
              <div className="space-y-2">
                <Label htmlFor="acc-balance">
                  {t("accounts.wizard.initialBalance")}
                </Label>
                <Input
                  id="acc-balance"
                  type="number"
                  step="0.01"
                  placeholder={t("accounts.wizard.balancePlaceholder")}
                  value={state.initialBalance}
                  onChange={(e) =>
                    setState((prev) => ({
                      ...prev,
                      initialBalance: e.target.value,
                    }))
                  }
                />
                <p className="text-xs text-muted-foreground">
                  {state.accountType === "liability"
                    ? t("accounts.wizard.liabilityBalanceHelp")
                    : t("accounts.fieldHelp.initialBalance")}
                </p>
              </div>
            )}

            {/* Date for opening balance */}
            {(state.accountType === "asset" ||
              state.accountType === "liability") && (
              <div className="space-y-2">
                <Label htmlFor="acc-date">
                  {t("accounts.wizard.initialBalanceDate")}
                </Label>
                <DatePicker
                  value={state.initialBalanceDate}
                  onChange={(date) =>
                    setState((prev) => ({
                      ...prev,
                      initialBalanceDate: date,
                    }))
                  }
                  placeholder={t("accounts.wizard.initialBalanceDate")}
                />
              </div>
            )}

            {/* Currency selector — create mode only */}
            <div className="space-y-2">
              <Label>{t("accounts.wizard.currency")}</Label>
              <Select
                value={state.currency}
                onValueChange={(val) =>
                  setState((prev) => ({ ...prev, currency: val }))
                }
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {COMMON_CURRENCIES.map((c) => (
                    <SelectItem key={c} value={c}>
                      {c}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={handleBack}>
                {t("common.previous")}
              </Button>
              <Button
                onClick={handleNext}
                disabled={!state.formValues.name.trim()}
              >
                {t("common.next")}
              </Button>
            </DialogFooter>
          </div>
        )}

        {/* ── Step 3: Confirm ───────────────────────── */}
        {state.step === 3 && state.accountType && (
          <div className="space-y-4 py-2">
            <div className="rounded-lg border bg-muted/30 p-4 space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("accounts.wizard.accountTypeLabel")}
                </span>
                <span className="font-medium">
                  {t(`accounts.types.${state.accountType}`)}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("accounts.wizard.nameLabel")}
                </span>
                <span className="font-medium">
                  {state.formValues.name.trim()}
                </span>
              </div>
              {state.formValues.description && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.description")}
                  </span>
                  <span className="font-medium max-w-[200px] truncate">
                    {state.formValues.description}
                  </span>
                </div>
              )}
              {state.formValues.account_number && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.accountNumber")}
                  </span>
                  <span className="font-medium">
                    {state.formValues.account_number}
                  </span>
                </div>
              )}
              {state.formValues.meta["account_role"] && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.subtype")}
                  </span>
                  <span className="font-medium">
                    {t(
                      `accounts.subtypes.${state.formValues.meta["account_role"]}`,
                    )}
                  </span>
                </div>
              )}
              {state.formValues.meta["liability_type"] && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.subtype")}
                  </span>
                  <span className="font-medium">
                    {t(
                      `accounts.subtypes.${state.formValues.meta["liability_type"]}`,
                    )}
                  </span>
                </div>
              )}
              {(state.accountType === "asset" ||
                state.accountType === "liability") && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.wizard.initialBalance")}
                  </span>
                  <span className="font-medium">
                    {CURRENCY_SYMBOLS[state.currency] ?? state.currency}
                    {parseFloat(state.initialBalance) || 0}
                  </span>
                </div>
              )}
              {(state.accountType === "asset" ||
                state.accountType === "liability") && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">
                    {t("accounts.wizard.initialBalanceDate")}
                  </span>
                  <span className="font-medium">
                    {state.initialBalanceDate}
                  </span>
                </div>
              )}
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("accounts.wizard.currency")}
                </span>
                <span className="font-medium">{state.currency}</span>
              </div>
            </div>

            <DialogFooter>
              <Button type="button" variant="outline" onClick={handleBack}>
                {t("common.previous")}
              </Button>
              <Button
                type="button"
                variant="secondary"
                onClick={handleAddAndContinue}
                disabled={loading}
              >
                {t("accounts.wizard.addAndContinue")}
              </Button>
              <Button onClick={handleFinish} disabled={loading}>
                {loading ? t("accounts.wizard.creating") : t("common.add")}
              </Button>
            </DialogFooter>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
