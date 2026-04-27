import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import type { AccountDto, AccountMetaSchema, AccountType } from "@/types";
import { accountMetaSchema } from "@/utils/api";
import { DatePicker } from "@/components/ui/date-picker";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface AccountFieldValues {
  name: string;
  description: string;
  account_number: string;
  iban: string;
  is_active: boolean;
  include_net_worth: boolean;
  meta: Record<string, string>;
}

export interface AccountFieldsProps {
  /** Whether this is create or edit mode */
  mode: "create" | "edit";
  /** The account type (determines subtype options) */
  accountType: AccountType;
  /** Current field values — controlled by parent */
  values: AccountFieldValues;
  /** Called when any field changes */
  onChange: (values: AccountFieldValues) => void;
  /** Only in edit mode: the existing account (for read-only currency display) */
  existingAccount?: AccountDto | null;
  /** Validation errors keyed by field name */
  errors?: Record<string, string | undefined>;
}

function getSubtypeMetaKey(type: AccountType): string {
  if (type === "asset") return "account_role";
  if (type === "liability") return "liability_type";
  return "";
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AccountFields({
  mode,
  accountType,
  values,
  onChange,
  existingAccount,
  errors,
}: AccountFieldsProps) {
  const { t } = useTranslation();
  const { data: schema } = useQuery<AccountMetaSchema>({
    queryKey: ["account-meta-schema"],
    queryFn: accountMetaSchema,
    staleTime: Infinity,
  });
  const subtypeOptions = schema
    ? accountType === "asset"
      ? schema.valid_account_roles
      : accountType === "liability"
        ? schema.valid_liability_types
        : []
    : [];
  const subtypeMetaKey = getSubtypeMetaKey(accountType);

  const set = (partial: Partial<AccountFieldValues>) => {
    onChange({ ...values, ...partial });
  };

  // For liability accounts, also check legacy "account_role" key for backwards compatibility
  const subtypeValue =
    accountType === "liability"
      ? (values.meta["liability_type"] ?? values.meta["account_role"] ?? "")
      : (values.meta[subtypeMetaKey] ?? "");

  const showCreditCardFields =
    accountType === "asset" && subtypeValue === "ccAsset";

  const showLiabilityFields = accountType === "liability";

  return (
    <div className="space-y-4">
      {/* Account Name */}
      <div className="space-y-2">
        <Label htmlFor="acc-fields-name">{t("accounts.accountName")}</Label>
        <Input
          id="acc-fields-name"
          placeholder={t("accounts.wizard.namePlaceholder")}
          value={values.name}
          onChange={(e) => set({ name: e.target.value })}
          autoFocus={mode === "create"}
        />
        {errors?.name && (
          <p className="text-sm text-destructive">{errors.name}</p>
        )}
        {!errors?.name && mode === "create" && (
          <p className="text-xs text-muted-foreground whitespace-pre-wrap">
            {t(`accounts.fieldHelp.accountNameByType.${accountType}`)}
          </p>
        )}
      </div>

      {/* Description */}
      <div className="space-y-2">
        <Label htmlFor="acc-fields-desc">{t("accounts.description")}</Label>
        <textarea
          id="acc-fields-desc"
          rows={2}
          className="flex w-full rounded-md border border-input bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          placeholder={t("accounts.descriptionPlaceholder")}
          value={values.description}
          onChange={(e) => set({ description: e.target.value })}
        />
        <p className="text-xs text-muted-foreground">
          {t("accounts.fieldHelp.description")}
        </p>
      </div>

      {/* Account Number */}
      <div className="space-y-2">
        <Label htmlFor="acc-fields-number">{t("accounts.accountNumber")}</Label>
        <Input
          id="acc-fields-number"
          placeholder={t("accounts.accountNumberPlaceholder")}
          value={values.account_number}
          onChange={(e) => set({ account_number: e.target.value })}
        />
        <p className="text-xs text-muted-foreground">
          {t("accounts.fieldHelp.accountNumber")}
        </p>
      </div>

      {/* Subtype */}
      {subtypeOptions.length > 0 && (
        <div className="space-y-2">
          <Label>{t("accounts.subtype")}</Label>
          <Select
            value={subtypeValue || "__none"}
            onValueChange={(val) =>
              set({
                meta: {
                  ...values.meta,
                  [subtypeMetaKey]: val === "__none" ? "" : val,
                },
              })
            }
          >
            <SelectTrigger>
              <SelectValue placeholder="—" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none">—</SelectItem>
              {subtypeOptions.map((st) => (
                <SelectItem key={st} value={st}>
                  {t(`accounts.subtypes.${st}`)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <p className="text-xs text-muted-foreground">
            {t("accounts.fieldHelp.subtype")}
          </p>
          {subtypeValue && (
            <p className="text-xs text-primary/80">
              {t(`accounts.subtypeDescriptions.${subtypeValue}`)}
            </p>
          )}
        </div>
      )}

      {/* IBAN */}
      <div className="space-y-2">
        <Label htmlFor="acc-fields-iban">{t("accounts.iban")}</Label>
        <Input
          id="acc-fields-iban"
          placeholder={t("accounts.ibanPlaceholder")}
          value={values.iban}
          onChange={(e) => set({ iban: e.target.value })}
        />
        <p className="text-xs text-muted-foreground">
          {t("accounts.fieldHelp.iban")}
        </p>
      </div>

      {/* Credit Card specific fields */}
      {showCreditCardFields && (
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="acc-fields-cc-type">
              {t("accounts.creditCardType")}
            </Label>
            <Select
              value={values.meta["credit_card_type"] || "__none"}
              onValueChange={(val) =>
                set({
                  meta: {
                    ...values.meta,
                    credit_card_type: val === "__none" ? "" : val,
                  },
                })
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="—" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__none">—</SelectItem>
                <SelectItem value="monthlyFull">
                  {t("accounts.creditCardTypes.monthlyFull")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="acc-fields-cc-date">
              {t("accounts.monthlyPaymentDate")}
            </Label>
            <DatePicker
              value={values.meta["monthly_payment_date"]}
              onChange={(date) =>
                set({
                  meta: {
                    ...values.meta,
                    monthly_payment_date: date,
                  },
                })
              }
              placeholder={t("accounts.monthlyPaymentDate")}
            />
          </div>
        </div>
      )}

      {/* Liability specific fields */}
      {showLiabilityFields && (
        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="acc-fields-interest">
              {t("accounts.interest")}
            </Label>
            <Input
              id="acc-fields-interest"
              placeholder={t("accounts.interestPlaceholder")}
              value={values.meta["interest"]}
              onChange={(e) =>
                set({ meta: { ...values.meta, interest: e.target.value } })
              }
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="acc-fields-interest-period">
              {t("accounts.interestPeriod")}
            </Label>
            <Select
              value={values.meta["interest_period"] || "__none"}
              onValueChange={(val) =>
                set({
                  meta: {
                    ...values.meta,
                    interest_period: val === "__none" ? "" : val,
                  },
                })
              }
            >
              <SelectTrigger>
                <SelectValue placeholder="—" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__none">—</SelectItem>
                <SelectItem value="monthly">
                  {t("accounts.interestPeriods.monthly")}
                </SelectItem>
                <SelectItem value="yearly">
                  {t("accounts.interestPeriods.yearly")}
                </SelectItem>
              </SelectContent>
            </Select>
          </div>
        </div>
      )}

      {/* Currency: editable in create mode, read-only in edit mode */}
      {mode === "edit" && existingAccount ? (
        <div className="space-y-2">
          <Label>{t("accounts.currency")}</Label>
          <p className="text-sm text-muted-foreground">
            {existingAccount.currency}
          </p>
        </div>
      ) : null}

      <div className="flex items-center space-x-2">
        <Checkbox
          id="acc-fields-active"
          checked={values.is_active}
          onCheckedChange={(checked) => set({ is_active: checked === true })}
        />
        <div className="space-y-0.5">
          <Label htmlFor="acc-fields-active" className="text-sm font-normal">
            {t("accounts.isActive")}
          </Label>
          <p className="text-xs text-muted-foreground">
            {t("accounts.fieldHelp.isActive")}
          </p>
        </div>
      </div>

      {(accountType === "asset" || accountType === "liability") && (
        <div className="flex items-center space-x-2">
          <Checkbox
            id="acc-fields-net-worth"
            checked={values.include_net_worth}
            onCheckedChange={(checked) =>
              set({ include_net_worth: checked === true })
            }
          />
          <div className="space-y-0.5">
            <Label
              htmlFor="acc-fields-net-worth"
              className="text-sm font-normal"
            >
              {t("accounts.includeNetWorth")}
            </Label>
            <p className="text-xs text-muted-foreground">
              {t("accounts.fieldHelp.includeNetWorth")}
            </p>
          </div>
        </div>
      )}
    </div>
  );
}
