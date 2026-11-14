import { useTranslation } from "react-i18next";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useCategoryBreakdown } from "@/hooks/useCategoryBreakdown";
import {
  PieChart,
  Pie,
  Cell,
  ResponsiveContainer,
  Tooltip,
  Legend,
} from "recharts";
import { useChartColors } from "@/utils/chartColors";
import { formatAmount } from "@/utils/format";

/**
 * Spending by category, as a ring.
 *
 * A solid pie with labels on the slices was the wrong chart here: with six or
 * more categories the labels collide, and the legend already names every one of
 * them — so the labels repeated the legend while obscuring the pie. The ring
 * keeps a hole for the eye to rest in and leaves the labelling to the legend.
 *
 * Colours come from the theme's eight-hue series rather than a hardcoded
 * "dashboard palette", so the chart belongs to the same system as everything
 * around it and adapts when the theme does.
 */
export function CategoryBreakdownChart() {
  const { t } = useTranslation();
  const colors = useChartColors();
  const { data, isLoading, error } = useCategoryBreakdown();

  if (error) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>{t("dashboard.categoryBreakdown")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  const chartData =
    data?.categories.map((cat) => ({
      name: cat.category_name,
      value: cat.amount / 100,
      percentage: cat.percentage,
      icon: cat.icon,
    })) ?? [];

  const hasData = chartData.length > 0;

  return (
    <Card className="flex h-[300px] shrink-0 flex-col">
      <CardHeader>
        <CardTitle>{t("dashboard.categoryBreakdown")}</CardTitle>
      </CardHeader>
      <CardContent className="min-h-0 flex-1">
        {isLoading ? (
          <div className="flex h-full min-h-[180px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.loading")}</p>
          </div>
        ) : !hasData ? (
          <div className="flex h-full min-h-[180px] items-center justify-center">
            <p className="text-muted-foreground">{t("common.noRecords")}</p>
          </div>
        ) : (
          <div className="relative h-full min-h-[196px]">
            <ResponsiveContainer width="100%" height="100%">
              <PieChart>
                <Pie
                  data={chartData}
                  cx="50%"
                  cy="50%"
                  innerRadius={62}
                  outerRadius={98}
                  paddingAngle={1}
                  stroke="none"
                  dataKey="value"
                >
                  {chartData.map((entry, index) => (
                    <Cell
                      key={`cell-${entry.name}`}
                      fill={colors.series[index % colors.series.length]}
                    />
                  ))}
                </Pie>
                <Tooltip
                  contentStyle={{
                    backgroundColor: "var(--color-popover)",
                    border: "1px solid var(--color-border)",
                    borderRadius: "var(--radius)",
                    fontSize: "12px",
                  }}
                  formatter={(value) => formatAmount((value as number) * 100)}
                />
                <Legend
                  layout="horizontal"
                  verticalAlign="bottom"
                  align="center"
                  iconSize={8}
                  wrapperStyle={{ paddingTop: "8px", fontSize: "11px" }}
                />
              </PieChart>
            </ResponsiveContainer>

            {/* The hole is where the total belongs: it is the one number every
                slice divides into, and the legend below already names them. */}
            <div className="pointer-events-none absolute inset-0 flex flex-col items-center justify-center pb-11">
              <span className="text-xs text-muted-foreground">
                {t("dashboard.categoryTotal")}
              </span>
              <span className="font-mono text-base font-medium">
                {formatAmount(data?.total_amount ?? 0)}
              </span>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
