import { useQuery } from "@tanstack/react-query";
import { dashboardCategoryBreakdown } from "@/utils/api";
import { useMemo } from "react";

export function useCategoryBreakdown(
  year?: number,
  month?: number,
  categoryType?: "income" | "expense",
) {
  const params = useMemo(() => {
    const now = new Date();
    return {
      year: year ?? now.getFullYear(),
      month: month ?? now.getMonth() + 1,
      categoryType: categoryType ?? "expense",
    };
  }, [year, month, categoryType]);

  return useQuery({
    queryKey: [
      "dashboard-category-breakdown",
      params.year,
      params.month,
      params.categoryType,
    ],
    queryFn: () => dashboardCategoryBreakdown(params),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
