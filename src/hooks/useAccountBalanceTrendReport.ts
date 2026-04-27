import { useQuery } from "@tanstack/react-query";
import { reportAccountBalanceTrend } from "@/utils/api";
import type { ReportFilter } from "@/types/report";

export function useAccountBalanceTrendReport(
  accountId: string,
  filter: ReportFilter,
) {
  return useQuery({
    queryKey: ["report_account_balance_trend", accountId, filter],
    queryFn: () => reportAccountBalanceTrend(accountId, filter),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    enabled: accountId.length > 0,
  });
}
