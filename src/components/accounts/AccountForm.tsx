import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { useQueryClient } from "@tanstack/react-query";
import type { AccountDto } from "@/types";
import { accountUpdate } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { AccountFields, type AccountFieldValues } from "./AccountFields";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { todayLocalDate } from "@/utils/date";
import { liabilityStorageToDisplay } from "@/utils/liabilityBalance";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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

interface AccountFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  editNode?: AccountDto | null;
  onSuccess: () => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AccountForm({
  open,
  onOpenChange,
  editNode,
  onSuccess,
}: AccountFormProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [loading, setLoading] = useState(false);
  const isEditing = !!editNode;

  const [formValues, setFormValues] = useState<AccountFieldValues>({
    name: "",
    description: "",
    account_number: "",
    iban: "",
    is_active: true,
    include_net_worth: true,
    meta: {},
  });

  // Initial balance for edit mode (stored as a decimal string)
  const [initialBalance, setInitialBalance] = useState("");
  const [initialBalanceDate, setInitialBalanceDate] = useState("");

  const accountType = editNode?.account_type ?? "";
  const canHaveBalance = accountType === "asset" || accountType === "liability";

  // Pre-fill form when editing
  useEffect(() => {
    if (editNode) {
      const meta: Record<string, string> = {};
      for (const m of editNode.meta ?? []) {
        meta[m.key] = m.value;
      }

      setFormValues({
        name: editNode.name,
        description: editNode.description ?? "",
        account_number: editNode.account_number ?? "",
        iban: editNode.iban ?? "",
        is_active: editNode.is_active ?? true,
        include_net_worth: editNode.include_net_worth ?? true,
        meta,
      });
      if (canHaveBalance) {
        const balanceCents = editNode.initial_balance ?? 0;
        const displayBalance = liabilityStorageToDisplay(
          balanceCents,
          accountType,
        );
        const balanceDecimal =
          displayBalance !== 0 ? (displayBalance / 100).toFixed(2) : "0";
        setInitialBalance(balanceDecimal);
        setInitialBalanceDate(
          editNode.initial_balance_date ?? todayLocalDate(),
        );
      }
    }
  }, [editNode, canHaveBalance, accountType]);

  const onSubmit = async () => {
    const name = formValues.name.trim();
    if (!name) {
      toast.error(t("accounts.nameRequired"));
      return;
    }

    setLoading(true);
    try {
      if (isEditing && editNode) {
        // Build meta from form meta values (non-empty strings only)
        const metaPayload: Record<string, string> = {};
        for (const [key, value] of Object.entries(formValues.meta)) {
          if (value !== "") {
            metaPayload[key] = value;
          }
        }
        const hasMeta = Object.keys(metaPayload).length > 0;

        const hasInitialBalance =
          canHaveBalance &&
          initialBalance.trim() !== "" &&
          initialBalance.trim() !== "0";

        const payload = {
          name: formValues.name.trim() || undefined,
          description: formValues.description.trim() || undefined,
          account_number: formValues.account_number.trim() || undefined,
          initial_balance: hasInitialBalance
            ? initialBalance.trim()
            : undefined,
          initial_balance_date: hasInitialBalance
            ? initialBalanceDate.trim() || undefined
            : undefined,
          iban: formValues.iban.trim() || undefined,
          is_active: formValues.is_active,
          include_net_worth: formValues.include_net_worth,
          meta: hasMeta ? metaPayload : undefined,
        };

        await accountUpdate(editNode.id, payload);
        toast.success(t("accounts.accountUpdated"));
      }
      onSuccess();
      queryClient.invalidateQueries();
      onOpenChange(false);
    } catch (err: unknown) {
      console.error("[AccountForm] update failed:", err);
      toast.error(translateErrorMessage(err, t));
    } finally {
      setLoading(false);
    }
  };

  if (!isEditing) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[40%] sm:min-w-[360px] max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("accounts.editAccount")}</DialogTitle>
          <DialogDescription>{t("accounts.editAccountDesc")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <AccountFields
            mode="edit"
            accountType={editNode.account_type}
            values={formValues}
            onChange={setFormValues}
            existingAccount={editNode}
          />

          {/* Initial balance for asset/liability — edit mode */}
          {canHaveBalance && (
            <>
              <div className="space-y-2">
                <Label htmlFor="edit-initial-balance">
                  {t("accounts.wizard.initialBalance")}
                </Label>
                <Input
                  id="edit-initial-balance"
                  type="number"
                  step="0.01"
                  placeholder={t("accounts.wizard.balancePlaceholder")}
                  value={initialBalance}
                  onChange={(e) => setInitialBalance(e.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="edit-initial-date">
                  {t("accounts.wizard.initialBalanceDate")}
                </Label>
                <DatePicker
                  value={initialBalanceDate}
                  onChange={setInitialBalanceDate}
                  placeholder={t("accounts.wizard.initialBalanceDate")}
                />
              </div>
            </>
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
          <Button onClick={onSubmit} disabled={loading}>
            {loading ? t("common.processing") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
