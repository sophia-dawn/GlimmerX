import { useTranslation } from "react-i18next";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { transactionDeletePreview, transactionDelete } from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { formatDateShort } from "@/utils/date";
import { Badge } from "@/components/ui/badge";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { AlertTriangle, Trash2 } from "lucide-react";

interface TransactionDeleteDialogProps {
  transactionId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function TransactionDeleteDialog({
  transactionId,
  open,
  onOpenChange,
  onSuccess,
}: TransactionDeleteDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const {
    data: preview,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["transactionDeletePreview", transactionId],
    queryFn: () => transactionDeletePreview(transactionId),
    enabled: open,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const mutation = useMutation({
    onMutate: () => {
      queryClient.cancelQueries({ queryKey: ["transactionListPaginated"] });
    },
    mutationFn: () => transactionDelete(transactionId),
    onSuccess: () => {
      queryClient.removeQueries({
        queryKey: ["transactionDetail", transactionId],
      });
      queryClient.invalidateQueries({ queryKey: ["transactionListPaginated"] });
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      onOpenChange(false);
      onSuccess?.();
    },
    onError: (err) => {
      console.error("Delete error:", translateErrorMessage(err, t));
    },
  });

  const handleDelete = () => {
    mutation.mutate();
  };

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("transactions.delete.title")}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3">
              {isLoading && (
                <p className="text-muted-foreground">{t("common.loading")}</p>
              )}

              {error && (
                <p className="text-destructive">
                  {translateErrorMessage(error, t)}
                </p>
              )}

              {preview && (
                <>
                  <div className="rounded-md border p-3 space-y-2">
                    <div className="flex justify-between items-start">
                      <span className="font-medium">
                        {preview.description || t("transactions.noDescription")}
                      </span>
                      <Badge variant="outline" className="text-xs">
                        {formatDateShort(preview.date)}
                      </Badge>
                    </div>
                    <div className="text-sm text-muted-foreground">
                      {t("transactions.delete.postingCount", {
                        count: preview.postingCount,
                      })}
                    </div>
                  </div>

                  {preview.warningMessage && (
                    <div className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-500">
                      <AlertTriangle className="h-4 w-4" />
                      <span>{preview.warningMessage}</span>
                    </div>
                  )}

                  {!preview.canDelete && (
                    <div className="flex items-center gap-2 text-sm text-destructive">
                      <AlertTriangle className="h-4 w-4" />
                      <span>{t("transactions.delete.cannotDelete")}</span>
                    </div>
                  )}

                  {preview.canDelete && (
                    <p className="text-sm text-muted-foreground">
                      {t("transactions.delete.confirmWarning")}
                    </p>
                  )}
                </>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={mutation.isPending}>
            {t("common.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            onClick={handleDelete}
            disabled={mutation.isPending || isLoading || !preview?.canDelete}
          >
            {mutation.isPending ? (
              t("common.processing")
            ) : (
              <>
                <Trash2 className="h-4 w-4 mr-1" />
                {t("common.delete")}
              </>
            )}
          </AlertDialogAction>
        </AlertDialogFooter>

        {mutation.error && (
          <p className="text-sm text-destructive px-6 pb-4">
            {translateErrorMessage(mutation.error, t)}
          </p>
        )}
      </AlertDialogContent>
    </AlertDialog>
  );
}
