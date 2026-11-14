import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { mcpApproveRequest, mcpCurrentApproval } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { formatAmount } from "@/utils/format";
import type { McpApprovalRequest } from "@/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ShieldAlert } from "lucide-react";

/**
 * Global confirmation card for MCP writes in "confirm" trust mode
 * (design doc 25, §5.4). Listens for `ledger://approval` events from the
 * backend and resolves the pending request with the user's decision.
 *
 * The project has hard deletes and no undo, so an agent's fuzzy match that
 * slips through here is permanent — the card shows exactly what the agent
 * asked to write, verbatim.
 */
export function McpApprovalDialog() {
  const { t } = useTranslation();
  const [pending, setPending] = useState<McpApprovalRequest | null>(null);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // The event is the normal path; the initial fetch covers the window
    // being reloaded while a request waits.
    let unlisten: (() => void) | undefined;
    let alive = true;
    listen<McpApprovalRequest>("ledger://approval", (event) => {
      setPending(event.payload);
      setError(null);
    }).then((fn) => {
      if (alive) unlisten = fn;
      else fn();
    });
    mcpCurrentApproval().then((request) => {
      if (alive && request) setPending(request);
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, []);

  const decide = async (approved: boolean) => {
    if (!pending) return;
    setSubmitting(true);
    setError(null);
    try {
      // false = already handled elsewhere; the card simply closes.
      await mcpApproveRequest(pending.requestId, approved);
      setPending(null);
    } catch (err) {
      setError(translateErrorMessage(err, t));
    } finally {
      setSubmitting(false);
    }
  };

  const req = pending?.request ?? {};
  const amountCents =
    typeof req.amountCents === "number" ? req.amountCents : null;
  const openingCents =
    typeof req.openingBalanceCents === "number"
      ? req.openingBalanceCents
      : null;

  return (
    <Dialog
      open={pending !== null}
      onOpenChange={(open) => {
        if (!open) decide(false);
      }}
    >
      <DialogContent
        className="sm:max-w-md"
        onInteractOutside={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <ShieldAlert className="h-4 w-4 text-signal" />
            {t("mcp.approval.title")}
          </DialogTitle>
          <DialogDescription>
            {pending?.tool === "create_account"
              ? t("mcp.approval.createAccountDesc")
              : t("mcp.approval.transactionDesc")}
          </DialogDescription>
        </DialogHeader>

        <dl className="space-y-1.5 text-sm">
          {amountCents !== null && (
            <div className="flex items-baseline justify-between gap-4">
              <dt className="text-muted-foreground">
                {t("mcp.approval.amount")}
              </dt>
              <dd className="font-mono text-base">
                {formatAmount(amountCents)}
              </dd>
            </div>
          )}
          {typeof req.kind === "string" && (
            <Row
              label={t("mcp.approval.kind")}
              value={t(`mcp.kind.${req.kind}`)}
            />
          )}
          {typeof req.date === "string" && (
            <Row label={t("mcp.approval.date")} value={req.date} />
          )}
          {typeof req.sourceAccount === "string" && (
            <Row label={t("mcp.approval.source")} value={req.sourceAccount} />
          )}
          {typeof req.destinationAccount === "string" && (
            <Row
              label={t("mcp.approval.destination")}
              value={req.destinationAccount}
            />
          )}
          {typeof req.category === "string" && (
            <Row label={t("mcp.approval.category")} value={req.category} />
          )}
          {typeof req.path === "string" && (
            <Row label={t("mcp.approval.path")} value={req.path} />
          )}
          {openingCents !== null && (
            <Row
              label={t("mcp.approval.openingBalance")}
              value={formatAmount(openingCents)}
            />
          )}
          {typeof req.description === "string" && req.description !== "" && (
            <Row
              label={t("mcp.approval.description")}
              value={req.description}
            />
          )}
        </dl>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={submitting}
            onClick={() => decide(false)}
          >
            {t("mcp.approval.reject")}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={submitting}
            onClick={() => decide(true)}
          >
            {t("mcp.approval.approve")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline justify-between gap-4">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="text-right font-mono text-sm">{value}</dd>
    </div>
  );
}
