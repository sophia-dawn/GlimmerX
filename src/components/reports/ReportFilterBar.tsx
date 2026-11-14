import { RotateCcw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { MonthPicker } from "@/components/ui/month-picker";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DATE_RANGE_PRESETS } from "@/constants/report";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import type { DateRangePreset } from "@/types/report";

/**
 * The one filter bar.
 *
 * What it offers is decided by the report, not by the user's patience: a range
 * for cash flow, categories, net worth and accounts; a single date for the
 * balance sheet; a month for budgets. Offering a range where the data is a
 * point is how the old page ended up with four copies of a date picker that
 * three of them did not need.
 *
 * The tick size of a chart's axis is not here: it is not a property of the
 * report but of the chart, so it sits on the chart (see `ChartScaleSelect`).
 *
 * The state lives in the URL (see `useReportQuery`), so the reset button is
 * really "go back to the default period" rather than "clear local state".
 */
export function ReportFilterBar({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t } = useTranslation();
  const { query, definition } = controller;

  const onPreset = (value: string) =>
    controller.setPreset(value as DateRangePreset);

  return (
    <div
      data-print="hide"
      className="flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl border bg-card px-4 py-2.5"
    >
      {definition.dateControl === "range" && (
        <>
          <Select value={query.preset} onValueChange={onPreset}>
            <SelectTrigger size="sm" className="w-[124px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {DATE_RANGE_PRESETS.map((preset) => (
                <SelectItem key={preset.id} value={preset.id}>
                  {t(preset.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {query.preset === "custom" && (
            <div className="flex items-center gap-1.5">
              <DatePicker
                value={query.start || undefined}
                onChange={(date) => controller.setCustomRange(date, query.end)}
                className="w-[136px]"
              />
              <span className="text-xs text-muted-foreground">~</span>
              <DatePicker
                value={query.end || undefined}
                onChange={(date) =>
                  controller.setCustomRange(query.start, date)
                }
                className="w-[136px]"
              />
            </div>
          )}
        </>
      )}

      {definition.dateControl === "snapshot" && (
        <>
          <span className="text-xs text-muted-foreground">
            {t("reports.balanceSheet.snapshotDate")}
          </span>
          <DatePicker
            value={query.anchor}
            onChange={controller.setAnchor}
            className="w-[150px]"
          />
        </>
      )}

      {definition.dateControl === "month" && (
        <>
          <span className="text-xs text-muted-foreground">
            {t("reports.budgets.monthLabel")}
          </span>
          <MonthPicker
            value={controller.anchorMonthValue}
            onChange={(month) => controller.setAnchor(`${month}-01`)}
            className="w-[150px]"
          />
        </>
      )}

      <div className="ml-auto">
        <Button
          variant="ghost"
          size="sm"
          onClick={controller.reset}
          className="text-muted-foreground"
        >
          <RotateCcw className="mr-1.5 h-3.5 w-3.5" />
          {t("reports.common.reset")}
        </Button>
      </div>
    </div>
  );
}
