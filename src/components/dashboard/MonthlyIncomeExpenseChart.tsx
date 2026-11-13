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
import { useChartColors } from "@/utils/chartColors";
import { formatAmount } from "@/utils/format";

export function MonthlyIncomeExpenseChart() {
  const { t, i18n } = useTranslation();
  const colors = useChartColors();
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

  // Full digits with grouping, not compact notation. `notation: "compact"`
  // switches unit at 10 000, so one axis read "1.8万" above "9,000" — two scales
  // on one axis. Every tick has to be in the same unit to be comparable.
  const formatTick = (value: number) => value.toLocaleString(i18n.language);

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t("dashboard.monthlyChart")}</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex h-[300px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.loading")}</p>
          </div>
        ) : !hasData ? (
          <div className="flex h-[300px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.noRecords")}</p>
          </div>
        ) : (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData} margin={{ top: 4, right: 8, left: 0 }}>
              <CartesianGrid
                stroke={colors.grid}
                strokeDasharray="2 4"
                vertical={false}
              />
              <XAxis
                dataKey="date"
                tick={{ fontSize: 11, fill: colors.axis }}
                tickLine={false}
                axisLine={{ stroke: colors.grid }}
                interval="preserveStartEnd"
                minTickGap={24}
              />
              <YAxis
                tick={{ fontSize: 11, fill: colors.axis }}
                tickLine={false}
                axisLine={false}
                width={56}
                tickFormatter={formatTick}
              />
              <Tooltip
                contentStyle={{
                  backgroundColor: "var(--color-popover)",
                  border: "1px solid var(--color-border)",
                  borderRadius: "var(--radius)",
                  fontSize: "12px",
                }}
                labelStyle={{ color: "var(--color-muted-foreground)" }}
                formatter={(value) => formatAmount((value as number) * 100)}
              />
              <Legend iconSize={8} wrapperStyle={{ fontSize: "11px" }} />
              <Line
                type="monotone"
                dataKey="income"
                name={t("dashboard.monthlyChartIncome")}
                stroke={colors.income}
                strokeWidth={1.5}
                dot={false}
                activeDot={{ r: 4 }}
              />
              <Line
                type="monotone"
                dataKey="expense"
                name={t("dashboard.monthlyChartExpense")}
                stroke={colors.expense}
                strokeWidth={1.5}
                dot={false}
                activeDot={{ r: 4 }}
              />
            </LineChart>
          </ResponsiveContainer>
        )}
      </CardContent>
    </Card>
  );
}
