import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { DatePicker } from "@/components/ui/date-picker";
import { Label } from "@/components/ui/label";
import {
  AlertCircle,
  Loader2,
  Wallet,
  CreditCard,
  TrendingUp,
} from "lucide-react";
import { useBalanceSheetReport } from "@/hooks/useBalanceSheetReport";
import { formatAmount } from "@/utils/format";
import { todayLocalDate } from "@/utils/date";
import { useState } from "react";
import { REPORT_CHART_COLORS } from "@/constants/report";
import { useTranslation } from "react-i18next";
import { useCurrency } from "@/hooks/useCurrency";

export function BalanceSheetReport() {
  const { t } = useTranslation();
  const { symbol } = useCurrency();
  const [snapshotDate, setSnapshotDate] = useState(todayLocalDate());
  const { data, isLoading, error } = useBalanceSheetReport(snapshotDate);

  const formatAccountType = (type: string) => {
    const key = `accounts.types.${type}`;
    const translated = t(key);
    return translated === key ? type : translated;
  };

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

  const formatDateForDisplay = (date: string) => {
    const parts = date.split("-");
    if (parts.length === 3) {
      const year = parts[0];
      const month = parseInt(parts[1] ?? "1", 10);
      const day = parts[2];
      return `${year} ${t(`reports.monthNames.${month}`)} ${day}`;
    }
    return date;
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <Label>{t("reports.balanceSheet.snapshotDate")}:</Label>
        <DatePicker
          value={snapshotDate}
          onChange={setSnapshotDate}
          className="w-[180px]"
        />
      </div>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm text-muted-foreground">
            {t("reports.balanceSheet.netWorth")} (
            {formatDateForDisplay(snapshotDate)})
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex items-center justify-between">
            <div>
              <div className="text-2xl font-bold text-blue-600">
                {data ? formatAmount(data.netWorth) : `${symbol}0.00`}
              </div>
              <div className="text-sm mt-1 text-muted-foreground">
                {t("reports.balanceSheet.netWorthFormula")}
              </div>
            </div>
            <TrendingUp className="h-8 w-8 text-muted-foreground opacity-50" />
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wallet
              className="h-5 w-5"
              style={{ color: REPORT_CHART_COLORS.assets }}
            />
            {t("reports.balanceSheet.assetDetails")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {!data || data.assets.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
              <AlertCircle className="h-10 w-10 mb-3 opacity-50" />
              <p className="text-lg">{t("reports.balanceSheet.noAssetData")}</p>
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead className="bg-muted">
                <tr>
                  <th className="px-4 py-2 text-left">
                    {t("reports.tableHeaders.accountName")}
                  </th>
                  <th className="px-4 py-2 text-left">
                    {t("reports.tableHeaders.accountType")}
                  </th>
                  <th className="px-4 py-2 text-right">
                    {t("reports.tableHeaders.balance")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.assets.map((item) => (
                  <tr key={item.accountId} className="border-b">
                    <td className="px-4 py-2">{item.accountName}</td>
                    <td className="px-4 py-2">
                      {formatAccountType(item.accountType)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(item.balance)}
                    </td>
                  </tr>
                ))}
              </tbody>
              <tfoot className="bg-muted/50">
                <tr>
                  <td className="px-4 py-2 font-medium" colSpan={2}>
                    {t("reports.balanceSheet.assetTotal")}
                  </td>
                  <td className="px-4 py-2 text-right font-medium">
                    {formatAmount(data.totalAssets)}
                  </td>
                </tr>
              </tfoot>
            </table>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <CreditCard
              className="h-5 w-5"
              style={{ color: REPORT_CHART_COLORS.liabilities }}
            />
            {t("reports.balanceSheet.liabilityDetails")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {!data || data.liabilities.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-6 text-muted-foreground">
              <AlertCircle className="h-10 w-10 mb-3 opacity-50" />
              <p className="text-lg">
                {t("reports.balanceSheet.noLiabilityData")}
              </p>
            </div>
          ) : (
            <table className="w-full text-sm">
              <thead className="bg-muted">
                <tr>
                  <th className="px-4 py-2 text-left">
                    {t("reports.tableHeaders.accountName")}
                  </th>
                  <th className="px-4 py-2 text-left">
                    {t("reports.tableHeaders.accountType")}
                  </th>
                  <th className="px-4 py-2 text-right">
                    {t("reports.tableHeaders.balance")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.liabilities.map((item) => (
                  <tr key={item.accountId} className="border-b">
                    <td className="px-4 py-2">{item.accountName}</td>
                    <td className="px-4 py-2">
                      {formatAccountType(item.accountType)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(item.balance)}
                    </td>
                  </tr>
                ))}
              </tbody>
              <tfoot className="bg-muted/50">
                <tr>
                  <td className="px-4 py-2 font-medium" colSpan={2}>
                    {t("reports.balanceSheet.liabilityTotal")}
                  </td>
                  <td className="px-4 py-2 text-right font-medium">
                    {formatAmount(data.totalLiabilities)}
                  </td>
                </tr>
              </tfoot>
            </table>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.balanceSheet.summary")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4 text-center">
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.balanceSheet.totalAssets")}
              </div>
              <div
                className="text-xl font-bold"
                style={{ color: REPORT_CHART_COLORS.assets }}
              >
                {data ? formatAmount(data.totalAssets) : `${symbol}0.00`}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.balanceSheet.totalLiabilities")}
              </div>
              <div
                className="text-xl font-bold"
                style={{ color: REPORT_CHART_COLORS.liabilities }}
              >
                {data ? formatAmount(data.totalLiabilities) : `${symbol}0.00`}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.balanceSheet.netWorth")}
              </div>
              <div
                className="text-xl font-bold"
                style={{ color: REPORT_CHART_COLORS.net }}
              >
                {data ? formatAmount(data.netWorth) : `${symbol}0.00`}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
