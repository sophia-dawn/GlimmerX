import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ResponsiveContainer,
  ComposedChart,
  Line,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
  Legend,
} from "recharts";
import { AlertCircle, Loader2 } from "lucide-react";
import { ReportFilterBar } from "./ReportFilterBar";
import { useTrendReport } from "@/hooks/useTrendReport";
import { formatAmount } from "@/utils/format";
import { getDefaultRangePresetDates } from "@/utils/date";
import { GRANULARITY_OPTIONS, REPORT_CHART_COLORS } from "@/constants/report";
import { useTranslation } from "react-i18next";
import { useCurrency } from "@/hooks/useCurrency";
import { useState } from "react";
import type {
  DateRangePreset,
  PeriodGranularity,
  ReportFilter,
} from "@/types/report";

export function TrendReport() {
  const { t } = useTranslation();
  const { symbol } = useCurrency();

  // 篩选状态
  const [dateRangePreset, setDateRangePreset] =
    useState<DateRangePreset>("currentMonth");
  const [startDate, setStartDate] = useState<string | undefined>(undefined);
  const [endDate, setEndDate] = useState<string | undefined>(undefined);
  const [periodGranularity, setPeriodGranularity] =
    useState<PeriodGranularity>("monthly");
  const [categoryIds, setCategoryIds] = useState<string[] | undefined>(
    undefined,
  );

  // 计算日期范围
  const range =
    dateRangePreset === "custom"
      ? { startDate: startDate ?? "", endDate: endDate ?? "" }
      : getDefaultRangePresetDates(dateRangePreset);

  // 构建 filter 对象
  const filter: ReportFilter = {
    dateRangePreset,
    startDate: range.startDate,
    endDate: range.endDate,
    periodGranularity,
    accountIds: undefined,
    categoryIds,
  };

  const { data, isLoading, error } = useTrendReport(filter);

  // 重置筛选器
  const resetFilter = () => {
    setDateRangePreset("currentMonth");
    setStartDate(undefined);
    setEndDate(undefined);
    setPeriodGranularity("monthly");
    setCategoryIds(undefined);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-destructive">
        <AlertCircle className="h-12 w-12 mb-4" />
        <p className="text-lg">{t("reports.common.error")}</p>
        <p className="text-sm mt-1">{error.message}</p>
      </div>
    );
  }

  const chartData =
    data?.dataPoints.map((point) => ({
      period: point.period,
      income: point.income / 100,
      expense: point.expense / 100,
      net: point.net / 100,
    })) ?? [];

  const totalIncome = data?.totalIncome ?? 0;
  const totalExpense = data?.totalExpense ?? 0;
  const totalNet = data?.totalNet ?? 0;

  const granularityLabel = GRANULARITY_OPTIONS.find(
    (g) => g.id === filter.periodGranularity,
  );

  return (
    <div className="space-y-4">
      <ReportFilterBar
        filter={filter}
        setDateRangePreset={setDateRangePreset}
        setStartDate={setStartDate}
        setEndDate={setEndDate}
        setPeriodGranularity={setPeriodGranularity}
        setCategoryIds={setCategoryIds}
        resetFilter={resetFilter}
        showCategoryFilter={true}
      />

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.trend.chart")}</CardTitle>
        </CardHeader>
        <CardContent>
          {chartData.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-8 text-muted-foreground">
              <AlertCircle className="h-10 w-10 mb-3 opacity-50" />
              <p className="text-lg">{t("reports.common.noData")}</p>
            </div>
          ) : (
            <ResponsiveContainer width="100%" height={300}>
              <ComposedChart data={chartData}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis dataKey="period" />
                <YAxis />
                <Tooltip
                  formatter={(value) =>
                    value !== undefined
                      ? `${symbol}${Number(value).toFixed(2)}`
                      : ""
                  }
                />
                <Legend />
                <Area
                  type="monotone"
                  dataKey="income"
                  name={t("reports.trend.totalIncome")}
                  stroke={REPORT_CHART_COLORS.income}
                  fill={REPORT_CHART_COLORS.income}
                  fillOpacity={0.3}
                />
                <Area
                  type="monotone"
                  dataKey="expense"
                  name={t("reports.trend.totalExpense")}
                  stroke={REPORT_CHART_COLORS.expense}
                  fill={REPORT_CHART_COLORS.expense}
                  fillOpacity={0.3}
                />
                <Line
                  type="monotone"
                  dataKey="net"
                  name={t("reports.trend.totalNet")}
                  stroke={REPORT_CHART_COLORS.net}
                  strokeWidth={2}
                />
              </ComposedChart>
            </ResponsiveContainer>
          )}
        </CardContent>
      </Card>

      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.trend.totalIncome")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {formatAmount(totalIncome)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.trend.totalExpense")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">
              {formatAmount(totalExpense)}
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.trend.totalNet")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div
              className={`text-2xl font-bold ${totalNet >= 0 ? "text-green-600" : "text-red-600"}`}
            >
              {formatAmount(totalNet)}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardContent className="py-4">
          <div className="flex items-center gap-2 text-muted-foreground">
            <span>
              {t("reports.trend.currentGranularity")}:{" "}
              {granularityLabel
                ? t(granularityLabel.labelKey)
                : filter.periodGranularity}
            </span>
            <span className="text-sm">
              ({data?.granularity ?? filter.periodGranularity})
            </span>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
