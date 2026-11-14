import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useBalanceSheetReport } from "@/hooks/useBalanceSheetReport";
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
import type { BalanceSheetItem } from "@/types/report";
import {
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

/**
 * A balance sheet at one date: what is owned, what is owed, and the difference.
 *
 * The chart puts assets and liabilities on one axis instead of drawing two
 * bars side by side. Liabilities are stored negative — that is what makes
 * `net worth = assets + liabilities` true without a special case — so a single
 * sorted bar chart already shows the sign change at the axis, and the reader
 * sees immediately which side of zero the money is on rather than comparing two
 * totals that live on different halves of the page.
 *
 * The tables stay, because a balance sheet is a document people check line by
 * line, not only a shape.
 */
function accountTypeLabel(
  type: string,
  translate: (key: string) => string,
): string {
  const key = `accounts.types.${type}`;
  const translated = translate(key);
  return translated === key ? type : translated;
}

export function BalanceSheetReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const snapshotDate = controller.query.anchor;
  const { data, isLoading, error } = useBalanceSheetReport(snapshotDate);

  const assets = useMemo(() => data?.assets ?? [], [data]);
  const liabilities = useMemo(() => data?.liabilities ?? [], [data]);
  const totalAssets = data?.totalAssets ?? 0;
  const totalLiabilities = data?.totalLiabilities ?? 0;
  const netWorth = data?.netWorth ?? 0;
  const leverage =
    totalAssets > 0 ? (Math.abs(totalLiabilities) / totalAssets) * 100 : null;

  const rows = useMemo(
    () => [
      ...assets.map((item) => ({ ...item, side: "asset" as const })),
      ...liabilities.map((item) => ({ ...item, side: "liability" as const })),
    ],
    [assets, liabilities],
  );

  const option = useMemo<EChartsCoreOption>(() => {
    const ordered = [...rows].reverse();
    const money = (yuan: number) => formatAmount(Math.round(yuan * 100));
    return {
      ...chartAnimation(),
      textStyle: chartTextStyle(colors),
      grid: chartGrid({ left: 4, right: 20, top: 8 }),
      tooltip: chartTooltip(colors, {
        formatter: (raw) => {
          const p = (
            raw as { name: string; marker: string; value: number }[]
          )[0];
          if (!p) return "";
          return `${p.name}<br/>${p.marker}${money(Number(p.value))}`;
        },
      }),
      xAxis: valueAxis(colors, yuanTick(i18n.language)),
      yAxis: {
        ...categoryAxis(colors),
        data: ordered.map((item) => item.accountName),
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
          data: ordered.map((item) => ({
            value: item.balance / 100,
            itemStyle: {
              color: item.side === "asset" ? colors.info : colors.signal,
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
  }, [rows, colors, i18n.language]);

  const columns: ReportColumn<BalanceSheetItem>[] = [
    {
      header: t("reports.tableHeaders.accountName"),
      cell: (row) => row.accountName,
    },
    {
      header: t("reports.tableHeaders.accountType"),
      cell: (row) => accountTypeLabel(row.accountType, t),
    },
    {
      header: t("reports.tableHeaders.balance"),
      align: "right",
      cell: (row) => formatAmount(row.balance),
    },
  ];

  return (
    <ReportFrame
      title={t("reports.names.balanceSheet")}
      scope={t("reports.scope.balanceSheet", { date: snapshotDate })}
      isLoading={isLoading}
      error={error}
      isEmpty={assets.length === 0 && liabilities.length === 0}
      emptyHint={t("reports.balanceSheet.noAssetData")}
    >
      <ReportStats
        items={[
          {
            label: t("reports.balanceSheet.totalAssets"),
            value: formatAmount(totalAssets),
            tone: "plain",
            hint: t("reports.balanceSheet.assetTotal"),
          },
          {
            label: t("reports.balanceSheet.totalLiabilities"),
            value: formatAmount(totalLiabilities),
            tone: "expense",
            hint: t("reports.balanceSheet.liabilityTotal"),
          },
          {
            label: t("reports.balanceSheet.netWorth"),
            value: formatAmount(netWorth),
            tone: netWorth >= 0 ? "income" : "expense",
            hint: t("reports.balanceSheet.netWorthFormula"),
          },
          {
            label: t("reports.balanceSheet.leverage"),
            value: leverage === null ? "—" : `${leverage.toFixed(1)}%`,
            hint: t("reports.balanceSheet.leverageHint"),
          },
        ]}
      />

      <ReportPanel
        title={t("reports.balanceSheet.composition")}
        hint={t("reports.balanceSheet.compositionHint")}
      >
        {rows.length > 0 ? (
          <EChart
            option={option}
            height={Math.max(160, rows.length * 26 + 24)}
            label={t("reports.balanceSheet.composition")}
          />
        ) : (
          <p className="py-10 text-center text-sm text-muted-foreground">
            {t("reports.common.noData")}
          </p>
        )}
      </ReportPanel>

      <ReportPanel title={t("reports.balanceSheet.assetDetails")}>
        {assets.length > 0 ? (
          <ReportTable
            columns={columns}
            rows={assets}
            rowKey={(row) => row.accountId}
            footer={
              <tr>
                <td className="px-4 py-2 font-medium" colSpan={2}>
                  {t("reports.balanceSheet.assetTotal")}
                </td>
                <td className="px-4 py-2 text-right font-mono font-medium">
                  {formatAmount(totalAssets)}
                </td>
              </tr>
            }
          />
        ) : (
          <p className="py-8 text-center text-sm text-muted-foreground">
            {t("reports.balanceSheet.noAssetData")}
          </p>
        )}
      </ReportPanel>

      <ReportPanel title={t("reports.balanceSheet.liabilityDetails")}>
        {liabilities.length > 0 ? (
          <ReportTable
            columns={columns}
            rows={liabilities}
            rowKey={(row) => row.accountId}
            footer={
              <tr>
                <td className="px-4 py-2 font-medium" colSpan={2}>
                  {t("reports.balanceSheet.liabilityTotal")}
                </td>
                <td className="px-4 py-2 text-right font-mono font-medium">
                  {formatAmount(totalLiabilities)}
                </td>
              </tr>
            }
          />
        ) : (
          <p className="py-8 text-center text-sm text-muted-foreground">
            {t("reports.balanceSheet.noLiabilityData")}
          </p>
        )}
      </ReportPanel>
    </ReportFrame>
  );
}
