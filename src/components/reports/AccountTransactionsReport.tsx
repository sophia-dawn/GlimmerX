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
import { Button } from "@/components/ui/button";
import { AlertCircle, Loader2, ChevronLeft, ChevronRight } from "lucide-react";
import { useAccountTransactionsReport } from "@/hooks/useAccountTransactionsReport";
import { useAccountList } from "@/hooks/useAccountList";
import { formatAmount } from "@/utils/format";
import { useState, useEffect } from "react";
import type {
  ReportFilter,
  DateRangePreset,
  PeriodGranularity,
} from "@/types/report";
import { DATE_RANGE_PRESETS } from "@/constants/report";
import { getDefaultRangePresetDates } from "@/utils/date";
import { useTranslation } from "react-i18next";

export function AccountTransactionsReport() {
  const { t } = useTranslation();
  const { data: accounts, isLoading: accountsLoading } = useAccountList();
  const [selectedAccountId, setSelectedAccountId] = useState<string>("");
  const [preset, setPreset] = useState<DateRangePreset>("currentMonth");
  const [customStart, setCustomStart] = useState<string>("");
  const [customEnd, setCustomEnd] = useState<string>("");
  const [page, setPage] = useState(1);
  const pageSize = 50;

  // Reset page when account or filter changes
  useEffect(() => {
    setPage(1);
  }, [selectedAccountId, preset, customStart, customEnd]);

  const range =
    preset === "custom"
      ? { startDate: customStart, endDate: customEnd }
      : getDefaultRangePresetDates(preset);

  const filter: ReportFilter = {
    dateRangePreset: preset,
    startDate: range.startDate,
    endDate: range.endDate,
    periodGranularity: "monthly" as PeriodGranularity,
  };

  const {
    data: reportData,
    isLoading: reportLoading,
    error,
  } = useAccountTransactionsReport(selectedAccountId, filter, page, pageSize);

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

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader>
          <CardTitle>{t("reports.names.accountTransactions")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4 mb-4">
            <div>
              <Label>{t("reports.accountTransactions.selectAccount")}</Label>
              <Select
                value={selectedAccountId}
                onValueChange={setSelectedAccountId}
              >
                <SelectTrigger className="w-full mt-1">
                  <SelectValue
                    placeholder={t("reports.accountTransactions.selectAccount")}
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
            {preset === "custom" && (
              <div className="col-span-1">
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
            {t("reports.accountTransactions.selectAccount")}
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
        <>
          <div className="grid grid-cols-3 gap-4">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  {t("reports.accountTransactions.totalInflow")}
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-green-600">
                  {formatAmount(reportData.totalInflow)}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  {t("reports.accountTransactions.totalOutflow")}
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-2xl font-bold text-red-600">
                  {formatAmount(reportData.totalOutflow)}
                </div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm text-muted-foreground">
                  {t("reports.accountTransactions.netChange")}
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div
                  className={`text-2xl font-bold ${reportData.netChange >= 0 ? "text-green-600" : "text-red-600"}`}
                >
                  {formatAmount(reportData.netChange)}
                </div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>
                {t("reports.accountTransactions.transactionDetails")} (
                {reportData.startDate} ~ {reportData.endDate})
              </CardTitle>
            </CardHeader>
            <CardContent>
              {reportData.transactions.length === 0 ? (
                <div className="text-center py-8 text-muted-foreground">
                  {t("reports.common.noData")}
                </div>
              ) : (
                <>
                  <table className="w-full text-sm">
                    <thead className="bg-muted">
                      <tr>
                        <th className="px-4 py-2 text-left">
                          {t("reports.tableHeaders.date")}
                        </th>
                        <th className="px-4 py-2 text-left">
                          {t("reports.tableHeaders.description")}
                        </th>
                        <th className="px-4 py-2 text-left">
                          {t("reports.tableHeaders.category")}
                        </th>
                        <th className="px-4 py-2 text-left">
                          {t("reports.accountTransactions.counterAccount")}
                        </th>
                        <th className="px-4 py-2 text-right">
                          {t("reports.tableHeaders.amount")}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      {reportData.transactions.map((tx) => (
                        <tr key={tx.transactionId} className="border-b">
                          <td className="px-4 py-2">{tx.date}</td>
                          <td className="px-4 py-2">{tx.description}</td>
                          <td className="px-4 py-2">
                            {tx.categoryName ?? "-"}
                          </td>
                          <td className="px-4 py-2">
                            {tx.counterAccountName ?? "-"}
                          </td>
                          <td
                            className={`px-4 py-2 text-right ${tx.isInflow ? "text-green-600" : "text-red-600"}`}
                          >
                            {tx.isInflow ? "+" : "-"}
                            {formatAmount(tx.amount)}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                  {reportData.totalPages > 1 && (
                    <div className="flex items-center justify-center gap-4 py-4 border-t mt-4">
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={page === 1}
                        onClick={() => setPage(page - 1)}
                      >
                        <ChevronLeft className="h-4 w-4" />
                        {t("common.previous")}
                      </Button>
                      <span className="text-sm text-muted-foreground">
                        {t("reports.accountTransactions.page")} {page}{" "}
                        {t("reports.accountTransactions.of")}{" "}
                        {reportData.totalPages}
                      </span>
                      <Button
                        variant="outline"
                        size="sm"
                        disabled={page >= reportData.totalPages}
                        onClick={() => setPage(page + 1)}
                      >
                        {t("common.next")}
                        <ChevronRight className="h-4 w-4" />
                      </Button>
                    </div>
                  )}
                </>
              )}
            </CardContent>
          </Card>
        </>
      )}
    </div>
  );
}
