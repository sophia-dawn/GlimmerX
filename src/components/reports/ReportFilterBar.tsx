import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { MultiSelectButton } from "./MultiSelectButton";
import { DATE_RANGE_PRESETS, GRANULARITY_OPTIONS } from "@/constants/report";
import type { ReportFilter } from "@/types/report";
import type { DateRangePreset, PeriodGranularity } from "@/types/report";
import { useAccountList } from "@/hooks/useAccountList";
import { useCategoryList } from "@/hooks/useCategoryList";
import { useTranslation } from "react-i18next";

interface ReportFilterBarProps {
  filter: ReportFilter;
  setDateRangePreset: (preset: DateRangePreset) => void;
  setStartDate: (date: string | undefined) => void;
  setEndDate: (date: string | undefined) => void;
  setPeriodGranularity?: (granularity: PeriodGranularity) => void;
  setAccountIds?: (ids: string[] | undefined) => void;
  setCategoryIds?: (ids: string[] | undefined) => void;
  resetFilter?: () => void;
  onExport?: () => void;
  showAccountFilter?: boolean;
  showCategoryFilter?: boolean;
}

export function ReportFilterBar({
  filter,
  setDateRangePreset,
  setStartDate,
  setEndDate,
  setPeriodGranularity,
  setAccountIds,
  setCategoryIds,
  resetFilter,
  onExport,
  showAccountFilter = false,
  showCategoryFilter = false,
}: ReportFilterBarProps) {
  const { t } = useTranslation();
  const { data: accounts } = useAccountList();
  const { data: categories } = useCategoryList();

  const accountOptions = (accounts ?? []).map((a) => ({
    id: a.id,
    name: a.name,
  }));

  const categoryOptions = (categories ?? []).map((c) => ({
    id: c.id,
    name: c.name,
  }));

  return (
    <div className="px-4 py-3 border-b bg-muted/20">
      <div className="flex flex-col gap-3">
        <div className="flex items-center gap-3 flex-wrap">
          <div className="flex items-center gap-2">
            <Label className="text-xs whitespace-nowrap">
              {t("reports.common.dateRange")}
            </Label>
            <Select
              value={filter.dateRangePreset}
              onValueChange={(v) => setDateRangePreset(v as DateRangePreset)}
            >
              <SelectTrigger className="w-[130px]">
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
          </div>

          {filter.dateRangePreset === "custom" && (
            <div className="flex items-center gap-2">
              <DatePicker
                value={filter.startDate}
                onChange={(d) => setStartDate(d)}
                className="w-[130px]"
              />
              <span className="text-muted-foreground">~</span>
              <DatePicker
                value={filter.endDate}
                onChange={(d) => setEndDate(d)}
                className="w-[130px]"
              />
            </div>
          )}

          {setPeriodGranularity && (
            <div className="flex items-center gap-2">
              <Label className="text-xs whitespace-nowrap">
                {t("reports.common.granularity")}
              </Label>
              <Select
                value={filter.periodGranularity}
                onValueChange={(v) =>
                  setPeriodGranularity(v as PeriodGranularity)
                }
              >
                <SelectTrigger className="w-[90px]">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {GRANULARITY_OPTIONS.map((option) => (
                    <SelectItem key={option.id} value={option.id}>
                      {t(option.labelKey)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          )}
        </div>

        <div className="flex items-center gap-3 flex-wrap">
          {showAccountFilter && setAccountIds && (
            <MultiSelectButton
              label={t("reports.common.accounts")}
              options={accountOptions}
              selected={filter.accountIds}
              onSelect={setAccountIds}
              className="w-[160px]"
            />
          )}

          {showCategoryFilter && setCategoryIds && (
            <MultiSelectButton
              label={t("reports.common.categories")}
              options={categoryOptions}
              selected={filter.categoryIds}
              onSelect={setCategoryIds}
              className="w-[160px]"
            />
          )}

          {resetFilter && (
            <Button variant="outline" size="sm" onClick={resetFilter}>
              {t("reports.common.reset")}
            </Button>
          )}

          {onExport && (
            <Button variant="default" size="sm" onClick={onExport}>
              {t("reports.common.export")}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
