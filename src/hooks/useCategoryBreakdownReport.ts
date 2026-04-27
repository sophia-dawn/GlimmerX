import { useQuery } from "@tanstack/react-query";
import { reportCategoryBreakdown } from "@/utils/api";
import type { ReportFilter } from "@/types/report";

export function useCategoryBreakdownReport(
  filter: ReportFilter,
  incomeOrExpense: "income" | "expense",
) {
  return useQuery({
    queryKey: ["report_category_breakdown", filter, incomeOrExpense],
    queryFn: () => reportCategoryBreakdown(filter, incomeOrExpense),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
