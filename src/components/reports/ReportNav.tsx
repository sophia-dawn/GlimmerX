import {
  Activity,
  PieChart,
  Scale,
  Target,
  TrendingUp,
  Wallet,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { REPORT_LIST, type ReportId } from "@/constants/report";
import { cn } from "@/lib/utils";

const ICONS: Record<ReportId, LucideIcon> = {
  cashflow: Activity,
  categories: PieChart,
  netWorth: TrendingUp,
  balanceSheet: Scale,
  accounts: Wallet,
  budgets: Target,
};

/**
 * The report index, as a column rather than a dropdown.
 *
 * A `<Select>` in the page header hid the whole catalogue behind a click: the
 * nine reports had to be discovered one at a time, and the list gave no sense
 * of what was there. Six entries fit in a column, so the page shows its own
 * table of contents and switching costs one click instead of two.
 *
 * The active row is marked by a signal bar in the gutter — the same "current
 * item" role the accent has everywhere else — not by a filled pill.
 */
export function ReportNav({
  selected,
  onSelect,
}: {
  selected: ReportId;
  onSelect: (id: ReportId) => void;
}) {
  const { t } = useTranslation();

  return (
    <nav
      aria-label={t("reports.title")}
      data-print="hide"
      className="flex w-[188px] shrink-0 flex-col gap-0.5 border-r border-border py-4 pr-3 pl-4"
    >
      {REPORT_LIST.map((report) => {
        const Icon = ICONS[report.id];
        const active = report.id === selected;
        return (
          <button
            key={report.id}
            type="button"
            onClick={() => onSelect(report.id)}
            aria-current={active ? "page" : undefined}
            className={cn(
              "relative flex items-center gap-2.5 rounded-md px-2.5 py-2 text-left text-sm transition-colors",
              active
                ? "bg-secondary text-foreground"
                : "text-muted-foreground hover:bg-secondary/50 hover:text-foreground",
            )}
          >
            {active && (
              <span
                aria-hidden
                className="absolute top-1.5 bottom-1.5 -left-2 w-[2px] rounded-full bg-signal"
              />
            )}
            <Icon
              className={cn("h-3.5 w-3.5 shrink-0", active && "text-signal")}
            />
            <span className="truncate">{t(report.nameKey)}</span>
          </button>
        );
      })}
    </nav>
  );
}
