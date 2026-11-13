import { useQuery } from "@tanstack/react-query";
import { reportAudit } from "@/utils/api";

export function useAuditReport() {
  return useQuery({
    queryKey: ["report_audit"],
    queryFn: reportAudit,
  });
}
