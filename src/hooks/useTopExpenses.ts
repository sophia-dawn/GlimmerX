import { useQuery } from "@tanstack/react-query";
import { dashboardTopExpenses } from "@/utils/api";
import { useMemo } from "react";
import { DASHBOARD_LIMITS } from "@/constants";

export function useTopExpenses(year?: number, month?: number, limit?: number) {
  const params = useMemo(() => {
    const now = new Date();
    return {
      year: year ?? now.getFullYear(),
      month: month ?? now.getMonth() + 1,
      limit: limit ?? DASHBOARD_LIMITS.topExpenses,
    };
  }, [year, month, limit]);

  return useQuery({
    queryKey: [
      "dashboard-top-expenses",
      params.year,
      params.month,
      params.limit,
    ],
    queryFn: () => dashboardTopExpenses(params),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
