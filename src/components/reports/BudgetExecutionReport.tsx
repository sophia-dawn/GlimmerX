import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useBudgetStatuses } from "@/hooks/useBudgetStatuses";
import { EChart } from "@/components/charts/EChart";
import type { EChartsCoreOption } from "@/components/charts/echarts";
import { useChartColors } from "@/utils/chartColors";
import {
  categoryAxis,
  chartAnimation,
  chartGrid,
  chartTextStyle,
  chartTooltip,
  valueAxis,
  yuanTick,
} from "@/utils/chartTheme";
import { formatAmount } from "@/utils/format";
import type { BudgetStatus } from "@/types";
import {
  Amount,
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

/**
 * Budget execution: the plan against what actually happened.
 *
 * The bars are drawn as an overlay, not as a pair: the budget is a track and the
 * spend is a bar growing along it, so "how much of the plan is used" is the
 * length of the bar relative to the track rather than a subtraction the reader
 * has to do. Over-budget categories run past the track end and switch to the
 * signal colour — the one place the accent is allowed in a chart, because it
 * means "this one needs attention" and nothing else.
 *
 * One honesty note, stated in the scope line rather than buried: the budgets
 * table has no history. A past month is computed from *today's* budget
 * definition, so the spend is real but the target may be a later revision.
 */
export function BudgetExecutionReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const { data, isLoading, error } = useBudgetStatuses(
    controller.anchorYear,
    controller.anchorMonth,
  );

  const budgets = useMemo<BudgetStatus[]>(() => data ?? [], [data]);

  const totals = useMemo(
    () =>
      budgets.reduce(
        (acc, budget) => ({
          planned: acc.planned + budget.amount,
          spent: acc.spent + budget.spent,
          remaining: acc.remaining + budget.remaining,
          over: acc.over + (budget.overBudget ? 1 : 0),
        }),
        { planned: 0, spent: 0, remaining: 0, over: 0 },
      ),
    [budgets],
  );

  /** Worst first: the row that needs attention should not be at the bottom. */
  const ranked = useMemo(
    () =>
      [...budgets].sort((a, b) => {
        const rateA = a.amount > 0 ? a.spent / a.amount : 0;
        const rateB = b.amount > 0 ? b.spent / b.amount : 0;
        return rateB - rateA;
      }),
    [budgets],
  );

  const option = useMemo<EChartsCoreOption>(() => {
    const ordered = [...ranked].reverse();
    const money = (yuan: number) => formatAmount(Math.round(yuan * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      grid: chartGrid({ left: 4, right: 16, top: 8 }),
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const params = raw as {
            name: string;
            marker: string;
            seriesName: string;
            value: number;
          }[];
          if (!params.length) return "";
          const body = params
            .map((p) => `${p.marker}${p.seriesName}：${money(Number(p.value))}`)
            .join("<br/>");
          return `${params[0]?.name ?? ""}<br/>${body}`;
        },
      }),
      xAxis: valueAxis(colors, yuanTick(i18n.language)),
      yAxis: {
        ...categoryAxis(colors),
        data: ordered.map((budget) => budget.categoryName),
        axisLine: { show: false },
        axisLabel: {
          color: colors.neutral,
          fontSize: 11,
          fontFamily: colors.sans,
          width: 96,
          overflow: "truncate",
        },
      },
      series: [
        {
          type: "bar",
          name: t("reports.budgets.planned"),
          data: ordered.map((budget) => budget.amount / 100),
          barMaxWidth: 12,
          z: 1,
          itemStyle: {
            color: colors.neutral,
            opacity: 0.22,
            borderRadius: 2,
          },
        },
        {
          type: "bar",
          name: t("reports.budgets.spent"),
          data: ordered.map((budget) => ({
            value: budget.spent / 100,
            itemStyle: {
              color: budget.overBudget ? colors.signal : colors.info,
              borderRadius: 2,
            },
          })),
          barMaxWidth: 12,
          barGap: "-100%",
          z: 2,
        },
      ],
    };
  }, [ranked, colors, i18n.language, t]);

  const executionRate = (budget: BudgetStatus) =>
    budget.amount > 0 ? (budget.spent / budget.amount) * 100 : 0;

  const columns: ReportColumn<BudgetStatus>[] = [
    {
      header: t("reports.tableHeaders.categoryName"),
      cell: (row) => row.categoryName,
    },
    {
      header: t("reports.tableHeaders.budgetAmount"),
      align: "right",
      cell: (row) => <Amount value={row.amount} tone="plain" />,
    },
    {
      header: t("reports.tableHeaders.spent"),
      align: "right",
      cell: (row) => <Amount value={row.spent} tone="plain" />,
    },
    {
      header: t("reports.tableHeaders.remaining"),
      align: "right",
      cell: (row) => (
        <Amount
          value={row.remaining}
          tone={row.remaining >= 0 ? "income" : "expense"}
        />
      ),
    },
    {
      header: t("reports.tableHeaders.executionRate"),
      align: "right",
      cell: (row) => {
        const rate = executionRate(row);
        return (
          <span
            className={row.overBudget ? "text-signal" : "text-muted-foreground"}
          >
            {rate.toFixed(0)}%
          </span>
        );
      },
    },
    {
      header: t("reports.budgets.rollover"),
      align: "right",
      cell: (row) =>
        row.rollover ? (
          <span className="text-muted-foreground">
            {formatAmount(row.rolloverAmount)}
          </span>
        ) : (
          <span className="text-muted-foreground">—</span>
        ),
    },
  ];

  return (
    <ReportFrame
      title={t("reports.names.budgets")}
      scope={t("reports.scope.budgets", {
        month: controller.anchorMonthValue,
      })}
      isLoading={isLoading}
      error={error}
      isEmpty={budgets.length === 0}
      emptyHint={t("reports.budgets.noBudgets")}
    >
      <ReportStats
        items={[
          {
            label: t("reports.budgets.planned"),
            value: formatAmount(totals.planned),
            hint: t("reports.budgets.categories", { count: budgets.length }),
          },
          {
            label: t("reports.budgets.spent"),
            value: formatAmount(totals.spent),
            tone: "plain",
            hint:
              totals.planned > 0
                ? `${((totals.spent / totals.planned) * 100).toFixed(0)}%`
                : "—",
          },
          {
            label: t("reports.budgets.remaining"),
            value: formatAmount(totals.remaining),
            tone: totals.remaining >= 0 ? "income" : "expense",
            hint: t("reports.budgets.remainingHint"),
          },
          {
            label: t("reports.budgets.over"),
            value: String(totals.over),
            tone: totals.over > 0 ? "signal" : "plain",
            hint: t("reports.budgets.overHint"),
          },
        ]}
      />

      <ReportPanel
        title={t("reports.budgets.chart")}
        hint={t("reports.budgets.chartHint")}
      >
        <EChart
          option={option}
          height={Math.max(150, ranked.length * 26 + 24)}
          label={t("reports.budgets.chart")}
        />
      </ReportPanel>

      <ReportPanel title={t("reports.budgets.detail")}>
        <ReportTable columns={columns} rows={ranked} rowKey={(row) => row.id} />
      </ReportPanel>
    </ReportFrame>
  );
}
