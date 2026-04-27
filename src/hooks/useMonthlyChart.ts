import { useQuery } from "@tanstack/react-query";
import { dashboardMonthlyChart } from "@/utils/api";
import { useMemo } from "react";

export function useMonthlyChart(year?: number, month?: number) {
  const params = useMemo(() => {
    const now = new Date();
    return {
      year: year ?? now.getFullYear(),
      month: month ?? now.getMonth() + 1,
    };
  }, [year, month]);

  return useQuery({
    queryKey: ["dashboard-monthly-chart", params.year, params.month],
    queryFn: () => dashboardMonthlyChart(params),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
