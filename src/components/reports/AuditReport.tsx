import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { AlertCircle, Loader2, CheckCircle, AlertTriangle } from "lucide-react";
import { useAuditReport } from "@/hooks/useAuditReport";
import { formatAmount } from "@/utils/format";
import { formatDateTime } from "@/utils/date";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  UnbalancedTransaction,
  UncategorizedTransaction,
  AccountUsageStat,
} from "@/types/report";

export function AuditReport() {
  const { t } = useTranslation();
  const { data, isLoading, error } = useAuditReport();
  const [expandedSection, setExpandedSection] = useState<string | null>(null);

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

  const totalIssues =
    data.balanceCheck.unbalancedCount +
    data.categoryCheck.uncategorizedTransactions;

  const isHealthy = totalIssues === 0;

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center gap-2">
            {isHealthy ? (
              <CheckCircle className="h-5 w-5 text-green-600" />
            ) : (
              <AlertTriangle className="h-5 w-5 text-yellow-600" />
            )}
            {t("reports.audit.overview")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-4 gap-4 text-center">
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.audit.totalTransactions")}
              </div>
              <div className="text-xl font-bold">
                {data.balanceCheck.totalTransactions}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.audit.balancedTransactions")}
              </div>
              <div className="text-xl font-bold text-green-600">
                {data.balanceCheck.balancedCount}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.audit.issuesFound")}
              </div>
              <div
                className={`text-xl font-bold ${isHealthy ? "text-green-600" : "text-red-600"}`}
              >
                {totalIssues}
              </div>
            </div>
            <div>
              <div className="text-sm text-muted-foreground">
                {t("reports.audit.generatedAt")}
              </div>
              <div className="text-sm">{formatDateTime(data.generatedAt)}</div>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              {data.balanceCheck.unbalancedCount === 0 ? (
                <CheckCircle className="h-4 w-4 text-green-600" />
              ) : (
                <AlertTriangle className="h-4 w-4 text-yellow-600" />
              )}
              {t("reports.audit.balanceCheck")}
            </span>
            <span className="text-sm text-muted-foreground">
              {t("reports.audit.unbalancedTransactions", {
                count: data.balanceCheck.unbalancedCount,
              })}
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {data.balanceCheck.unbalancedCount === 0 ? (
            <div className="text-center py-4 text-muted-foreground">
              {t("reports.audit.balanceCheckPassed")}
            </div>
          ) : (
            <>
              <Button
                variant="link"
                size="sm"
                onClick={() =>
                  setExpandedSection(
                    expandedSection === "balance" ? null : "balance",
                  )
                }
                className="mb-2"
              >
                {expandedSection === "balance"
                  ? t("reports.common.collapseDetails")
                  : t("reports.common.expandDetails")}
              </Button>
              {expandedSection === "balance" && (
                <table className="w-full text-sm">
                  <thead className="bg-muted">
                    <tr>
                      <th className="px-4 py-2 text-left">
                        {t("reports.tableHeaders.date")}
                      </th>
                      <th className="px-4 py-2 text-left">
                        {t("reports.tableHeaders.description")}
                      </th>
                      <th className="px-4 py-2 text-right">
                        {t("reports.tableHeaders.difference")}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.balanceCheck.unbalancedTransactions.map(
                      (tx: UnbalancedTransaction) => (
                        <tr key={tx.transactionId} className="border-b">
                          <td className="px-4 py-2">{tx.date}</td>
                          <td className="px-4 py-2">{tx.description}</td>
                          <td
                            className={`px-4 py-2 text-right ${tx.sum > 0 ? "text-red-600" : "text-green-600"}`}
                          >
                            {formatAmount(tx.sum)}
                          </td>
                        </tr>
                      ),
                    )}
                  </tbody>
                </table>
              )}
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="flex items-center justify-between">
            <span className="flex items-center gap-2">
              {data.categoryCheck.uncategorizedTransactions === 0 ? (
                <CheckCircle className="h-4 w-4 text-green-600" />
              ) : (
                <AlertTriangle className="h-4 w-4 text-yellow-600" />
              )}
              {t("reports.audit.categoryCheck")}
            </span>
            <span className="text-sm text-muted-foreground">
              {t("reports.audit.uncategorizedTransactions", {
                count: data.categoryCheck.uncategorizedTransactions,
              })}
            </span>
          </CardTitle>
        </CardHeader>
        <CardContent>
          {data.categoryCheck.uncategorizedTransactions === 0 ? (
            <div className="text-center py-4 text-muted-foreground">
              {t("reports.audit.categoryCheckPassed")}
            </div>
          ) : (
            <div>
              <Button
                variant="link"
                size="sm"
                onClick={() =>
                  setExpandedSection(
                    expandedSection === "uncategorized"
                      ? null
                      : "uncategorized",
                  )
                }
                className="mb-2"
              >
                {expandedSection === "uncategorized"
                  ? t("reports.audit.collapseUncategorized")
                  : t("reports.audit.expandUncategorized")}
              </Button>
              {expandedSection === "uncategorized" && (
                <table className="w-full text-sm">
                  <thead className="bg-muted">
                    <tr>
                      <th className="px-4 py-2 text-left">
                        {t("reports.tableHeaders.date")}
                      </th>
                      <th className="px-4 py-2 text-left">
                        {t("reports.tableHeaders.description")}
                      </th>
                    </tr>
                  </thead>
                  <tbody>
                    {data.categoryCheck.uncategorizedList.map(
                      (tx: UncategorizedTransaction) => (
                        <tr key={tx.transactionId} className="border-b">
                          <td className="px-4 py-2">{tx.date}</td>
                          <td className="px-4 py-2">{tx.description}</td>
                        </tr>
                      ),
                    )}
                  </tbody>
                </table>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t("reports.audit.accountUsage")}</CardTitle>
        </CardHeader>
        <CardContent>
          {data.accountUsage.length === 0 ? (
            <div className="text-center py-4 text-muted-foreground">
              {t("reports.audit.noAccountUsageData")}
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
                    {t("reports.tableHeaders.postingCount")}
                  </th>
                  <th className="px-4 py-2 text-right">
                    {t("reports.tableHeaders.debitTotal")}
                  </th>
                  <th className="px-4 py-2 text-right">
                    {t("reports.tableHeaders.creditTotal")}
                  </th>
                  <th className="px-4 py-2 text-left">
                    {t("reports.audit.lastTransaction")}
                  </th>
                </tr>
              </thead>
              <tbody>
                {data.accountUsage.map((stat: AccountUsageStat) => (
                  <tr key={stat.accountId} className="border-b">
                    <td className="px-4 py-2">{stat.accountName}</td>
                    <td className="px-4 py-2">
                      {t(`accounts.types.${stat.accountType}`)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {stat.postingCount}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(stat.totalDebit)}
                    </td>
                    <td className="px-4 py-2 text-right">
                      {formatAmount(stat.totalCredit)}
                    </td>
                    <td className="px-4 py-2">
                      {stat.lastTransactionDate ?? "-"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
