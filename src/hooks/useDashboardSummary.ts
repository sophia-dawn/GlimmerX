import { useQuery } from "@tanstack/react-query";
import { dashboardSummary } from "@/utils/api";
import { useMemo } from "react";
import { currentMonthBoundsLocal } from "@/utils/date";

export function useDashboardSummary() {
  const params = useMemo(() => {
    const { start, end } = currentMonthBoundsLocal();
    return {
      fromDate: start,
      toDate: end,
    };
  }, []);

  return useQuery({
    queryKey: ["dashboard-summary", params.fromDate, params.toDate],
    queryFn: () => dashboardSummary(params),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
