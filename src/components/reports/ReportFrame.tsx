import type { ReactNode } from "react";
import { AlertCircle, Inbox, Loader2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Card } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { formatAmount } from "@/utils/format";

/**
 * The chrome every report shares.
 *
 * Reports used to each own their loading spinner, their error block and their
 * empty message — three nearly identical copies of the same three states, all
 * slightly different heights, so switching reports made the page jump. The
 * frame fixes the order (title → scope line → toolbar → content) and renders
 * the states itself; a report only says which one it is in.
 *
 * The scope line is not decoration. Every number on this page has a counting
 * rule behind it — which postings count as income, whether transfers cancel,
 * whether a budget is today's definition or the one in force at the time — and
 * a report that hides its rule gets read as a fact.
 */
interface ReportFrameProps {
  title: string;
  scope?: string;
  isLoading?: boolean;
  error?: Error | null;
  isEmpty?: boolean;
  emptyHint?: string;
  /** Report-local controls: account picker, income/expense toggle, … */
  toolbar?: ReactNode;
  children?: ReactNode;
}

export function ReportFrame({
  title,
  scope,
  isLoading,
  error,
  isEmpty,
  emptyHint,
  toolbar,
  children,
}: ReportFrameProps) {
  const { t } = useTranslation();

  return (
    <section className="flex flex-col gap-4">
      <header className="flex flex-col gap-1">
        <h2 className="text-base font-medium">{title}</h2>
        {scope && <p className="text-xs text-muted-foreground">{scope}</p>}
      </header>

      {toolbar}

      {error ? (
        <div className="flex flex-col items-center justify-center gap-2 rounded-xl border bg-card py-14 text-destructive">
          <AlertCircle className="h-8 w-8" />
          <p className="text-sm">{t("reports.common.error")}</p>
          <p className="text-xs text-muted-foreground">{error.message}</p>
        </div>
      ) : isLoading ? (
        <div className="flex items-center justify-center gap-2 rounded-xl border bg-card py-14 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          <span className="text-sm">{t("reports.common.loading")}</span>
        </div>
      ) : isEmpty ? (
        <div className="flex flex-col items-center justify-center gap-2 rounded-xl border bg-card py-14 text-muted-foreground">
          <Inbox className="h-8 w-8 opacity-50" />
          <p className="text-sm">{emptyHint ?? t("reports.common.noData")}</p>
        </div>
      ) : (
        children
      )}
    </section>
  );
}

/**
 * The figures a report opens with, on one baseline each and separated by a
 * hairline rather than by four separate cards. Four cards read as four places
 * to look; one strip reads as one answer in four parts.
 */
export interface ReportStat {
  label: string;
  value: ReactNode;
  hint?: ReactNode;
  tone?: "plain" | "income" | "expense" | "signal";
}

const TONE: Record<NonNullable<ReportStat["tone"]>, string> = {
  plain: "text-foreground",
  income: "text-income",
  expense: "text-expense",
  signal: "text-signal",
};

export function ReportStats({ items }: { items: ReportStat[] }) {
  return (
    <div
      data-elevation="raise"
      className="flex items-stretch rounded-xl border bg-card px-4 py-3"
    >
      {items.map((item, index) => (
        <div
          key={item.label}
          className={cn(
            "min-w-0 flex-1",
            index > 0 && "border-l border-border pl-4",
          )}
        >
          <div className="text-xs text-muted-foreground">{item.label}</div>
          <div
            className={cn(
              "mt-1 truncate font-mono text-xl font-medium tracking-tight",
              TONE[item.tone ?? "plain"],
            )}
          >
            {item.value}
          </div>
          {item.hint && (
            <div className="mt-1 truncate text-xs text-muted-foreground">
              {item.hint}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

/** A titled panel. `hint` is the line that keeps the chart honest about units. */
export function ReportPanel({
  title,
  hint,
  action,
  children,
  className,
}: {
  title?: string;
  hint?: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={cn("gap-3", className)}>
      {(title || action) && (
        <div className="flex items-start justify-between gap-4 px-4">
          <div>
            {title && <div className="text-sm font-medium">{title}</div>}
            {hint && (
              <div className="mt-1 text-xs text-muted-foreground">{hint}</div>
            )}
          </div>
          {action}
        </div>
      )}
      <div className="px-4">{children}</div>
    </Card>
  );
}

export interface ReportColumn<T> {
  header: ReactNode;
  align?: "left" | "right";
  className?: string;
  cell: (row: T) => ReactNode;
}

/**
 * One table style for the whole page: header on the sunken surface, hairline
 * rows, numerals right-aligned in the mono face. Reports had four slightly
 * different tables before, which is how a page starts looking assembled rather
 * than designed.
 */
export function ReportTable<T>({
  columns,
  rows,
  rowKey,
  footer,
}: {
  columns: ReportColumn<T>[];
  rows: T[];
  rowKey: (row: T) => string;
  footer?: ReactNode;
}) {
  return (
    <div className="-mx-4 overflow-x-auto">
      <table className="w-full text-sm">
        <thead className="bg-muted/60 text-xs text-muted-foreground">
          <tr>
            {columns.map((column, index) => (
              <th
                key={index}
                className={cn(
                  "border-y border-border px-4 py-2 font-normal whitespace-nowrap",
                  column.align === "right" ? "text-right" : "text-left",
                )}
              >
                {column.header}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={rowKey(row)} className="border-b border-border/60">
              {columns.map((column, index) => (
                <td
                  key={index}
                  className={cn(
                    "px-4 py-2",
                    column.align === "right"
                      ? "text-right font-mono"
                      : "text-left",
                    column.className,
                  )}
                >
                  {column.cell(row)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
        {footer && <tfoot className="bg-muted/40">{footer}</tfoot>}
      </table>
    </div>
  );
}

/** A money figure in the mono face, coloured by direction unless told not to. */
export function Amount({
  value,
  tone = "auto",
  className,
}: {
  value: number;
  tone?: "auto" | "income" | "expense" | "plain";
  className?: string;
}) {
  const resolved =
    tone === "auto"
      ? value >= 0
        ? "text-income"
        : "text-expense"
      : TONE[tone];
  return (
    <span className={cn("font-mono", resolved, className)}>
      {formatAmount(value)}
    </span>
  );
}
