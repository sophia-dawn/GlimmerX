import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { useDashboardSummary } from "@/hooks/useDashboardSummary";
import { formatAmount, formatAmountWithSign } from "@/utils/format";
import { cn } from "@/lib/utils";

interface StatusItem {
  key: string;
  label: string;
  value: string;
  colorClass: string;
  hint?: ReactNode;
}

/**
 * The four figures that answer "where do I stand" before any scrolling.
 *
 * Cards were the wrong shape for this. Four borders, four headings and four
 * icons to read four numbers — each of which also started on its own baseline,
 * so the amounts could not be compared without moving the eye up and down. One
 * rule-separated strip puts them on a single line: the numbers share a baseline,
 * a column width and a font, which is the only arrangement where a glance works.
 *
 * Net worth carries the asset/liability split underneath it rather than taking
 * two more cells, because those are already its components — showing them apart
 * from it invites adding them twice.
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

  const items: StatusItem[] = [
    {
      key: "netWorth",
      label: t("dashboard.netWorth"),
      value: formatAmount(netWorth),
      colorClass: netWorth >= 0 ? "text-income" : "text-expense",
      hint: (
        <>
          {t("dashboard.assetsShort")}{" "}
          <span className="font-mono text-foreground">
            {formatAmount(summary?.total_assets ?? 0)}
          </span>
          <span className="mx-1.5">·</span>
          {t("dashboard.liabilitiesShort")}{" "}
          <span className="font-mono text-foreground">
            {formatAmount(summary?.total_liabilities ?? 0)}
          </span>
        </>
      ),
    },
    {
      key: "monthIncome",
      label: t("dashboard.monthIncome"),
      value: formatAmountWithSign(income),
      colorClass: "text-income",
    },
    {
      key: "monthExpense",
      label: t("dashboard.monthExpense"),
      value: formatAmountWithSign(-expense),
      colorClass: "text-expense",
    },
    {
      key: "monthBalance",
      label: t("dashboard.monthBalance"),
      value: formatAmountWithSign(balance),
      colorClass: balance >= 0 ? "text-income" : "text-expense",
    },
  ];

  return (
    <dl className="flex flex-col divide-y divide-border border-y border-border bg-card sm:flex-row sm:divide-x sm:divide-y-0">
      {items.map((item) => (
        <div key={item.key} className="min-w-0 flex-1 px-4 py-3">
          <dt className="text-xs text-muted-foreground">{item.label}</dt>
          <dd
            className={cn(
              "mt-1 font-mono text-xl font-medium",
              item.colorClass,
            )}
          >
            {isLoading ? "--" : item.value}
          </dd>
          {item.hint && (
            <div className="mt-0.5 truncate text-xs text-muted-foreground">
              {item.hint}
            </div>
          )}
        </div>
      ))}
    </dl>
  );
}
