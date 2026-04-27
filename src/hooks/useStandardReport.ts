import { useQuery } from "@tanstack/react-query";
import { reportStandard } from "@/utils/api";
import type { ReportFilter } from "@/types/report";

export function useStandardReport(filter: ReportFilter) {
  return useQuery({
    queryKey: ["standardReport", filter],
    queryFn: () => reportStandard(filter),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
