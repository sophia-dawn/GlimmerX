import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useStandardReport } from "@/hooks/useStandardReport";
import { useTrendReport } from "@/hooks/useTrendReport";
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
  valueAxis,
  yuanTick,
} from "@/utils/chartTheme";
import { formatAmount, formatAmountWithSign } from "@/utils/format";
import {
  Amount,
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

/**
 * Cash flow: what came in, what went out, and the shape of it over time.
 *
 * This replaces four reports — standard, trend, month comparison, year summary —
 * that all answered "how did money move in a period" with different framing:
 * one had the totals, one had the series, one compared two months picked by
 * hand, one fixed the period to a year. A user who wanted to know whether
 * October was worse than September needed two of them and a mental subtraction.
 * Here the series is the period already chosen in the filter bar, the previous
 * period is fetched alongside it, and the comparison is one row.
 *
 * The series is drawn as paired bars with a net line rather than three lines:
 * income and expense are magnitudes that start at zero and need to be read as
 * heights, while the net is a signed value that can cross the axis and needs to
 * be read as a position.
 */
interface ComparisonRow {
  key: "income" | "expense" | "net";
  current: number;
  previous: number;
}

function signedPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}%`;
}

function percentChange(current: number, previous: number): number | null {
  if (previous === 0) return current === 0 ? null : 100;
  return ((current - previous) / Math.abs(previous)) * 100;
}

function periodLabel(
  period: string,
  granularity: string,
  monthName: (month: number) => string,
): string {
  switch (granularity) {
    case "daily":
      return period.slice(5);
    case "weekly":
      return period.slice(2).replace("-", " ");
    case "monthly": {
      const month = Number(period.slice(5, 7));
      const year = period.slice(2, 4);
      return `${year} ${monthName(month)}`;
    }
    default:
      return period;
  }
}

export function CashflowReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const standard = useStandardReport(controller.filter);
  const trend = useTrendReport(controller.filter);

  const monthName = (month: number) => t(`reports.monthNames.${month}`);

  const points = trend.data?.dataPoints ?? [];
  const income = standard.data?.periodIncome ?? 0;
  const expense = standard.data?.periodExpense ?? 0;
  const net = income - expense;
  const previousNet =
    (standard.data?.prevIncome ?? 0) - (standard.data?.prevExpense ?? 0);
  const savingsRate = income > 0 ? (net / income) * 100 : null;

  const rows = useMemo<ComparisonRow[]>(() => {
    if (!standard.data) return [];
    return [
      {
        key: "income",
        current: standard.data.periodIncome,
        previous: standard.data.prevIncome,
      },
      {
        key: "expense",
        current: standard.data.periodExpense,
        previous: standard.data.prevExpense,
      },
      {
        key: "net",
        current: standard.data.periodIncome - standard.data.periodExpense,
        previous: standard.data.prevIncome - standard.data.prevExpense,
      },
    ];
  }, [standard.data]);

  const option = useMemo<EChartsCoreOption>(() => {
    const labels = points.map((point) =>
      periodLabel(
        point.period,
        trend.data?.granularity ?? "monthly",
        monthName,
      ),
    );
    const money = (yuan: number) => formatAmount(Math.round(yuan * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      color: [colors.income, colors.expense, colors.info],
      grid: chartGrid({ top: 32 }),
      legend: {
        ...chartLegend(colors),
        data: [
          t("reports.cashflow.incomeSeries"),
          t("reports.cashflow.expenseSeries"),
          t("reports.cashflow.netSeries"),
        ],
      },
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const params = raw as {
            marker: string;
            seriesName: string;
            value: number;
            axisValue: string;
          }[];
          if (!params.length) return "";
          const body = params
            .map((p) => `${p.marker}${p.seriesName}：${money(Number(p.value))}`)
            .join("<br/>");
          return `${params[0]?.axisValue ?? ""}<br/>${body}`;
        },
      }),
      xAxis: { ...categoryAxis(colors), data: labels },
      yAxis: valueAxis(colors, yuanTick(i18n.language)),
      series: [
        {
          type: "bar",
          name: t("reports.cashflow.incomeSeries"),
          data: points.map((point) => point.income / 100),
          barMaxWidth: 16,
          itemStyle: { color: colors.income, borderRadius: [2, 2, 0, 0] },
        },
        {
          type: "bar",
          name: t("reports.cashflow.expenseSeries"),
          data: points.map((point) => point.expense / 100),
          barMaxWidth: 16,
          itemStyle: { color: colors.expense, borderRadius: [2, 2, 0, 0] },
        },
        {
          type: "line",
          name: t("reports.cashflow.netSeries"),
          data: points.map((point) => point.net / 100),
          smooth: 0.2,
          showSymbol: false,
          z: 4,
          lineStyle: { width: 1.5, color: colors.text },
          itemStyle: { color: colors.text },
        },
      ],
    };
    // `monthName` closes over `t`; both change together on a language switch.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [points, colors, i18n.language, t, trend.data?.granularity]);

  const columns: ReportColumn<ComparisonRow>[] = [
    {
      header: t("reports.cashflow.metric"),
      cell: (row) => t(`reports.cashflow.metrics.${row.key}`),
    },
    {
      header: t("reports.cashflow.current"),
      align: "right",
      cell: (row) => <Amount value={row.current} tone="plain" />,
    },
    {
      header: t("reports.cashflow.previous"),
      align: "right",
      cell: (row) => <Amount value={row.previous} tone="plain" />,
    },
    {
      header: t("reports.cashflow.diff"),
      align: "right",
      cell: (row) => {
        const diff = row.current - row.previous;
        return <Amount value={diff} tone={diff >= 0 ? "income" : "expense"} />;
      },
    },
    {
      header: t("reports.tableHeaders.changeRate"),
      align: "right",
      cell: (row) => {
        const pct = percentChange(row.current, row.previous);
        if (pct === null)
          return <span className="text-muted-foreground">—</span>;
        return (
          <span className={pct >= 0 ? "text-income" : "text-expense"}>
            {pct >= 0 ? "+" : "−"}
            {Math.abs(pct).toFixed(1)}%
          </span>
        );
      },
    },
  ];

  return (
    <ReportFrame
      title={t("reports.names.cashflow")}
      scope={t("reports.scope.cashflow", {
        start: controller.range.startDate,
        end: controller.range.endDate,
      })}
      isLoading={standard.isLoading || trend.isLoading}
      error={standard.error ?? trend.error ?? null}
      isEmpty={points.length === 0}
    >
      <ReportStats
        items={[
          {
            label: t("reports.cashflow.stats.income"),
            value: formatAmount(income),
            tone: "income",
            hint: t("reports.cashflow.vsPrevious", {
              value: signedPercent(standard.data?.incomeChangePct ?? 0),
            }),
          },
          {
            label: t("reports.cashflow.stats.expense"),
            value: formatAmount(expense),
            tone: "expense",
            hint: t("reports.cashflow.vsPrevious", {
              value: signedPercent(standard.data?.expenseChangePct ?? 0),
            }),
          },
          {
            label: t("reports.cashflow.stats.net"),
            value: formatAmount(net),
            tone: net >= 0 ? "income" : "expense",
            hint: t("reports.cashflow.vsPrevious", {
              value: formatAmountWithSign(net - previousNet),
            }),
          },
          {
            label: t("reports.cashflow.stats.savingsRate"),
            value: savingsRate === null ? "—" : `${savingsRate.toFixed(1)}%`,
            tone: (savingsRate ?? 0) >= 0 ? "plain" : "expense",
            hint: t("reports.cashflow.stats.savingsRateHint"),
          },
        ]}
      />

      <ReportPanel title={t("reports.cashflow.chart")}>
        <EChart
          option={option}
          height={260}
          label={t("reports.cashflow.chart")}
        />
      </ReportPanel>

      <ReportPanel title={t("reports.cashflow.comparison")}>
        <ReportTable columns={columns} rows={rows} rowKey={(row) => row.key} />
      </ReportPanel>
    </ReportFrame>
  );
}
