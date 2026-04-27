import { useState } from "react";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { MonthPicker } from "@/components/ui/month-picker";
import { useMonthComparisonReport } from "@/hooks/useMonthComparisonReport";
import { formatAmount, centsToYuan } from "@/utils/format";
import { REPORT_CHART_COLORS } from "@/constants/report";
import {
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip,
  CartesianGrid,
  Legend,
} from "recharts";
import { useTranslation } from "react-i18next";
import { useCurrency } from "@/hooks/useCurrency";
import { todayLocalDate } from "@/utils/date";

export function MonthComparisonReport() {
  const { t } = useTranslation();
  const { symbol } = useCurrency();

  const getMonthLabel = (value: string): string => {
    const [year, month] = value.split("-");
    return `${year} ${t(`reports.monthNames.${parseInt(month ?? "1", 10)}`)}`;
  };

  const today = todayLocalDate();
  const currentMonth = today.substring(0, 7);
  const lastMonthDate = new Date(
    parseInt(today.substring(0, 4)),
    parseInt(today.substring(5, 7)) - 2,
    1,
  );
  const lastMonth = `${lastMonthDate.getFullYear()}-${String(lastMonthDate.getMonth() + 1).padStart(2, "0")}`;

  const [month1, setMonth1] = useState(lastMonth);
  const [month2, setMonth2] = useState(currentMonth);

  const { data, isLoading, error } = useMonthComparisonReport({
    month1,
    month2,
    enabled: month1 !== month2,
  });

  if (error) {
    return (
      <div className="text-center py-8 text-destructive">
        {t("reports.common.errorWithMessage", { message: error.message })}
      </div>
    );
  }

  if (!data && isLoading) {
    return (
      <div className="space-y-4">
        <div className="flex gap-4">
          <div className="w-[160px] h-9 bg-muted rounded-md animate-pulse" />
          <div className="w-[160px] h-9 bg-muted rounded-md animate-pulse" />
        </div>
        <div className="grid grid-cols-2 gap-4">
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-muted-foreground">
                {t("reports.monthComparison.incomeComparison")}
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
                {t("reports.monthComparison.expenseComparison")}
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
      <div className="text-center py-8">
        {t("reports.monthComparison.selectDifferentMonths")}
      </div>
    );
  }

  const chartData = [
    {
      name: getMonthLabel(data.month1),
      income: centsToYuan(data.month1Income),
      expense: centsToYuan(Math.abs(data.month1Expense)),
    },
    {
      name: getMonthLabel(data.month2),
      income: centsToYuan(data.month2Income),
      expense: centsToYuan(Math.abs(data.month2Expense)),
    },
  ];

  const incomeChangeIcon = data.incomeChangePct >= 0 ? "↑" : "↓";
  const expenseChangeIcon = data.expenseChangePct >= 0 ? "↑" : "↓";
  const incomeChangeColor =
    data.incomeChangePct >= 0 ? "text-green-600" : "text-red-600";
  const expenseChangeColor =
    data.expenseChangePct >= 0 ? "text-red-600" : "text-green-600";

  return (
    <div className="space-y-4">
      <div className="flex flex-col sm:flex-row gap-4 items-start sm:items-center">
        <div className="flex items-center gap-2">
          <Label className="whitespace-nowrap">
            {t("reports.monthComparison.month1")}:
          </Label>
          <MonthPicker
            value={month1}
            onChange={setMonth1}
            className="w-[140px]"
          />
        </div>

        <div className="flex items-center gap-2">
          <Label className="whitespace-nowrap">
            {t("reports.monthComparison.month2")}:
          </Label>
          <MonthPicker
            value={month2}
            onChange={setMonth2}
            className="w-[140px]"
          />
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.monthComparison.incomeComparison")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-1">
              <div className="flex justify-between text-sm">
                <span>{getMonthLabel(data.month1)}</span>
                <span className="text-green-600 font-medium">
                  {formatAmount(data.month1Income)}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span>{getMonthLabel(data.month2)}</span>
                <span className="text-green-600 font-medium">
                  {formatAmount(data.month2Income)}
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-1 mt-1">
                <span className="text-muted-foreground">
                  {t("reports.tableHeaders.change")}
                </span>
                <span className={incomeChangeColor}>
                  {incomeChangeIcon} {formatAmount(Math.abs(data.incomeDiff))} (
                  {Math.abs(data.incomeChangePct).toFixed(1)}%)
                </span>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.monthComparison.expenseComparison")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-1">
              <div className="flex justify-between text-sm">
                <span>{getMonthLabel(data.month1)}</span>
                <span className="text-red-600 font-medium">
                  {formatAmount(data.month1Expense)}
                </span>
              </div>
              <div className="flex justify-between text-sm">
                <span>{getMonthLabel(data.month2)}</span>
                <span className="text-red-600 font-medium">
                  {formatAmount(data.month2Expense)}
                </span>
              </div>
              <div className="flex justify-between text-sm border-t pt-1 mt-1">
                <span className="text-muted-foreground">
                  {t("reports.tableHeaders.change")}
                </span>
                <span className={expenseChangeColor}>
                  {expenseChangeIcon} {formatAmount(Math.abs(data.expenseDiff))}{" "}
                  ({Math.abs(data.expenseChangePct).toFixed(1)}%)
                </span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.monthComparison.comparisonChart")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="name" />
              <YAxis />
              <Tooltip
                formatter={(v) =>
                  typeof v === "number"
                    ? `${symbol}${v.toLocaleString("zh-CN")}`
                    : "--"
                }
              />
              <Legend />
              <Bar
                dataKey="income"
                fill={REPORT_CHART_COLORS.income}
                name={t("reports.common.income")}
              />
              <Bar
                dataKey="expense"
                fill={REPORT_CHART_COLORS.expense}
                name={t("reports.common.expense")}
              />
            </BarChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.monthComparison.categoryDetails")}</CardTitle>
        </CardHeader>
        <CardContent>
          <table className="w-full text-sm">
            <thead className="bg-muted">
              <tr>
                <th className="px-4 py-2 text-left">
                  {t("reports.tableHeaders.category")}
                </th>
                <th className="px-4 py-2 text-right">
                  {getMonthLabel(data.month1)}
                </th>
                <th className="px-4 py-2 text-right">
                  {getMonthLabel(data.month2)}
                </th>
                <th className="px-4 py-2 text-right">
                  {t("reports.tableHeaders.change")}
                </th>
                <th className="px-4 py-2 text-right">
                  {t("reports.tableHeaders.changeRate")}
                </th>
              </tr>
            </thead>
            <tbody>
              {data.categoryComparison.map((item) => {
                const isNewCategory = item.month1Amount === 0;
                const changeIcon = item.diff >= 0 ? "↑" : "↓";
                const changeColor =
                  item.diff >= 0 ? "text-red-600" : "text-green-600";
                const changePctDisplay = isNewCategory
                  ? t("reports.monthComparison.newCategory")
                  : `${Math.abs(item.changePct).toFixed(1)}%`;
                return (
                  <tr key={item.categoryId}>
                    <td className="px-4 py-2">{item.categoryName}</td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(item.month1Amount)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(item.month2Amount)}
                    </td>
                    <td className={`px-4 py-2 text-right ${changeColor}`}>
                      {changeIcon} {formatAmount(Math.abs(item.diff))}
                    </td>
                    <td className={`px-4 py-2 text-right ${changeColor}`}>
                      {changePctDisplay}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </CardContent>
      </Card>
    </div>
  );
}
