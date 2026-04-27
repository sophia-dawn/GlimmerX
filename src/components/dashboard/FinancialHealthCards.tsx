import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useDashboardSummary } from "@/hooks/useDashboardSummary";
import { formatAmount } from "@/utils/format";
import { Wallet, CreditCard, Scale } from "lucide-react";

export function FinancialHealthCards() {
  const { t } = useTranslation();
  const { data: summary, isLoading, error } = useDashboardSummary();

  if (error) {
    return (
      <div className="text-destructive text-sm">
        {t("common.errorGeneric")}: {String(error)}
      </div>
    );
  }

  const cards = [
    {
      key: "totalAssets",
      title: t("dashboard.totalAssets"),
      value: summary?.total_assets ?? 0,
      icon: Wallet,
      colorClass: "text-blue-600",
    },
    {
      key: "totalDebt",
      title: t("dashboard.totalDebt"),
      value: summary?.total_liabilities ?? 0,
      icon: CreditCard,
      colorClass: "text-orange-600",
    },
    {
      key: "netWorth",
      title: t("dashboard.netWorth"),
      value: summary?.net_worth ?? 0,
      icon: Scale,
      colorClass:
        (summary?.net_worth ?? 0) >= 0 ? "text-green-600" : "text-red-600",
    },
  ];

  return (
    <div className="grid gap-4 md:grid-cols-3">
      {cards.map((card) => (
        <Card key={card.key}>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">{card.title}</CardTitle>
            <card.icon className={`h-4 w-4 ${card.colorClass}`} />
          </CardHeader>
          <CardContent>
            {isLoading ? (
              <p className="text-2xl font-bold">--</p>
            ) : (
              <p className={`text-2xl font-bold ${card.colorClass}`}>
                {formatAmount(card.value)}
              </p>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
