import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { aiParseTransaction } from "@/utils/api";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Sparkles, Loader2 } from "lucide-react";

interface AiInputDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSuccess?: () => void;
}

export function AiInputDialog({
  open,
  onOpenChange,
  onSuccess,
}: AiInputDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [text, setText] = useState("");
  const [submitError, setSubmitError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setText("");
      setSubmitError(null);
    }
  }, [open]);

  const mutation = useMutation({
    mutationFn: (input: string) => aiParseTransaction(input),
    onSuccess: () => {
      queryClient.invalidateQueries({
        predicate: (query) =>
          query.queryKey[0] === "transactionListPaginated" ||
          query.queryKey[0] === "transactionDetail",
      });
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      toast.success(t("ai.success"));
      onOpenChange(false);
      onSuccess?.();
    },
    onError: (err) => {
      setSubmitError(translateErrorMessage(err, t));
    },
  });

  const canSubmit = text.trim().length > 0 && !mutation.isPending;

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!canSubmit) return;
    mutation.mutate(text.trim());
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            {t("ai.title")}
          </DialogTitle>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="space-y-4">
            <Textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={t("ai.inputPlaceholder")}
              className="min-h-[80px] resize-none"
              autoFocus
              disabled={mutation.isPending}
            />

            {submitError && (
              <p className="text-sm text-destructive">{submitError}</p>
            )}
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={mutation.isPending}
            >
              {t("common.cancel")}
            </Button>
            <Button type="submit" disabled={!canSubmit}>
              {mutation.isPending ? (
                <>
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {t("ai.recognizing")}
                </>
              ) : (
                t("ai.button")
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
