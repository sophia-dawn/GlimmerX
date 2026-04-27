import { useQuery } from "@tanstack/react-query";
import { accountList } from "@/utils/api";
import type { AccountDto } from "@/types";

export function useAccountList() {
  return useQuery<AccountDto[]>({
    queryKey: ["accounts"],
    queryFn: accountList,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });
}
