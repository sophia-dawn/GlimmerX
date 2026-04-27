import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useDashboardSummary } from "@/hooks/useDashboardSummary";
import { formatAmountWithSign } from "@/utils/format";
import { TrendingUp, TrendingDown } from "lucide-react";

export function DashboardSummaryCards() {
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
      key: "monthIncome",
      title: t("dashboard.monthIncome"),
      value: summary?.month_income ?? 0,
      icon: TrendingUp,
      colorClass: "text-green-600",
    },
    {
      key: "monthExpense",
      title: t("dashboard.monthExpense"),
      value: -(summary?.month_expense ?? 0),
      icon: TrendingDown,
      colorClass: "text-red-600",
    },
    {
      key: "yearIncome",
      title: t("dashboard.yearIncome"),
      value: summary?.year_income ?? 0,
      icon: TrendingUp,
      colorClass: "text-green-600",
    },
    {
      key: "yearExpense",
      title: t("dashboard.yearExpense"),
      value: -(summary?.year_expense ?? 0),
      icon: TrendingDown,
      colorClass: "text-red-600",
    },
  ];

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
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
                {formatAmountWithSign(card.value)}
              </p>
            )}
          </CardContent>
        </Card>
      ))}
    </div>
  );
}
