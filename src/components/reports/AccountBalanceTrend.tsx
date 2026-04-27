import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { DatePicker } from "@/components/ui/date-picker";
import { Label } from "@/components/ui/label";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { AlertCircle, Loader2 } from "lucide-react";
import { useAccountBalanceTrendReport } from "@/hooks/useAccountBalanceTrendReport";
import { useAccountList } from "@/hooks/useAccountList";
import { useState } from "react";
import type {
  ReportFilter,
  DateRangePreset,
  PeriodGranularity,
} from "@/types/report";
import { DATE_RANGE_PRESETS, GRANULARITY_OPTIONS } from "@/constants/report";
import { getDefaultRangePresetDates } from "@/utils/date";
import { useTranslation } from "react-i18next";
import { useCurrency } from "@/hooks/useCurrency";

export function AccountBalanceTrend() {
  const { t } = useTranslation();
  const { symbol } = useCurrency();
  const { data: accounts, isLoading: accountsLoading } = useAccountList();
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [preset, setPreset] = useState<DateRangePreset>("currentMonth");
  const [granularity, setGranularity] = useState<PeriodGranularity>("monthly");
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
    periodGranularity: granularity,
  };

  const {
    data: reportData,
    isLoading: reportLoading,
    error,
  } = useAccountBalanceTrendReport(selectedAccountId, filter);

  if (accountsLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  const assetAccounts =
    accounts?.filter(
      (a) => a.account_type === "asset" || a.account_type === "liability",
    ) ?? [];

  const chartData =
    reportData?.dataPoints.map((point) => ({
      period: point.period,
      balance: point.balance / 100,
    })) ?? [];

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>{t("reports.accountBalanceTrend.balanceTrend")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-4 gap-4 mb-4">
            <div>
              <Label>{t("reports.accountBalanceTrend.selectAccount")}</Label>
              <Select
                value={selectedAccountId}
                onValueChange={setSelectedAccountId}
              >
                <SelectTrigger className="w-full mt-1">
                  <SelectValue
                    placeholder={t("reports.accountBalanceTrend.selectAccount")}
                  />
                </SelectTrigger>
                <SelectContent>
                  {assetAccounts.map((a) => (
                    <SelectItem key={a.id} value={a.id}>
                      {a.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label>{t("reports.common.dateRange")}</Label>
              <Select
                value={preset}
                onValueChange={(v) => setPreset(v as DateRangePreset)}
              >
                <SelectTrigger className="w-full mt-1">
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
            <div>
              <Label>{t("reports.common.granularity")}</Label>
              <Select
                value={granularity}
                onValueChange={(v) => setGranularity(v as PeriodGranularity)}
              >
                <SelectTrigger className="w-full mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {GRANULARITY_OPTIONS.map((g) => (
                    <SelectItem key={g.id} value={g.id}>
                      {t(g.labelKey)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {preset === "custom" && (
              <div>
                <Label>{t("reports.dateRange.custom")}</Label>
                <div className="flex gap-2 mt-1">
                  <DatePicker
                    value={customStart}
                    onChange={setCustomStart}
                    placeholder={t("reports.dateRange.startDate")}
                    className="flex-1"
                  />
                  <DatePicker
                    value={customEnd}
                    onChange={setCustomEnd}
                    placeholder={t("reports.dateRange.endDate")}
                    className="flex-1"
                  />
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {!selectedAccountId && (
        <Card>
          <CardContent className="text-center py-8 text-muted-foreground">
            {t("reports.accountBalanceTrend.selectAccount")}
          </CardContent>
        </Card>
      )}

      {selectedAccountId && reportLoading && (
        <div className="flex items-center justify-center py-8">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}

      {selectedAccountId && error && (
        <div className="flex flex-col items-center justify-center py-8 text-destructive">
          <AlertCircle className="h-12 w-12 mb-4" />
          <p className="text-lg">{t("reports.common.error")}</p>
          <p className="text-sm mt-1">{error.message}</p>
        </div>
      )}

      {selectedAccountId && reportData && (
        <Card>
          <CardHeader>
            <CardTitle>
              {reportData.accountName}{" "}
              {t("reports.accountBalanceTrend.balanceTrend")} (
              {reportData.granularity})
            </CardTitle>
          </CardHeader>
          <CardContent>
            {chartData.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                {t("reports.common.noData")}
              </div>
            ) : (
              <ResponsiveContainer width="100%" height={300}>
                <LineChart data={chartData}>
                  <CartesianGrid strokeDasharray="3 3" />
                  <XAxis dataKey="period" />
                  <YAxis />
                  <Tooltip
                    formatter={(value) =>
                      value !== undefined
                        ? `${symbol}${Number(value).toFixed(2)}`
                        : ""
                    }
                  />
                  <Line
                    type="monotone"
                    dataKey="balance"
                    stroke="#3b82f6"
                    strokeWidth={2}
                    name={t("reports.accountBalanceTrend.balanceHistory")}
                  />
                </LineChart>
              </ResponsiveContainer>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
