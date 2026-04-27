import { useQuery } from "@tanstack/react-query";
import { budgetListStatuses } from "@/utils/api";
import { useMemo } from "react";

export function useBudgetList() {
  const params = useMemo(() => {
    const now = new Date();
    return {
      year: now.getFullYear(),
      month: now.getMonth() + 1,
    };
  }, []);

  return useQuery({
    queryKey: ["budget-list", params.year, params.month],
    queryFn: () => budgetListStatuses(params.year, params.month),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
