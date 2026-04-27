import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  ResponsiveContainer,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
} from "recharts";
import { ReportFilterBar } from "./ReportFilterBar";
import { useStandardReport } from "@/hooks/useStandardReport";
import { formatAmount } from "@/utils/format";
import { getDefaultRangePresetDates } from "@/utils/date";
import { REPORT_CHART_COLORS } from "@/constants/report";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import type { DateRangePreset, ReportFilter } from "@/types/report";

export function StandardReport() {
  const { t } = useTranslation();

  // 篩选状态
  const [dateRangePreset, setDateRangePreset] =
    useState<DateRangePreset>("currentMonth");
  const [startDate, setStartDate] = useState<string | undefined>(undefined);
  const [endDate, setEndDate] = useState<string | undefined>(undefined);

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
    periodGranularity: "monthly",
    accountIds: undefined,
    categoryIds: undefined,
  };

  const { data, isLoading, error } = useStandardReport(filter);

  if (error) {
    return (
      <div className="space-y-4">
        <ReportFilterBar
          filter={filter}
          setDateRangePreset={setDateRangePreset}
          setStartDate={setStartDate}
          setEndDate={setEndDate}
        />
        <div className="text-center py-8 text-destructive">
          {t("reports.common.errorWithMessage", { message: error.message })}
        </div>
      </div>
    );
  }

  if (!data && isLoading) {
    return (
      <div className="space-y-4">
        <ReportFilterBar
          filter={filter}
          setDateRangePreset={setDateRangePreset}
          setStartDate={setStartDate}
          setEndDate={setEndDate}
        />
        <div className="grid grid-cols-2 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-muted-foreground">
                {t("reports.standard.periodIncome")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">--</div>
              <div className="text-sm mt-1 text-muted-foreground">
                {t("reports.common.loading")}
              </div>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-muted-foreground">
                {t("reports.standard.periodExpense")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">--</div>
              <div className="text-sm mt-1 text-muted-foreground">
                {t("reports.common.loading")}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="space-y-4">
        <ReportFilterBar
          filter={filter}
          setDateRangePreset={setDateRangePreset}
          setStartDate={setStartDate}
          setEndDate={setEndDate}
        />
        <div className="text-center py-8">{t("reports.common.noData")}</div>
      </div>
    );
  }

  const incomeChangeIcon = data.incomeChangePct >= 0 ? "↑" : "↓";
  const expenseChangeIcon = data.expenseChangePct >= 0 ? "↑" : "↓";
  const incomeChangeColor =
    data.incomeChangePct >= 0 ? "text-green-600" : "text-red-600";
  const expenseChangeColor =
    data.expenseChangePct >= 0 ? "text-red-600" : "text-green-600";

  return (
    <div className="space-y-4">
      <ReportFilterBar
        filter={filter}
        setDateRangePreset={setDateRangePreset}
        setStartDate={setStartDate}
        setEndDate={setEndDate}
      />

      <div className="grid grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.standard.periodIncome")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {formatAmount(data.periodIncome)}
            </div>
            <div className={`text-sm mt-1 ${incomeChangeColor}`}>
              {t("reports.common.vsPrevious")}: {incomeChangeIcon}{" "}
              {Math.abs(data.incomeChangePct).toFixed(1)}%
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.standard.periodExpense")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">
              {formatAmount(data.periodExpense)}
            </div>
            <div className={`text-sm mt-1 ${expenseChangeColor}`}>
              {t("reports.common.vsPrevious")}: {expenseChangeIcon}{" "}
              {Math.abs(data.expenseChangePct).toFixed(1)}%
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.standard.netWorthTrend")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ResponsiveContainer width="100%" height={200}>
            <AreaChart data={data.netWorthTrend}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="date" />
              <YAxis />
              <Tooltip
                formatter={(v) =>
                  typeof v === "number" ? formatAmount(v) : "--"
                }
                labelFormatter={(l) =>
                  t("reports.common.tooltipDate", { date: l })
                }
              />
              <Area
                type="monotone"
                dataKey="netWorth"
                stroke={REPORT_CHART_COLORS.net}
                fill={REPORT_CHART_COLORS.net}
                fillOpacity={0.3}
              />
            </AreaChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.standard.accountChanges")}</CardTitle>
        </CardHeader>
        <CardContent>
          <table className="w-full text-sm">
            <thead className="bg-muted">
              <tr>
                <th className="px-4 py-2 text-left">
                  {t("reports.tableHeaders.account")}
                </th>
                <th className="px-4 py-2 text-right">
                  {t("reports.tableHeaders.startBalance")}
                </th>
                <th className="px-4 py-2 text-right">
                  {t("reports.tableHeaders.endBalance")}
                </th>
                <th className="px-4 py-2 text-right">
                  {t("reports.tableHeaders.change")}
                </th>
              </tr>
            </thead>
            <tbody>
              {data.accountChanges.map((change) => (
                <tr key={change.accountId}>
                  <td className="px-4 py-2">{change.accountName}</td>
                  <td className="px-4 py-2 text-right">
                    {formatAmount(change.startBalance)}
                  </td>
                  <td className="px-4 py-2 text-right">
                    {formatAmount(change.endBalance)}
                  </td>
                  <td
                    className={`px-4 py-2 text-right ${
                      change.change >= 0 ? "text-green-600" : "text-red-600"
                    }`}
                  >
                    {formatAmount(change.change)}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </div>
  );
}
