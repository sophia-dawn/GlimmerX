import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useMonthlyChart } from "@/hooks/useMonthlyChart";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Legend,
} from "recharts";
import { CHART_COLORS } from "@/constants";

export function MonthlyIncomeExpenseChart() {
  const { t } = useTranslation();
  const { data, isLoading, error } = useMonthlyChart();

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>{t("dashboard.monthlyChart")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  // Transform data for Recharts: convert cents to yuan
  const chartData =
    data?.days.map((day) => ({
      date: day.date.slice(5), // Show "MM-DD" format
      income: day.income / 100,
      expense: day.expense / 100,
      hasTransactions: day.has_transactions,
    })) ?? [];

  const hasData = chartData.some((d) => d.hasTransactions);

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("dashboard.monthlyChart")}</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="h-[300px] flex items-center justify-center">
            <p className="text-muted-foreground">{t("common.loading")}</p>
          </div>
        ) : !hasData ? (
          <div className="h-[300px] flex items-center justify-center">
            <p className="text-muted-foreground">{t("common.noRecords")}</p>
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" className="stroke-border" />
              <XAxis
                dataKey="date"
                className="text-muted-foreground"
                tick={{ fontSize: 12 }}
              />
              <YAxis
                className="text-muted-foreground"
                tick={{ fontSize: 12 }}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "var(--color-card)",
                  border: "1px solid var(--color-border)",
                  borderRadius: "var(--radius)",
                }}
                labelStyle={{ color: "var(--color-foreground)" }}
              />
              <Legend />
              <Line
                type="monotone"
                dataKey="income"
                name={t("dashboard.monthlyChartIncome")}
                stroke={CHART_COLORS.income}
                strokeWidth={2}
                dot={{ r: 3 }}
                activeDot={{ r: 5 }}
              />
              <Line
                type="monotone"
                dataKey="expense"
                name={t("dashboard.monthlyChartExpense")}
                stroke={CHART_COLORS.expense}
                strokeWidth={2}
                dot={{ r: 3 }}
                activeDot={{ r: 5 }}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  );
}
