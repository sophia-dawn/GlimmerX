import { useQuery } from "@tanstack/react-query";
import { reportMonthComparison } from "@/utils/api";

interface UseMonthComparisonReportParams {
  month1: string;
  month2: string;
  enabled?: boolean;
}

export function useMonthComparisonReport({
  month1,
  month2,
  enabled = true,
}: UseMonthComparisonReportParams) {
  return useQuery({
    queryKey: ["monthComparisonReport", month1, month2],
    queryFn: () => reportMonthComparison(month1, month2),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    enabled,
  });
}
