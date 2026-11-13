import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { Plus, Target } from "lucide-react";
import type { SavingsGoal } from "@/types";
import {
  accountList,
  accountTransfer,
  savingsCreate,
  savingsDelete,
  savingsList,
  savingsSetStatus,
  savingsUpdate,
} from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { centsToYuan, formatAmount, yuanToCents } from "@/utils/format";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { useHeaderActions } from "@/contexts/HeaderContext";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { DatePicker } from "@/components/ui/date-picker";
import {
  Dialog,
  DialogContent,
  DialogDescription,
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

/**
 * Savings goals.
 *
 * A goal is a dedicated asset account with a target: "putting money aside" is
 * an ordinary transfer into it, so the progress bar is simply that account's
 * balance. All money values below come from the backend — the page only
 * renders them (and a width percentage, which is not money).
 */
export function SavingsPage() {
  const { t } = useTranslation();
  const { setActions } = useHeaderActions();
  const [formOpen, setFormOpen] = useState(false);
  const [editGoal, setEditGoal] = useState<SavingsGoal | null>(null);
  const [transferGoal, setTransferGoal] = useState<SavingsGoal | null>(null);
  const [transferDirection, setTransferDirection] = useState<
    "deposit" | "withdraw"
  >("deposit");

  const {
    data: goals = [],
    refetch,
    isLoading,
  } = useQuery({
    queryKey: ["savings-goals"],
    queryFn: savingsList,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const headerActions = useMemo(
    () => (
      <Button size="sm" onClick={() => setFormOpen(true)}>
        <Plus className="mr-1 h-4 w-4" />
        {t("savings.newGoal")}
      </Button>
    ),
    [t],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  const handleDelete = async (goal: SavingsGoal) => {
    try {
      await savingsDelete(goal.id);
      toast.success(t("savings.deleted"));
      refetch();
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    }
  };

  const handleStatus = async (
    goal: SavingsGoal,
    status: "active" | "archived",
  ) => {
    try {
      await savingsSetStatus(goal.id, status);
      refetch();
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    }
  };

  return (
    <div className="h-full overflow-auto p-6">
      {isLoading && (
        <div className="py-12 text-center text-muted-foreground">
          {t("common.loading")}
        </div>
      )}

      {!isLoading && goals.length === 0 && (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <Target className="mb-4 h-10 w-10 text-muted-foreground" />
          <h2 className="mb-2 text-lg font-medium">{t("savings.empty")}</h2>
          <p className="mb-6 max-w-md text-sm text-muted-foreground">
            {t("savings.emptyDesc")}
          </p>
          <Button onClick={() => setFormOpen(true)}>
            {t("savings.newGoal")}
          </Button>
        </div>
      )}

      {!isLoading && goals.length > 0 && (
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {goals.map((goal) => (
            <GoalCard
              key={goal.id}
              goal={goal}
              onEdit={() => {
                setEditGoal(goal);
                setFormOpen(true);
              }}
              onDeposit={() => {
                setTransferDirection("deposit");
                setTransferGoal(goal);
              }}
              onWithdraw={() => {
                setTransferDirection("withdraw");
                setTransferGoal(goal);
              }}
              onArchive={() =>
                handleStatus(
                  goal,
                  goal.status === "archived" ? "active" : "archived",
                )
              }
              onDelete={() => handleDelete(goal)}
            />
          ))}
        </div>
      )}

      <GoalFormDialog
        open={formOpen}
        onOpenChange={(open) => {
          setFormOpen(open);
          if (!open) setEditGoal(null);
        }}
        goal={editGoal}
        onSuccess={refetch}
      />

      {transferGoal && (
        <TransferDialog
          goal={transferGoal}
          direction={transferDirection}
          onClose={() => setTransferGoal(null)}
          onSuccess={refetch}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Card
// ---------------------------------------------------------------------------

interface GoalCardProps {
  goal: SavingsGoal;
  onEdit: () => void;
  onDeposit: () => void;
  onWithdraw: () => void;
  onArchive: () => void;
  onDelete: () => void;
}

function GoalCard({
  goal,
  onEdit,
  onDeposit,
  onWithdraw,
  onArchive,
  onDelete,
}: GoalCardProps) {
  const { t } = useTranslation();
  // Only used for the bar width; no money is computed here.
  const ratio =
    goal.targetAmount > 0
      ? Math.min(100, Math.round((goal.savedAmount / goal.targetAmount) * 100))
      : 0;

  return (
    <div className="rounded-lg border p-4">
      <div className="mb-3 flex items-start justify-between gap-2">
        <div className="min-w-0">
          <h3 className="truncate font-medium">{goal.name}</h3>
          {goal.status !== "active" && (
            <span className="text-xs text-muted-foreground">
              {t(
                goal.status === "achieved"
                  ? "savings.achieved"
                  : "savings.archived",
              )}
            </span>
          )}
        </div>
        <div className="flex shrink-0 gap-1">
          <Button variant="ghost" size="sm" onClick={onEdit}>
            {t("common.edit")}
          </Button>
          <Button variant="ghost" size="sm" onClick={onArchive}>
            {goal.status === "archived"
              ? t("savings.restore")
              : t("savings.archive")}
          </Button>
        </div>
      </div>

      <div className="mb-2 h-2 w-full overflow-hidden rounded-full bg-muted">
        <div
          className="h-full rounded-full bg-primary transition-all"
          style={{ width: `${ratio}%` }}
        />
      </div>

      <div className="mb-1 flex justify-between text-sm">
        <span className="tabular-nums">{formatAmount(goal.savedAmount)}</span>
        <span className="text-muted-foreground tabular-nums">
          {formatAmount(goal.targetAmount)}
        </span>
      </div>

      <div className="space-y-0.5 text-xs text-muted-foreground">
        <div>
          {t("savings.remaining")} · {formatAmount(goal.remaining)}
        </div>
        {goal.monthlyNeeded != null && (
          <div>
            {t("savings.monthlyNeeded", {
              amount: formatAmount(goal.monthlyNeeded),
            })}
          </div>
        )}
      </div>

      <div className="mt-3 flex gap-2">
        <Button size="sm" variant="secondary" onClick={onDeposit}>
          {t("savings.deposit")}
        </Button>
        <Button size="sm" variant="outline" onClick={onWithdraw}>
          {t("savings.withdraw")}
        </Button>
        <Button
          size="sm"
          variant="ghost"
          className="ml-auto text-destructive"
          onClick={onDelete}
        >
          {t("savings.delete")}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Create / edit
// ---------------------------------------------------------------------------

interface GoalFormDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  goal: SavingsGoal | null;
  onSuccess: () => void;
}

function GoalFormDialog({
  open,
  onOpenChange,
  goal,
  onSuccess,
}: GoalFormDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [target, setTarget] = useState("");
  const [targetDate, setTargetDate] = useState<string | undefined>(undefined);
  const [monthlyPlan, setMonthlyPlan] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (open) {
      setName(goal?.name ?? "");
      setTarget(goal ? String(centsToYuan(goal.targetAmount)) : "");
      setTargetDate(goal?.targetDate ?? undefined);
      setMonthlyPlan(
        goal?.monthlyPlan != null ? String(centsToYuan(goal.monthlyPlan)) : "",
      );
    }
  }, [open, goal]);

  const handleSubmit = async () => {
    setSaving(true);
    try {
      if (goal) {
        await savingsUpdate(goal.id, {
          name,
          targetAmount: target,
          targetDate: targetDate ?? null,
          monthlyPlan: monthlyPlan.trim() === "" ? null : monthlyPlan,
        });
        toast.success(t("savings.updated"));
      } else {
        await savingsCreate({
          name,
          targetAmount: target,
          targetDate: targetDate ?? null,
          monthlyPlan: monthlyPlan.trim() === "" ? null : monthlyPlan,
        });
        toast.success(t("savings.created"));
      }
      onSuccess();
      onOpenChange(false);
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>
            {goal ? t("savings.editTitle") : t("savings.newGoal")}
          </DialogTitle>
          <DialogDescription>{t("savings.emptyDesc")}</DialogDescription>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-2">
            <Label htmlFor="goal-name">{t("savings.name")}</Label>
            <Input
              id="goal-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="goal-target">{t("savings.targetAmount")}</Label>
            <Input
              id="goal-target"
              inputMode="decimal"
              placeholder="0.00"
              value={target}
              onChange={(e) => setTarget(e.target.value)}
            />
          </div>
          <div className="space-y-2">
            <Label>{t("savings.targetDate")}</Label>
            <DatePicker
              value={targetDate}
              onChange={setTargetDate}
              maxDate={new Date(2100, 11, 31)}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="goal-plan">{t("savings.monthlyPlan")}</Label>
            <Input
              id="goal-plan"
              inputMode="decimal"
              placeholder="0.00"
              value={monthlyPlan}
              onChange={(e) => setMonthlyPlan(e.target.value)}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={saving || name.trim() === "" || target.trim() === ""}
          >
            {t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// Deposit / withdraw
// ---------------------------------------------------------------------------

interface TransferDialogProps {
  goal: SavingsGoal;
  direction: "deposit" | "withdraw";
  onClose: () => void;
  onSuccess: () => void;
}

function TransferDialog({
  goal,
  direction,
  onClose,
  onSuccess,
}: TransferDialogProps) {
  const { t } = useTranslation();
  const [sourceId, setSourceId] = useState("");
  const [amount, setAmount] = useState("");
  const [saving, setSaving] = useState(false);

  const { data: accounts = [] } = useQuery({
    queryKey: ["accounts", "asset-for-transfer"],
    queryFn: () => accountList(["asset"]),
    ...QUERY_CONFIG.STATIC,
  });

  // The goal's own account must not appear in the picker.
  const options = accounts.filter((account) => account.id !== goal.accountId);

  const handleSubmit = async () => {
    if (!sourceId) return;
    setSaving(true);
    try {
      const cents = yuanToCents(amount);
      const description =
        direction === "deposit"
          ? t("savings.depositTitle", { name: goal.name })
          : t("savings.withdrawTitle", { name: goal.name });
      if (direction === "deposit") {
        await accountTransfer(sourceId, goal.accountId, cents, description);
      } else {
        await accountTransfer(goal.accountId, sourceId, cents, description);
      }
      toast.success(t("savings.transferDone"));
      onSuccess();
      onClose();
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>
            {t(
              direction === "deposit"
                ? "savings.depositTitle"
                : "savings.withdrawTitle",
              { name: goal.name },
            )}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-3">
          <div className="space-y-2">
            <Label>{t("savings.fromAccount")}</Label>
            <Select value={sourceId} onValueChange={setSourceId}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {options.map((account) => (
                  <SelectItem key={account.id} value={account.id}>
                    {account.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-2">
            <Label htmlFor="transfer-amount">{t("savings.amount")}</Label>
            <Input
              id="transfer-amount"
              inputMode="decimal"
              placeholder="0.00"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
            />
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={saving || !sourceId || amount.trim() === ""}
          >
            {t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
