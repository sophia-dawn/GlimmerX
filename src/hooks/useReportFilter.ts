import { useState, useCallback } from "react";
import type {
  ReportFilter,
  DateRangePreset,
  PeriodGranularity,
} from "@/types/report";

const DEFAULT_FILTER: ReportFilter = {
  dateRangePreset: "currentMonth",
  periodGranularity: "monthly",
  accountIds: undefined,
  categoryIds: undefined,
};

export function useReportFilter() {
  const [filter, setFilter] = useState<ReportFilter>(DEFAULT_FILTER);

  const setDateRangePreset = useCallback((preset: DateRangePreset) => {
    setFilter((prev) => ({
      ...prev,
      dateRangePreset: preset,
      startDate: preset === "custom" ? prev.startDate : undefined,
      endDate: preset === "custom" ? prev.endDate : undefined,
    }));
  }, []);

  const setStartDate = useCallback((date: string | undefined) => {
    setFilter((prev) => ({ ...prev, startDate: date }));
  }, []);

  const setEndDate = useCallback((date: string | undefined) => {
    setFilter((prev) => ({ ...prev, endDate: date }));
  }, []);

  const setPeriodGranularity = useCallback((granularity: PeriodGranularity) => {
    setFilter((prev) => ({ ...prev, periodGranularity: granularity }));
  }, []);

  const setAccountIds = useCallback((ids: string[] | undefined) => {
    setFilter((prev) => ({ ...prev, accountIds: ids }));
  }, []);

  const setCategoryIds = useCallback((ids: string[] | undefined) => {
    setFilter((prev) => ({ ...prev, categoryIds: ids }));
  }, []);

  const resetFilter = useCallback(() => {
    setFilter(DEFAULT_FILTER);
  }, []);

  return {
    filter,
    setFilter,
    setDateRangePreset,
    setStartDate,
    setEndDate,
    setPeriodGranularity,
    setAccountIds,
    setCategoryIds,
    resetFilter,
  };
}
