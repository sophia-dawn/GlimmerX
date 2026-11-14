import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GRANULARITY_OPTIONS } from "@/constants/report";
import type { PeriodGranularity } from "@/types/report";

/**
 * The tick size of a chart's time axis.
 *
 * It lives on the chart rather than in the page-level filter bar because it is
 * a property of the axis, not of the report: the range says how long the axis
 * is, this says how it is cut. Only the tick sizes the current range can
 * actually draw are offered — a span of one month has no weekly reading worth
 * two lonely bars — and when there is only one of them there is no choice to
 * make, so the control says nothing.
 */
export function ChartScaleSelect({
  value,
  options,
  onChange,
}: {
  value: PeriodGranularity;
  options: PeriodGranularity[];
  onChange: (granularity: PeriodGranularity) => void;
}) {
  const { t } = useTranslation();

  if (options.length < 2) return null;

  return (
    <Select
      value={value}
      onValueChange={(next) => onChange(next as PeriodGranularity)}
    >
      <SelectTrigger
        size="sm"
        className="w-[92px]"
        title={t("reports.common.displayAs")}
      >
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {options.map((option) => {
          const label = GRANULARITY_OPTIONS.find(
            (entry) => entry.id === option,
          );
          return (
            <SelectItem key={option} value={option}>
              {label ? t(label.labelKey) : option}
            </SelectItem>
          );
        })}
      </SelectContent>
    </Select>
  );
}
