import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useMonthlyChart } from "@/hooks/useMonthlyChart";
import { EChart } from "@/components/charts/EChart";
import type { EChartsCoreOption } from "@/components/charts/echarts";
import { useChartColors } from "@/utils/chartColors";
import {
  categoryAxis,
  chartAnimation,
  chartGrid,
  chartLegend,
  chartTextStyle,
  chartTooltip,
  lineSeriesDefaults,
  valueAxis,
  yuanTick,
} from "@/utils/chartTheme";
import { formatAmount } from "@/utils/format";

export function MonthlyIncomeExpenseChart() {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const { data, isLoading, error } = useMonthlyChart();

  // Transform data for the chart: convert cents to yuan, show "MM-DD".
  const chartData = useMemo(
    () =>
      data?.days.map((day) => ({
        date: day.date.slice(5),
        income: day.income / 100,
        expense: day.expense / 100,
        hasTransactions: day.has_transactions,
      })) ?? [],
    [data],
  );

  const option = useMemo<EChartsCoreOption>(() => {
    const labels = chartData.map((day) => day.date);
    const money = (value: number) => formatAmount(Math.round(value * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      color: [colors.income, colors.expense],
      grid: chartGrid({ top: 28 }),
      legend: {
        ...chartLegend(colors),
        data: [
          t("dashboard.monthlyChartIncome"),
          t("dashboard.monthlyChartExpense"),
        ],
      },
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const params = raw as {
            name: string;
            marker: string;
            seriesName: string;
            value: number;
          }[];
          if (!params.length) return "";
          const rows = params
            .map((p) => `${p.marker}${p.seriesName}：${money(Number(p.value))}`)
            .join("<br/>");
          return `${params[0]?.name ?? ""}<br/>${rows}`;
        },
      }),
      xAxis: {
        ...categoryAxis(colors),
        data: labels,
        axisLabel: {
          ...categoryAxis(colors).axisLabel,
          interval: "auto",
        },
      },
      yAxis: valueAxis(colors, yuanTick(i18n.language)),
      series: [
        {
          ...lineSeriesDefaults(colors),
          name: t("dashboard.monthlyChartIncome"),
          data: chartData.map((day) => day.income),
          itemStyle: { color: colors.income },
          areaStyle: { opacity: 0 },
        },
        {
          ...lineSeriesDefaults(colors),
          name: t("dashboard.monthlyChartExpense"),
          data: chartData.map((day) => day.expense),
          itemStyle: { color: colors.expense },
        },
      ],
    };
  }, [chartData, colors, i18n.language, t]);

  const hasData = chartData.some((d) => d.hasTransactions);

  return (
    <Card elevation="raise" className="flex min-h-[190px] flex-1 flex-col">
      <CardHeader>
        <CardTitle>{t("dashboard.monthlyChart")}</CardTitle>
      </CardHeader>
      <CardContent className="min-h-0 flex-1">
        {error ? (
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        ) : isLoading ? (
          <div className="flex h-full min-h-[150px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.loading")}</p>
          </div>
        ) : !hasData ? (
          <div className="flex h-full min-h-[150px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.noRecords")}</p>
          </div>
        ) : (
          <EChart option={option} label={t("dashboard.monthlyChart")} />
        )}
      </CardContent>
    </Card>
  );
}
