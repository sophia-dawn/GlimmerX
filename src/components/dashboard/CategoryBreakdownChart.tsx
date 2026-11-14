import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useCategoryBreakdown } from "@/hooks/useCategoryBreakdown";
import { EChart } from "@/components/charts/EChart";
import type { EChartsCoreOption } from "@/components/charts/echarts";
import { useChartColors } from "@/utils/chartColors";
import {
  chartAnimation,
  chartTextStyle,
  chartTooltip,
} from "@/utils/chartTheme";
import { formatAmount } from "@/utils/format";

/**
 * Spending by category, as a ring.
 *
 * A solid pie with labels on the slices was the wrong chart here: with six or
 * more categories the labels collide, and the legend already names every one of
 * them — so the labels repeated the legend while obscuring the pie. The ring
 * keeps a hole for the eye to rest in, the labels are turned off, and the hole
 * is where the total goes — the one number every slice divides into.
 *
 * Colours come from the theme's eight-hue series rather than a hardcoded
 * "dashboard palette", so the chart belongs to the same system as everything
 * around it and adapts when the theme does.
 */
export function CategoryBreakdownChart() {
  const { t } = useTranslation();
  const colors = useChartColors();
  const { data, isLoading, error } = useCategoryBreakdown();

  const chartData = useMemo(
    () =>
      data?.categories.map((cat) => ({
        name: cat.category_name,
        value: cat.amount / 100,
        percentage: cat.percentage,
      })) ?? [],
    [data],
  );

  const total = data?.total_amount ?? 0;

  const option = useMemo<EChartsCoreOption>(
    () => ({
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      color: colors.series,
      title: {
        text: formatAmount(total),
        subtext: t("dashboard.categoryTotal"),
        left: "center",
        top: "30%",
        textAlign: "center",
        textStyle: {
          color: colors.text,
          fontSize: 15,
          fontWeight: 500,
          fontFamily: colors.mono,
        },
        subtextStyle: {
          color: colors.neutral,
          fontSize: 11,
          fontFamily: colors.sans,
        },
        itemGap: 4,
      },
      legend: {
        type: "scroll",
        bottom: 0,
        left: "center",
        itemWidth: 8,
        itemHeight: 8,
        itemGap: 12,
        icon: "roundRect",
        textStyle: {
          color: colors.neutral,
          fontSize: 11,
          fontFamily: colors.sans,
        },
        pageIconColor: colors.neutral,
        pageIconInactiveColor: colors.grid,
        pageTextStyle: { color: colors.neutral, fontSize: 10 },
      },
      tooltip: chartTooltip(colors, {
        trigger: "item",
        formatter: (raw) => {
          const p = raw as {
            marker: string;
            name: string;
            value: number;
            percent: number;
          };
          return `${p.marker}${p.name}<br/>${formatAmount(Math.round(p.value * 100))}：${p.percent.toFixed(1)}%`;
        },
      }),
      series: [
        {
          type: "pie",
          radius: ["52%", "76%"],
          center: ["50%", "42%"],
          avoidLabelOverlap: true,
          padAngle: 1,
          itemStyle: { borderWidth: 0, borderRadius: 2 },
          label: { show: false },
          labelLine: { show: false },
          emphasis: { scale: true, scaleSize: 4 },
          data: chartData,
        },
      ],
    }),
    [chartData, colors, t, total],
  );

  const hasData = chartData.length > 0;

  return (
    <Card elevation="raise" className="flex h-[300px] shrink-0 flex-col">
      <CardHeader>
        <CardTitle>{t("dashboard.categoryBreakdown")}</CardTitle>
      </CardHeader>
      <CardContent className="min-h-0 flex-1">
        {error ? (
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        ) : isLoading ? (
          <div className="flex h-full min-h-[180px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.loading")}</p>
          </div>
        ) : !hasData ? (
          <div className="flex h-full min-h-[180px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.noRecords")}</p>
          </div>
        ) : (
          <EChart option={option} label={t("dashboard.categoryBreakdown")} />
        )}
      </CardContent>
    </Card>
  );
}
