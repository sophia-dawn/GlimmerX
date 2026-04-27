import { useQuery } from "@tanstack/react-query";
import { reportAccountTransactions } from "@/utils/api";
import type { ReportFilter } from "@/types/report";

export function useAccountTransactionsReport(
  accountId: string,
  filter: ReportFilter,
  page: number = 1,
  pageSize: number = 50,
) {
  return useQuery({
    queryKey: [
      "report_account_transactions",
      accountId,
      filter,
      page,
      pageSize,
    ],
    queryFn: () => reportAccountTransactions(accountId, filter, page, pageSize),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
    enabled: accountId.length > 0,
  });
}
