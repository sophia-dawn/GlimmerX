import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ReportQueryController } from "@/hooks/useReportQuery";
import { useAccountList } from "@/hooks/useAccountList";
import { useAccountTransactionsReport } from "@/hooks/useAccountTransactionsReport";
import { useAccountBalanceTrendReport } from "@/hooks/useAccountBalanceTrendReport";
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
import type { AccountTransactionItem, BalancePoint } from "@/types/report";
import { ChartScaleSelect } from "./ChartScaleSelect";
import {
  ReportFrame,
  ReportPanel,
  ReportStats,
  ReportTable,
  type ReportColumn,
} from "./ReportFrame";

const PAGE_SIZE = 50;

/**
 * One account, in detail: what it did over time and every line that did it.
 *
 * Two reports used to cover this — a transaction list and a balance trend —
 * each with its own account picker, so comparing them meant picking the same
 * account twice and hoping the range matched. They are one screen now: the
 * curve on top, the rows underneath, the same account and the same period
 * driving both. The account is in the URL, so "look at this card" is a link.
 */
export function AccountDetailReport({
  controller,
}: {
  controller: ReportQueryController;
}) {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
  const { data: accounts, isLoading: accountsLoading } = useAccountList();
  const [page, setPage] = useState(1);

  const accountId = controller.query.accountId;

  const balanceAccounts = useMemo(
    () =>
      (accounts ?? []).filter(
        (account) =>
          account.account_type === "asset" ||
          account.account_type === "liability",
      ),
    [accounts],
  );

  // A new period starts at the first page; page 7 of the old range is not a
  // page of the new one.
  useEffect(() => {
    setPage(1);
  }, [accountId, controller.filter.startDate, controller.filter.endDate]);

  const transactions = useAccountTransactionsReport(
    accountId,
    controller.filter,
    page,
    PAGE_SIZE,
  );
  const trend = useAccountBalanceTrendReport(accountId, controller.filter);

  const points = useMemo<BalancePoint[]>(
    () => trend.data?.dataPoints ?? [],
    [trend.data],
  );

  const option = useMemo<EChartsCoreOption>(() => {
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
        data: points.map((point) => point.period),
      },
      yAxis: valueAxis(colors, yuanTick(i18n.language)),
      series: [
        {
          type: "line",
          smooth: 0.2,
          showSymbol: false,
          data: points.map((point) => point.balance / 100),
          lineStyle: { width: 1.5, color: colors.info },
          itemStyle: { color: colors.info },
          areaStyle: { color: colors.info, opacity: 0.12 },
        },
      ],
    };
  }, [points, colors, i18n.language]);

  const columns: ReportColumn<AccountTransactionItem>[] = [
    {
      header: t("reports.tableHeaders.date"),
      className: "font-mono text-muted-foreground",
      cell: (row) => row.date,
    },
    {
      header: t("reports.tableHeaders.description"),
      cell: (row) => row.description || "—",
    },
    {
      header: t("reports.tableHeaders.category"),
      cell: (row) => row.categoryName ?? "—",
    },
    {
      header: t("reports.accounts.counterAccount"),
      cell: (row) => row.counterAccountName ?? "—",
    },
    {
      header: t("reports.tableHeaders.amount"),
      align: "right",
      cell: (row) => (
        <span className={row.isInflow ? "text-income" : "text-expense"}>
          {row.isInflow ? "+" : "−"}
          {formatAmount(row.amount)}
        </span>
      ),
    },
  ];

  const report = transactions.data;
  const totalPages = report?.totalPages ?? 0;

  return (
    <ReportFrame
      title={t("reports.names.accounts")}
      scope={t("reports.scope.accounts", {
        start: controller.range.startDate,
        end: controller.range.endDate,
      })}
      isLoading={
        accountsLoading || (accountId !== "" && transactions.isLoading)
      }
      error={transactions.error ?? trend.error ?? null}
      isEmpty={accountId === ""}
      emptyHint={t("reports.accounts.pickAccount")}
      toolbar={
        <div className="flex flex-wrap items-center gap-2 rounded-xl border bg-card px-4 py-2.5">
          <span className="text-xs text-muted-foreground">
            {t("reports.accounts.selectAccount")}
          </span>
          <Select value={accountId} onValueChange={controller.setAccount}>
            <SelectTrigger size="sm" className="w-[220px]">
              <SelectValue placeholder={t("reports.accounts.selectAccount")} />
            </SelectTrigger>
            <SelectContent>
              {balanceAccounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  {account.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      }
    >
      <ReportStats
        items={[
          {
            label: t("reports.accounts.inflow"),
            value: formatAmount(report?.totalInflow ?? 0),
            tone: "income",
            hint: t("reports.accounts.inflowHint"),
          },
          {
            label: t("reports.accounts.outflow"),
            value: formatAmount(report?.totalOutflow ?? 0),
            tone: "expense",
            hint: t("reports.accounts.outflowHint"),
          },
          {
            label: t("reports.accounts.netChange"),
            value: formatAmount(report?.netChange ?? 0),
            tone: (report?.netChange ?? 0) >= 0 ? "income" : "expense",
            hint: t("reports.accounts.netChangeHint"),
          },
          {
            label: t("reports.accounts.postings"),
            value: String(report?.totalCount ?? 0),
            hint: t("reports.accounts.postingsHint"),
          },
        ]}
      />

      <ReportPanel
        title={t("reports.accounts.balanceTrend")}
        hint={trend.data ? trend.data.accountName : undefined}
        action={
          <ChartScaleSelect
            value={controller.granularity}
            options={controller.granularityOptions}
            onChange={controller.setGranularity}
          />
        }
      >
        {points.length > 0 ? (
          <EChart
            option={option}
            height={200}
            label={t("reports.accounts.balanceTrend")}
          />
        ) : (
          <p className="py-10 text-center text-sm text-muted-foreground">
            {t("reports.common.noData")}
          </p>
        )}
      </ReportPanel>

      <ReportPanel
        title={t("reports.accounts.transactions")}
        action={
          totalPages > 1 ? (
            <div className="flex items-center gap-1">
              <Button
                variant="ghost"
                size="sm"
                disabled={page <= 1}
                onClick={() => setPage((current) => Math.max(1, current - 1))}
              >
                <ChevronLeft className="h-3.5 w-3.5" />
              </Button>
              <span className="font-mono text-xs text-muted-foreground">
                {page} / {totalPages}
              </span>
              <Button
                variant="ghost"
                size="sm"
                disabled={page >= totalPages}
                onClick={() => setPage((current) => current + 1)}
              >
                <ChevronRight className="h-3.5 w-3.5" />
              </Button>
            </div>
          ) : undefined
        }
      >
        {report && report.transactions.length > 0 ? (
          <ReportTable
            columns={columns}
            rows={report.transactions}
            rowKey={(row) => row.transactionId}
          />
        ) : (
          <p className="py-10 text-center text-sm text-muted-foreground">
            {t("reports.common.noData")}
          </p>
        )}
      </ReportPanel>
    </ReportFrame>
  );
}
