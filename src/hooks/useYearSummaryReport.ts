import { useQuery } from "@tanstack/react-query";
import { reportYearSummary } from "@/utils/api";

export function useYearSummaryReport(year: number) {
  return useQuery({
    queryKey: ["report_year_summary", year],
    queryFn: () => reportYearSummary(year),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
