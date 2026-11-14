import { useTranslation } from "react-i18next";
import { useDashboardSummary } from "@/hooks/useDashboardSummary";
import { formatAmount, formatAmountWithSign } from "@/utils/format";
import { ActivityCalendar } from "./ActivityCalendar";

/**
 * Conclusion layer: one number, then its components.
 *
 * The strip used to be four equally sized figures — net worth, income, expense,
 * balance — each 20 px on its own baseline. Four peers cannot have a focus, and
 * a glance at a ledger should answer "where do I stand" before anything else.
 * Net worth now owns the display step and the rest become 11 px annotations on
 * one line. The right half is the 90-day activity calendar, which shares the
 * page's 32 px/week substrate and is the reason that grid earns its ink.
 *
 * `data-elevation="raise"` puts this on the same beveled surface as the panels
 * below, so the whole page reads as one instrument rather than a strip pasted
 * on top of a set of cards.
 */
export function DashboardStatusBar() {
  const { t } = useTranslation();
  const { data: summary, isLoading, error } = useDashboardSummary();

  if (error) {
    return (
      <div className="border-y border-border bg-card px-4 py-3 text-sm text-destructive">
        {t("common.errorGeneric")}: {String(error)}
      </div>
    );
  }

  const income = summary?.month_income ?? 0;
  const expense = summary?.month_expense ?? 0;
  const netWorth = summary?.net_worth ?? 0;
  const balance = income - expense;

  return (
    <div
      data-elevation="raise"
      className="flex shrink-0 flex-col gap-3 rounded-xl border bg-card px-4 py-3 sm:flex-row sm:items-stretch sm:gap-4"
    >
      <div className="flex min-w-0 flex-col justify-center sm:basis-[42%]">
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span>{t("dashboard.netWorth")}</span>
          <span className="inline-flex items-center gap-1">
            <span
              aria-hidden
              className="h-1.5 w-1.5 animate-pulse rounded-full bg-signal"
            />
            {t("dashboard.live")}
          </span>
        </div>

        {/* Deliberately not `cn()`: tailwind-merge reads an unknown `text-*`
            class as a text colour, so it would drop `text-display` in favour
            of `text-foreground` and silently shrink the number back to 13 px. */}
        <div
          className={`mt-1 font-mono text-display font-medium tracking-tight ${
            !isLoading && netWorth < 0 ? "text-expense" : "text-foreground"
          }`}
        >
          {isLoading ? "--" : formatAmount(netWorth)}
        </div>

        <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-muted-foreground">
          <span className="font-mono text-income">
            {isLoading ? "--" : formatAmountWithSign(balance)}
          </span>
          <span>{t("dashboard.monthBalance")}</span>
          <span aria-hidden>·</span>
          <span>
            {t("dashboard.assetsShort")}{" "}
            <b className="font-mono font-normal text-foreground">
              {formatAmount(summary?.total_assets ?? 0)}
            </b>
          </span>
          <span>
            {t("dashboard.liabilitiesShort")}{" "}
            <b className="font-mono font-normal text-expense">
              {formatAmount(summary?.total_liabilities ?? 0)}
            </b>
          </span>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 border-t border-border pt-3 sm:border-t-0 sm:border-l sm:pl-4 sm:pt-0">
        <ActivityCalendar />
      </div>
    </div>
  );
}
