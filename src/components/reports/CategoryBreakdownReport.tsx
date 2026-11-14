import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useCategoryBreakdownReport } from "@/hooks/useCategoryBreakdownReport";
import { EChart } from "@/components/charts/EChart";
import type { EChartsCoreOption } from "@/components/charts/echarts";
import { useChartColors } from "@/utils/chartColors";
import {
  chartAnimation,
  chartTextStyle,
  chartTooltip,
} from "@/utils/chartTheme";
import { formatAmount } from "@/utils/format";
import type { CategoryBreakdownItem } from "@/types/report";
import {
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

/**
 * Where the money went, by category.
 *
 * The period comes from the page filter, not from a second date picker inside
 * the report — that duplication was the reason two reports could be showing
 * two different ranges at the same time. The only control here is the one that
 * is genuinely report-specific: whether to look at spending or at income.
 *
 * The ring carries no slice labels. With six or more categories they collide,
 * and the table below already names every one with its amount and share, so the
 * labels would repeat the table while covering the chart.
 */
export function CategoryBreakdownReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t } = useTranslation();
  const colors = useChartColors();
  const [kind, setKind] = useState<"expense" | "income">("expense");

  const { data, isLoading, error } = useCategoryBreakdownReport(
    controller.filter,
    kind,
  );

  const categories = useMemo<CategoryBreakdownItem[]>(
    () => data?.categories ?? [],
    [data],
  );

  const option = useMemo<EChartsCoreOption>(
    () => ({
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      color: colors.series,
      legend: {
        type: "scroll",
        bottom: 0,
        left: "center",
        itemWidth: 8,
        itemHeight: 8,
        itemGap: 14,
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
          // A ring, not a disc: the hole gives the eye a place to rest and
          // stops a wide slice from reading as the whole chart.
          radius: ["46%", "72%"],
          center: ["50%", "44%"],
          padAngle: 1,
          itemStyle: { borderWidth: 0, borderRadius: 2 },
          label: { show: false },
          labelLine: { show: false },
          emphasis: { scale: true, scaleSize: 5 },
          data: categories.map((category) => ({
            name: category.categoryName,
            value: category.amount / 100,
          })),
        },
      ],
    }),
    [categories, colors],
  );

  const total = data?.totalAmount ?? 0;
  const largest = categories[0];

  const columns: ReportColumn<CategoryBreakdownItem>[] = [
    {
      header: t("reports.tableHeaders.category"),
      cell: (row) => row.categoryName,
    },
    {
      header: t("reports.tableHeaders.amount"),
      align: "right",
      cell: (row) => formatAmount(row.amount),
    },
    {
      header: t("reports.tableHeaders.percentage"),
      align: "right",
      cell: (row) => (
        <span className="inline-flex items-center justify-end gap-2">
          {/* A share bar in the cell: four rows of "31.2%" are hard to rank by
              eye, one bar tells you which is biggest before you read it. */}
          <span
            aria-hidden
            className="h-1.5 rounded-full bg-info/60"
            style={{ width: `${Math.max(2, row.percentage)}%`, maxWidth: 64 }}
          />
          <span className="w-11 text-right text-muted-foreground">
            {row.percentage.toFixed(1)}%
          </span>
        </span>
      ),
    },
    {
      header: t("reports.tableHeaders.transactions"),
      align: "right",
      cell: (row) => (
        <span className="text-muted-foreground">{row.transactionCount}</span>
      ),
    },
  ];

  return (
    <ReportFrame
      title={t("reports.names.categories")}
      scope={t("reports.scope.categories", {
        start: controller.range.startDate,
        end: controller.range.endDate,
      })}
      isLoading={isLoading}
      error={error}
      isEmpty={categories.length === 0}
      emptyHint={t("reports.categoryBreakdown.noCategoryData")}
      toolbar={
        <div className="flex items-center gap-1 rounded-xl border bg-card px-2 py-1.5">
          {(["expense", "income"] as const).map((value) => (
            <Button
              key={value}
              variant={kind === value ? "secondary" : "ghost"}
              size="sm"
              onClick={() => setKind(value)}
              className="h-6 px-2.5 text-xs"
            >
              {t(
                value === "expense"
                  ? "categories.typeExpense"
                  : "categories.typeIncome",
              )}
            </Button>
          ))}
        </div>
      }
    >
      <ReportStats
        items={[
          {
            label:
              kind === "expense"
                ? t("reports.categoryBreakdown.expenseTotal")
                : t("reports.categoryBreakdown.incomeTotal"),
            value: formatAmount(total),
            tone: kind === "expense" ? "expense" : "income",
          },
          {
            label: t("reports.categoryBreakdown.categoriesCount"),
            value: String(categories.length),
          },
          {
            label: t("reports.categoryBreakdown.topShare"),
            value: largest ? `${largest.percentage.toFixed(1)}%` : "—",
            hint: largest?.categoryName,
          },
          {
            label: t("reports.categoryBreakdown.averageShare"),
            value:
              categories.length > 0
                ? `${(100 / categories.length).toFixed(1)}%`
                : "—",
            hint: t("reports.categoryBreakdown.averageShareHint"),
          },
        ]}
      />

      <ReportPanel title={t("reports.categoryBreakdown.categoryDistribution")}>
        <EChart
          option={option}
          height={300}
          label={t("reports.categoryBreakdown.categoryDistribution")}
        />
      </ReportPanel>

      <ReportPanel title={t("reports.categoryBreakdown.categoryDetails")}>
        <ReportTable
          columns={columns}
          rows={categories}
          rowKey={(row) => row.categoryId}
        />
      </ReportPanel>
    </ReportFrame>
  );
}
