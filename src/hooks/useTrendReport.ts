import { useQuery } from "@tanstack/react-query";
import { reportTrend } from "@/utils/api";
import type { ReportFilter } from "@/types/report";

export function useTrendReport(filter: ReportFilter) {
  return useQuery({
    queryKey: ["report_trend", filter],
    queryFn: () => reportTrend(filter),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
