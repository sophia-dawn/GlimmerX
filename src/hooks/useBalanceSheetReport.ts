import { useQuery } from "@tanstack/react-query";
import { reportBalanceSheet } from "@/utils/api";

export function useBalanceSheetReport(snapshotDate: string) {
  return useQuery({
    queryKey: ["report_balance_sheet", snapshotDate],
    queryFn: () => reportBalanceSheet(snapshotDate),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    enabled: snapshotDate.length > 0,
  });
}
