import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { aiParseTransaction, accountCreate } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import type { AiProposal, QuickAddInitial } from "@/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
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
import { QuickAddDialog } from "@/components/transactions/QuickAddDialog";
import { ProposalReview } from "@/components/transactions/ProposalReview";
import { Sparkles, Loader2 } from "lucide-react";

interface AiInputDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

type Stage = "input" | "confirmTransaction" | "confirmAccount";

/** Map an AI proposal onto the QuickAddDialog review prefill. */
function toQuickAddInitial(proposal: AiProposal): QuickAddInitial {
  return {
    mode: proposal.action as QuickAddInitial["mode"],
    amount: proposal.amount ?? "",
    sourceAccountId: proposal.sourceAccount?.id,
    sourceAccountName: proposal.sourceAccount?.name,
    destinationAccountId: proposal.destinationAccount?.id,
    destinationAccountName: proposal.destinationAccount?.name,
    categoryName: proposal.categoryName,
    description: proposal.description,
    date: proposal.date,
    warnings: proposal.warnings,
  };
}

export function AiInputDialog({
  open,
  onOpenChange,
  onSuccess,
}: AiInputDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [submitError, setSubmitError] = useState<string | null>(null);
  const [stage, setStage] = useState<Stage>("input");
  const [proposal, setProposal] = useState<AiProposal | null>(null);
  // Editable fields for the create_account confirmation card.
  const [accountPath, setAccountPath] = useState("");
  const [accountKind, setAccountKind] = useState<"asset" | "liability">(
    "asset",
  );
  const [openingBalance, setOpeningBalance] = useState("");

  useEffect(() => {
    if (!open) {
      setText("");
      setSubmitError(null);
      setStage("input");
      setProposal(null);
      setAccountPath("");
      setAccountKind("asset");
      setOpeningBalance("");
    }
  }, [open]);

  const parseMutation = useMutation({
    mutationFn: (input: string) => aiParseTransaction(input),
    onSuccess: (result) => {
      setSubmitError(null);
      if (result.action === "create_account") {
        setProposal(result);
        setAccountPath(result.accountPath ?? "");
        setAccountKind(
          result.accountKind === "liability" ? "liability" : "asset",
        );
        setOpeningBalance(result.openingBalance ?? "");
        setStage("confirmAccount");
      } else {
        setProposal(result);
        setStage("confirmTransaction");
      }
      // Recognition succeeded — the dialog stays open for confirmation.
    },
    onError: (err) => {
      setSubmitError(translateErrorMessage(err, t));
    },
  });

  const createAccountMutation = useMutation({
    mutationFn: (input: Parameters<typeof accountCreate>[0]) =>
      accountCreate(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(t("ai.success"));
      onOpenChange(false);
      onSuccess?.();
    },
    onError: (err) => {
      setSubmitError(translateErrorMessage(err, t));
    },
  });

  const canSubmit = text.trim().length > 0 && !parseMutation.isPending;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    parseMutation.mutate(text.trim());
  };

  const canConfirmAccount =
    accountPath.trim().length > 0 && !createAccountMutation.isPending;

  const confirmCreateAccount = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canConfirmAccount) return;
    createAccountMutation.mutate({
      name: accountPath.trim(),
      initial_balance: openingBalance.trim() || undefined,
      initial_balance_date: proposal?.date,
      description: proposal?.description || undefined,
    });
  };

  const backToInput = () => {
    setStage("input");
    setProposal(null);
    setSubmitError(null);
  };

  return (
    <>
      <Dialog
        open={open && (stage === "input" || stage === "confirmAccount")}
        onOpenChange={onOpenChange}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Sparkles className="h-5 w-5" />
              {stage === "input" ? t("ai.title") : t("ai.review.title")}
            </DialogTitle>
          </DialogHeader>

          {stage === "input" && (
            <form onSubmit={handleSubmit}>
              <div className="space-y-4">
                <Textarea
                  value={text}
                  onChange={(e) => setText(e.target.value)}
                  placeholder={t("ai.inputPlaceholder")}
                  className="min-h-[80px] resize-none"
                  autoFocus
                  disabled={parseMutation.isPending}
                />

                {submitError && (
                  <p className="text-sm text-destructive">{submitError}</p>
                )}
              </div>

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => onOpenChange(false)}
                  disabled={parseMutation.isPending}
                >
                  {t("common.cancel")}
                </Button>
                <Button type="submit" disabled={!canSubmit}>
                  {parseMutation.isPending ? (
                    <>
                      <Loader2 className="h-4 w-4 animate-spin" />
                      {t("ai.recognizing")}
                    </>
                  ) : (
                    t("ai.button")
                  )}
                </Button>
              </DialogFooter>
            </form>
          )}

          {stage === "confirmAccount" && proposal && (
            <form onSubmit={confirmCreateAccount}>
              <div className="space-y-4">
                <ProposalReview proposal={proposal} />

                <div className="space-y-2">
                  <Label htmlFor="ai-account-path">
                    {t("ai.review.accountPath")}
                  </Label>
                  <Input
                    id="ai-account-path"
                    value={accountPath}
                    onChange={(e) => setAccountPath(e.target.value)}
                    placeholder="Assets/支付宝"
                  />
                </div>

                <div className="space-y-2">
                  <Label>{t("ai.review.accountKind")}</Label>
                  <Select
                    value={accountKind}
                    onValueChange={(v) =>
                      setAccountKind(v === "liability" ? "liability" : "asset")
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="asset">
                        {t("quickAdd.assetAccount")}
                      </SelectItem>
                      <SelectItem value="liability">
                        {t("quickAdd.liabilityAccount")}
                      </SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="ai-opening-balance">
                    {t("ai.review.openingBalance")}
                    <span className="text-muted-foreground text-xs ml-1">
                      ({t("common.optional")})
                    </span>
                  </Label>
                  <Input
                    id="ai-opening-balance"
                    type="number"
                    step="0.01"
                    min="0"
                    placeholder="0.00"
                    value={openingBalance}
                    onChange={(e) => setOpeningBalance(e.target.value)}
                    className="font-mono"
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
                  onClick={backToInput}
                  disabled={createAccountMutation.isPending}
                >
                  {t("ai.review.back")}
                </Button>
                <Button type="submit" disabled={!canConfirmAccount}>
                  {createAccountMutation.isPending
                    ? t("common.processing")
                    : t("common.save")}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>

      {stage === "confirmTransaction" && proposal && (
        <QuickAddDialog
          open={open}
          variant="review"
          initial={toQuickAddInitial(proposal)}
          onOpenChange={(nextOpen) => {
            if (!nextOpen) backToInput();
          }}
          onSuccess={() => {
            toast.success(t("ai.success"));
            onOpenChange(false);
            onSuccess?.();
          }}
        />
      )}
    </>
  );
}
