import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { formatAmount } from "@/utils/format";
import { translateErrorMessage } from "@/utils/errorTranslation";
import {
  reconcileAutoMatch,
  reconcileCreate,
  reconcileList,
  reconcilePreview,
  reconcileTotals,
  reconcileUndo,
} from "@/utils/api";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { DatePicker } from "@/components/ui/date-picker";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface ReconcileDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  accountId: string;
  accountName: string;
  onSuccess: () => void;
}

/**
 * Statement reconciliation for one balance-sheet account.
 *
 * The unit is a posting, not a transaction: a transfer out of this account may
 * already be ticked on the other side's statement. Totals come from the backend
 * on every toggle — this component never adds money itself.
 */
export function ReconcileDialog({
  open,
  onOpenChange,
  accountId,
  accountName,
  onSuccess,
}: ReconcileDialogProps) {
  const { t } = useTranslation();
  const [statementDate, setStatementDate] = useState("");
  const [closingBalance, setClosingBalance] = useState("");
  const [statementLines, setStatementLines] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [matching, setMatching] = useState(false);

  useEffect(() => {
    if (open) {
      const today = new Date();
      const iso = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
      setStatementDate(iso);
      setClosingBalance("");
      setStatementLines("");
      setSelected([]);
    }
  }, [open]);

  // The candidate list is scoped to the statement date and the backend rejects
  // a posting dated after it, so a selection carried across dates would fail to
  // submit for a reason the user cannot see.
  useEffect(() => {
    setSelected([]);
  }, [statementDate]);

  const previewQuery = useQuery({
    queryKey: ["reconcile-preview", accountId, statementDate],
    queryFn: () => reconcilePreview(accountId, statementDate),
    enabled: open && statementDate.length === 10,
  });

  const historyQuery = useQuery({
    queryKey: ["reconcile-history", accountId],
    queryFn: () => reconcileList(accountId),
    enabled: open,
  });

  const selectedKey = useMemo(() => [...selected].sort().join(","), [selected]);

  const totalsQuery = useQuery({
    queryKey: [
      "reconcile-totals",
      accountId,
      statementDate,
      closingBalance,
      selectedKey,
    ],
    queryFn: () =>
      reconcileTotals(accountId, statementDate, closingBalance, selected),
    enabled: open && statementDate.length === 10,
  });

  const candidates = previewQuery.data?.candidates ?? [];
  const totals = totalsQuery.data;

  const toggle = (postingId: string) => {
    setSelected((prev) =>
      prev.includes(postingId)
        ? prev.filter((id) => id !== postingId)
        : [...prev, postingId],
    );
  };

  const handleAutoMatch = async () => {
    const amounts = statementLines
      .split(/[\s,;]+/)
      .map((line) => line.trim())
      .filter(Boolean);
    if (amounts.length === 0) return;

    setMatching(true);
    try {
      const result = await reconcileAutoMatch(
        accountId,
        statementDate,
        amounts,
      );
      setSelected(result.matched);
      if (result.unmatchedTargets > 0) {
        toast.warning(
          t("reconcile.unmatchedTargets", { count: result.unmatchedTargets }),
        );
      }
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setMatching(false);
    }
  };

  const handleSubmit = async () => {
    setSaving(true);
    try {
      await reconcileCreate(accountId, statementDate, closingBalance, selected);
      toast.success(t("reconcile.created"));
      onSuccess();
      onOpenChange(false);
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setSaving(false);
    }
  };

  const handleUndo = async (id: string) => {
    try {
      await reconcileUndo(id);
      toast.success(t("reconcile.undone"));
      await Promise.all([historyQuery.refetch(), previewQuery.refetch()]);
      onSuccess();
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>
            {t("reconcile.title", { name: accountName })}
          </DialogTitle>
          <DialogDescription>{t("reconcile.description")}</DialogDescription>
        </DialogHeader>

        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-2">
            <Label>{t("reconcile.statementDate")}</Label>
            <DatePicker
              value={statementDate}
              onChange={setStatementDate}
              maxDate={new Date()}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="reconcile-closing">
              {t("reconcile.closingBalance")}
            </Label>
            <Input
              id="reconcile-closing"
              inputMode="decimal"
              placeholder="0.00"
              value={closingBalance}
              onChange={(e) => setClosingBalance(e.target.value)}
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor="reconcile-lines">
            {t("reconcile.autoMatchLabel")}
          </Label>
          <div className="flex gap-2">
            <Textarea
              id="reconcile-lines"
              rows={2}
              placeholder={t("reconcile.autoMatchPlaceholder")}
              value={statementLines}
              onChange={(e) => setStatementLines(e.target.value)}
              className="font-mono text-xs"
            />
            <Button
              type="button"
              variant="secondary"
              onClick={handleAutoMatch}
              disabled={matching || statementLines.trim().length === 0}
            >
              {t("reconcile.autoMatch")}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">
            {t("reconcile.autoMatchHint")}
          </p>
        </div>

        <div className="max-h-64 overflow-auto rounded-md border">
          {previewQuery.isLoading ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {t("common.loading")}
            </div>
          ) : candidates.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {t("reconcile.noCandidates")}
            </div>
          ) : (
            <ul className="divide-y">
              {candidates.map((candidate) => (
                <li
                  key={candidate.postingId}
                  className="flex items-center gap-3 px-3 py-2"
                >
                  <Checkbox
                    checked={selected.includes(candidate.postingId)}
                    onCheckedChange={() => toggle(candidate.postingId)}
                  />
                  <span className="w-24 shrink-0 text-xs text-muted-foreground tabular-nums">
                    {candidate.date}
                  </span>
                  <span className="min-w-0 flex-1 truncate text-sm">
                    {candidate.description || t("transactions.noDescription")}
                  </span>
                  <span className="text-sm tabular-nums">
                    {formatAmount(candidate.amount)}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="space-y-1 rounded-md bg-muted/50 p-3 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">
              {t("reconcile.openingBalance")}
            </span>
            <span className="tabular-nums">
              {formatAmount(previewQuery.data?.openingBalance ?? 0)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">
              {t("reconcile.selectedTotal")}
            </span>
            <span className="tabular-nums">
              {formatAmount(totals?.selectedTotal ?? 0)}
            </span>
          </div>
          <div className="flex justify-between font-medium">
            <span>{t("reconcile.expectedBalance")}</span>
            <span className="tabular-nums">
              {formatAmount(totals?.expectedBalance ?? 0)}
            </span>
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">
              {t("reconcile.difference")}
            </span>
            <span
              className={`tabular-nums ${
                totals?.balanced ? "text-income" : "text-destructive"
              }`}
            >
              {formatAmount(totals?.difference ?? 0)}
            </span>
          </div>
        </div>

        {(historyQuery.data?.length ?? 0) > 0 && (
          <div className="space-y-1">
            <Label>{t("reconcile.history")}</Label>
            <ul className="max-h-32 divide-y overflow-auto rounded-md border text-sm">
              {historyQuery.data?.map((record) => (
                <li
                  key={record.id}
                  className="flex items-center gap-3 px-3 py-2"
                >
                  <span className="w-24 shrink-0 text-xs text-muted-foreground tabular-nums">
                    {record.statementDate}
                  </span>
                  <span className="flex-1 tabular-nums">
                    {formatAmount(record.closingBalance)}
                  </span>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => handleUndo(record.id)}
                  >
                    {t("reconcile.undo")}
                  </Button>
                </li>
              ))}
            </ul>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={
              saving ||
              selected.length === 0 ||
              !totals?.balanced ||
              closingBalance.trim().length === 0
            }
          >
            {t("reconcile.submit")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
