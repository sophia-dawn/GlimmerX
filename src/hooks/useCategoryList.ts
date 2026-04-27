import { useQuery } from "@tanstack/react-query";
import { categoryList } from "@/utils/api";
import type { Category } from "@/types";

export function useCategoryList(type?: "income" | "expense") {
  return useQuery<Category[]>({
    queryKey: ["categoryList", type],
    queryFn: () => categoryList(type),
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
