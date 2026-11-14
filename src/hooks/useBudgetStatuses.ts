import { useQuery } from "@tanstack/react-query";
import { budgetListStatuses } from "@/utils/api";
import { useMemo } from "react";

/**
 * Budget status for a month. Defaults to the current one so the dashboard can
 * call it bare; the budget report passes the month the user picked, which is
 * why the parameters exist at all.
 */
export function useBudgetStatuses(year?: number, month?: number) {
  const params = useMemo(() => {
    const now = new Date();
    return {
      year: year ?? now.getFullYear(),
      month: month ?? now.getMonth() + 1,
    };
  }, [year, month]);

  return useQuery({
    queryKey: ["budget-statuses", params.year, params.month],
    queryFn: () => budgetListStatuses(params.year, params.month),
  });
}
