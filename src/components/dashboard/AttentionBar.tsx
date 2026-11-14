import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { useActivityCalendar } from "@/hooks/useActivityCalendar";
import { useBudgetStatuses } from "@/hooks/useBudgetStatuses";

/** No entry for three days is worth a nudge; a quiet weekend is not. */
const STALE_AFTER_DAYS = 3;

interface AttentionItem {
  to: string;
  label: string;
  value: string;
}

/**
 * Stripe's "is anything waiting for me?" layer, expressed in double-entry
 * terms. Both signals come from data the ledger already owns: an over-budget
 * category from `budget_list_statuses`, and the gap since the last posting from
 * the transactions themselves. Nothing here is a fabricated notification.
 *
 * It stays visible when empty. A row that appears only when something is wrong
 * makes the page jump, and "all clear" is itself information worth showing.
 */
export function AttentionBar() {
  const { t } = useTranslation();
  const { data: budgets } = useBudgetStatuses();
  const { data: activity } = useActivityCalendar();

  const items: AttentionItem[] = [];

  const overBudget = budgets?.filter((budget) => budget.overBudget) ?? [];
  if (overBudget.length > 0) {
    items.push({
      to: "/budgets",
      label: t("dashboard.attentionOverBudget"),
      value: String(overBudget.length),
    });
  }

  const stale = activity?.daysSinceLastEntry;
  if (stale != null && stale >= STALE_AFTER_DAYS) {
    items.push({
      to: "/transactions",
      label: t("dashboard.attentionStale"),
      value: t("dashboard.attentionDays", { days: stale }),
    });
  }

  return (
    <div className="flex shrink-0 flex-wrap items-center gap-2 rounded-md border border-signal/30 bg-signal/5 px-3 py-2">
      <span className="text-xs tracking-wide text-signal">
        {t("dashboard.attention")}
      </span>

      {items.length === 0 ? (
        <span className="text-xs text-muted-foreground">
          {t("dashboard.attentionClear")}
        </span>
      ) : (
        items.map((item) => (
          <Link
            key={item.label}
            to={item.to}
            className="inline-flex h-6 items-center gap-2 rounded-sm border border-border bg-card px-2 text-xs text-foreground transition-colors hover:border-signal/50"
          >
            <span aria-hidden className="h-1.5 w-1.5 rounded-full bg-signal" />
            {item.label}
            <strong className="font-mono font-normal text-signal">
              {item.value}
            </strong>
          </Link>
        ))
      )}

      <Link
        to="/transactions"
        className="ml-auto text-xs text-muted-foreground transition-colors hover:text-foreground"
      >
        {t("dashboard.viewAll")} →
      </Link>
    </div>
  );
}
