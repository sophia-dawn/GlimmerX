import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Legend,
  Tooltip,
} from "recharts";
import { AlertCircle, Loader2, RotateCcw } from "lucide-react";
import { useCategoryBreakdownReport } from "@/hooks/useCategoryBreakdownReport";
import { formatAmount } from "@/utils/format";
import { REPORT_CATEGORY_COLORS } from "@/constants/report";
import { DATE_RANGE_PRESETS } from "@/constants";
import { getDefaultRangePresetDates } from "@/utils/date";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type { ReportFilter, DateRangePreset } from "@/types/report";

export function CategoryBreakdownReport() {
  const { t } = useTranslation();
  const [incomeOrExpense, setIncomeOrExpense] = useState<"income" | "expense">(
    "expense",
  );
  const [preset, setPreset] = useState<DateRangePreset>("currentMonth");
  const [customStart, setCustomStart] = useState<string>("");
  const [customEnd, setCustomEnd] = useState<string>("");

  const range =
    preset === "custom"
      ? { startDate: customStart, endDate: customEnd }
      : getDefaultRangePresetDates(preset);

  const filter: ReportFilter = {
    dateRangePreset: preset,
    startDate: range.startDate,
    endDate: range.endDate,
    periodGranularity: "monthly",
  };

  const { data, isLoading, error } = useCategoryBreakdownReport(
    filter,
    incomeOrExpense,
  );

  const resetFilter = () => {
    setPreset("currentMonth");
    setCustomStart("");
    setCustomEnd("");
    setIncomeOrExpense("expense");
  };

  const chartData =
    data?.categories?.map((cat, idx) => ({
      name: cat.categoryName,
      value: cat.amount,
      percentage: cat.percentage,
      fill: REPORT_CATEGORY_COLORS[idx % REPORT_CATEGORY_COLORS.length],
    })) ?? [];

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-end gap-4 pb-4">
        <div className="flex flex-col gap-1.5">
          <Label>{t("reports.common.dateRange")}</Label>
          <Select
            value={preset}
            onValueChange={(v) => setPreset(v as DateRangePreset)}
          >
            <SelectTrigger className="w-[180px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {DATE_RANGE_PRESETS.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  {t(p.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {preset === "custom" && (
          <>
            <div className="flex flex-col gap-1.5">
              <Label>{t("reports.dateRange.startDate")}</Label>
              <DatePicker
                value={customStart}
                onChange={setCustomStart}
                placeholder={t("reports.dateRange.startDate")}
              />
            </div>
            <div className="flex flex-col gap-1.5">
              <Label>{t("reports.dateRange.endDate")}</Label>
              <DatePicker
                value={customEnd}
                onChange={setCustomEnd}
                placeholder={t("reports.dateRange.endDate")}
              />
            </div>
          </>
        )}

        <Button variant="outline" size="sm" onClick={resetFilter}>
          <RotateCcw className="h-4 w-4 mr-2" />
          {t("reports.common.reset")}
        </Button>
      </div>

      <div className="flex gap-4">
        <Select
          value={incomeOrExpense}
          onValueChange={(v) => setIncomeOrExpense(v as "income" | "expense")}
        >
          <SelectTrigger className="w-[180px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="expense">
              {t("categories.typeExpense")}
            </SelectItem>
            <SelectItem value="income">{t("categories.typeIncome")}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {isLoading && (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {error && (
        <Card>
          <CardContent className="flex flex-col items-center justify-center py-8 text-destructive">
            <AlertCircle className="h-12 w-12 mb-4" />
            <p className="text-lg">{t("reports.common.error")}</p>
            <p className="text-sm mt-1">{error.message}</p>
          </CardContent>
        </Card>
      )}

      {!isLoading && !error && (!data || data.categories.length === 0) && (
        <Card>
          <CardContent className="text-center py-8 text-muted-foreground">
            <AlertCircle className="h-12 w-12 mb-4 opacity-50 mx-auto" />
            <p>{t("reports.categoryBreakdown.noCategoryData")}</p>
          </CardContent>
        </Card>
      )}

      {!isLoading && !error && data && data.categories.length > 0 && (
        <>
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-muted-foreground">
                {incomeOrExpense === "expense"
                  ? t("reports.categoryBreakdown.expenseTotal")
                  : t("reports.categoryBreakdown.incomeTotal")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="text-2xl font-bold">
                {formatAmount(data.totalAmount)}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>
                {t("reports.categoryBreakdown.categoryDistribution")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <ResponsiveContainer width="100%" height={300}>
                <PieChart>
                  <Pie
                    data={chartData}
                    dataKey="value"
                    nameKey="name"
                    cx="50%"
                    cy="50%"
                    outerRadius={100}
                    label={({ name, payload }) =>
                      `${name}: ${(payload?.percentage ?? 0).toFixed(1)}%`
                    }
                  >
                    {chartData.map((entry, idx) => (
                      <Cell key={idx} fill={entry.fill} />
                    ))}
                  </Pie>
                  <Tooltip
                    formatter={(value) => {
                      if (value === undefined) return "";
                      return formatAmount(Number(value));
                    }}
                  />
                  <Legend />
                </PieChart>
              </ResponsiveContainer>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>
                {t("reports.categoryBreakdown.categoryDetails")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <table className="w-full text-sm">
                <thead className="bg-muted">
                  <tr>
                    <th className="px-4 py-2 text-left">
                      {t("reports.tableHeaders.category")}
                    </th>
                    <th className="px-4 py-2 text-right">
                      {t("reports.tableHeaders.amount")}
                    </th>
                    <th className="px-4 py-2 text-right">
                      {t("reports.tableHeaders.percentage")}
                    </th>
                    <th className="px-4 py-2 text-right">
                      {t("reports.tableHeaders.transactions")}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {data.categories.map((cat) => (
                    <tr key={cat.categoryId} className="border-b">
                      <td className="px-4 py-2">{cat.categoryName}</td>
                      <td className="px-4 py-2 text-right">
                        {formatAmount(cat.amount)}
                      </td>
                      <td className="px-4 py-2 text-right">
                        {cat.percentage.toFixed(1)}%
                      </td>
                      <td className="px-4 py-2 text-right">
                        {cat.transactionCount}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
