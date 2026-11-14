import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useStandardReport } from "@/hooks/useStandardReport";
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
import type { AccountChange } from "@/types/report";
import {
  Amount,
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

/**
 * Net worth: where it was, where it went, and which account moved it.
 *
 * This is the report double-entry makes possible and a spreadsheet does not.
 * Net worth change over a period equals the sum of every balance-sheet
 * account's change — exactly, because a posting that leaves one asset and
 * enters another appears on both sides with opposite signs and cancels, while
 * one that enters from income has no counterpart and stays. So "净资产的变动"
 * can be split by account with no residual term and no estimating: the parts
 * add up to the whole by construction, not by luck.
 *
 * `accountChanges` already carries both boundaries (balance before the period,
 * balance through the end), so the opening and closing net worth are summed
 * from the same rows that produce the attribution — one query, one truth,
 * nothing that can disagree with the chart beside it.
 */
function accountTypeLabel(
  type: string,
  translate: (key: string) => string,
): string {
  const key = `accounts.types.${type}`;
  const translated = translate(key);
  return translated === key ? type : translated;
}

export function NetWorthAttributionReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const { data, isLoading, error } = useStandardReport(controller.filter);

  const changes = useMemo(() => data?.accountChanges ?? [], [data]);
  const trend = useMemo(() => data?.netWorthTrend ?? [], [data]);

  const opening = changes.reduce((sum, row) => sum + row.startBalance, 0);
  const closing = changes.reduce((sum, row) => sum + row.endBalance, 0);
  const delta = closing - opening;
  const deltaPct = opening === 0 ? null : (delta / Math.abs(opening)) * 100;

  /** Largest movement first — the chart is a ranking, not a ledger. */
  const ranked = useMemo(
    () =>
      [...changes]
        .filter((row) => row.change !== 0)
        .sort((a, b) => Math.abs(b.change) - Math.abs(a.change)),
    [changes],
  );

  const option = useMemo<EChartsCoreOption>(() => {
    // ECharts stacks categories bottom-up; reversing puts the largest at the top.
    const ordered = [...ranked].reverse();
    const money = (yuan: number) => formatAmount(Math.round(yuan * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      grid: chartGrid({ left: 4, right: 24, top: 8 }),
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const p = (
            raw as { name: string; value: number; marker: string }[]
          )[0];
          if (!p) return "";
          return `${p.name}<br/>${p.marker}${money(Number(p.value))}`;
        },
      }),
      xAxis: valueAxis(colors, yuanTick(i18n.language)),
      yAxis: {
        ...categoryAxis(colors),
        data: ordered.map((row) => row.accountName),
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
          barMaxWidth: 14,
          data: ordered.map((row) => ({
            value: row.change / 100,
            itemStyle: {
              color: row.change >= 0 ? colors.income : colors.expense,
              borderRadius: 2,
            },
          })),
          label: {
            show: true,
            position: "right",
            fontSize: 11,
            fontFamily: colors.mono,
            color: colors.neutral,
            formatter: (p: { value: number }) =>
              formatAmount(Math.round(Number(p.value) * 100)),
          },
        },
      ],
    };
  }, [ranked, colors, i18n.language]);

  const netWorthOption = useMemo<EChartsCoreOption>(() => {
    const money = (yuan: number) => formatAmount(Math.round(yuan * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      grid: chartGrid({ top: 12 }),
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const p = (
            raw as { axisValue: string; marker: string; value: number }[]
          )[0];
          if (!p) return "";
          return `${p.axisValue}<br/>${p.marker}${money(Number(p.value))}`;
        },
      }),
      xAxis: {
        ...categoryAxis(colors),
        data: trend.map((point) => point.date.slice(5)),
      },
      yAxis: valueAxis(colors, yuanTick(i18n.language)),
      series: [
        {
          type: "line",
          smooth: 0.2,
          showSymbol: false,
          data: trend.map((point) => point.netWorth / 100),
          lineStyle: { width: 1.5, color: colors.info },
          itemStyle: { color: colors.info },
          areaStyle: { color: colors.info, opacity: 0.12 },
        },
      ],
    };
  }, [trend, colors, i18n.language]);

  const columns: ReportColumn<AccountChange>[] = [
    {
      header: t("reports.tableHeaders.accountName"),
      cell: (row) => row.accountName,
    },
    {
      header: t("reports.tableHeaders.accountType"),
      cell: (row) => accountTypeLabel(row.accountType, t),
    },
    {
      header: t("reports.tableHeaders.startBalance"),
      align: "right",
      cell: (row) => <Amount value={row.startBalance} tone="plain" />,
    },
    {
      header: t("reports.tableHeaders.endBalance"),
      align: "right",
      cell: (row) => <Amount value={row.endBalance} tone="plain" />,
    },
    {
      header: t("reports.tableHeaders.change"),
      align: "right",
      cell: (row) => (
        <Amount
          value={row.change}
          tone={row.change >= 0 ? "income" : "expense"}
        />
      ),
    },
    {
      header: t("reports.netWorth.share"),
      align: "right",
      cell: (row) => {
        if (delta === 0)
          return <span className="text-muted-foreground">—</span>;
        const share = (row.change / delta) * 100;
        return (
          <span className="text-muted-foreground">{share.toFixed(1)}%</span>
        );
      },
    },
  ];

  return (
    <ReportFrame
      title={t("reports.names.netWorth")}
      scope={t("reports.scope.netWorth", {
        start: controller.range.startDate,
        end: controller.range.endDate,
      })}
      isLoading={isLoading}
      error={error}
      isEmpty={changes.length === 0}
    >
      <ReportStats
        items={[
          {
            label: t("reports.netWorth.opening"),
            value: formatAmount(opening),
            hint: controller.range.startDate,
          },
          {
            label: t("reports.netWorth.closing"),
            value: formatAmount(closing),
            hint: controller.range.endDate,
          },
          {
            label: t("reports.netWorth.delta"),
            value: formatAmount(delta),
            tone: delta >= 0 ? "income" : "expense",
            hint: t("reports.netWorth.deltaHint"),
          },
          {
            label: t("reports.netWorth.deltaPct"),
            value: deltaPct === null ? "—" : `${deltaPct.toFixed(1)}%`,
            tone: (deltaPct ?? 0) >= 0 ? "plain" : "expense",
            hint: t("reports.netWorth.deltaPctHint"),
          },
        ]}
      />

      <ReportPanel
        title={t("reports.netWorth.trend")}
        hint={t("reports.netWorth.trendHint")}
      >
        {trend.length > 0 ? (
          <EChart
            option={netWorthOption}
            height={200}
            label={t("reports.netWorth.trend")}
          />
        ) : (
          <p className="py-10 text-center text-sm text-muted-foreground">
            {t("reports.common.noData")}
          </p>
        )}
      </ReportPanel>

      <ReportPanel
        title={t("reports.netWorth.byAccount")}
        hint={t("reports.netWorth.byAccountHint")}
      >
        {ranked.length > 0 ? (
          <EChart
            option={option}
            height={Math.max(140, ranked.length * 26 + 24)}
            label={t("reports.netWorth.byAccount")}
          />
        ) : (
          <p className="py-10 text-center text-sm text-muted-foreground">
            {t("reports.netWorth.noMovement")}
          </p>
        )}
      </ReportPanel>

      <ReportPanel title={t("reports.netWorth.detail")}>
        <ReportTable
          columns={columns}
          rows={changes}
          rowKey={(row) => row.accountId}
        />
      </ReportPanel>
    </ReportFrame>
  );
}
