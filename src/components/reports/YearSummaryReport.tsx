import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Label } from "@/components/ui/label";
import { YearPicker } from "@/components/ui/year-picker";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
} from "recharts";
import { AlertCircle, Loader2 } from "lucide-react";
import { useYearSummaryReport } from "@/hooks/useYearSummaryReport";
import { formatAmount } from "@/utils/format";
import { useState } from "react";
import { todayLocalDate } from "@/utils/date";
import { useTranslation } from "react-i18next";

export function YearSummaryReport() {
  const { t } = useTranslation();
  const currentYear = parseInt(todayLocalDate().split("-")[0] ?? "2026");
  const [year, setYear] = useState(currentYear);
  const { data, isLoading, error } = useYearSummaryReport(year);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center py-8 text-destructive">
        <AlertCircle className="h-12 w-12 mb-4" />
        <p className="text-lg">{t("reports.common.error")}</p>
        <p className="text-sm mt-1">{error.message}</p>
      </div>
    );
  }

  if (!data) {
    return (
      <Card>
        <CardContent className="text-center py-8 text-muted-foreground">
          {t("reports.common.noData")}
        </CardContent>
      </Card>
    );
  }

  const chartData = data.monthlyBreakdown.map((m) => ({
    month: t(`reports.monthNames.${m.month}`),
    income: m.income,
    expense: m.expense,
    net: m.net,
  }));

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Label>{t("reports.yearSummary.yearLabel")}</Label>
        <YearPicker
          value={year}
          onChange={setYear}
          className="w-[120px]"
          minYear={2010}
          maxYear={currentYear}
        />
      </div>

      <div className="grid grid-cols-3 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.yearSummary.totalIncome")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-green-600">
              {formatAmount(data.totalIncome)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.yearSummary.totalExpense")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold text-red-600">
              {formatAmount(data.totalExpense)}
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm text-muted-foreground">
              {t("reports.yearSummary.annualNet")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div
              className={`text-2xl font-bold ${data.net >= 0 ? "text-green-600" : "text-red-600"}`}
            >
              {formatAmount(data.net)}
            </div>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.yearSummary.monthlyTrend")}</CardTitle>
        </CardHeader>
        <CardContent>
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" />
              <XAxis dataKey="month" />
              <YAxis />
              <Tooltip
                formatter={(value) =>
                  value !== undefined ? formatAmount(Number(value)) : ""
                }
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="income"
                stroke="#16a34a"
                name={t("reports.common.income")}
              />
              <Line
                type="monotone"
                dataKey="expense"
                stroke="#dc2626"
                name={t("reports.common.expense")}
              />
              <Line
                type="monotone"
                dataKey="net"
                stroke="#3b82f6"
                name={t("reports.common.netIncome")}
              />
            </LineChart>
          </ResponsiveContainer>
        </CardContent>
      </Card>

      <div className="grid grid-cols-2 gap-4">
        <Card>
          <CardHeader>
            <CardTitle>
              {t("reports.yearSummary.topIncomeCategories")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {data.topIncomeCategories.length === 0 ? (
              <div className="text-center py-4 text-muted-foreground">
                {t("reports.common.noData")}
              </div>
            ) : (
              <table className="w-full text-sm">
                <tbody>
                  {data.topIncomeCategories.map((cat) => (
                    <tr key={cat.categoryId} className="border-b">
                      <td className="py-2">{cat.categoryName}</td>
                      <td className="py-2 text-right">
                        {formatAmount(cat.amount)}
                      </td>
                      <td className="py-2 text-right text-muted-foreground">
                        {cat.percentage.toFixed(1)}%
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>
              {t("reports.yearSummary.topExpenseCategories")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {data.topExpenseCategories.length === 0 ? (
              <div className="text-center py-4 text-muted-foreground">
                {t("reports.common.noData")}
              </div>
            ) : (
              <table className="w-full text-sm">
                <tbody>
                  {data.topExpenseCategories.map((cat) => (
                    <tr key={cat.categoryId} className="border-b">
                      <td className="py-2">{cat.categoryName}</td>
                      <td className="py-2 text-right">
                        {formatAmount(cat.amount)}
                      </td>
                      <td className="py-2 text-right text-muted-foreground">
                        {cat.percentage.toFixed(1)}%
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
