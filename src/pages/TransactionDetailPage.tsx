import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useParams, useNavigate } from "react-router-dom";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  ArrowLeft,
  Pencil,
  Trash2,
  CheckCircle2,
  AlertCircle,
} from "lucide-react";
import {
  transactionDetail,
  transactionUpdate,
  accountList,
  categoryList,
} from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { TransactionDeleteDialog } from "@/components/transactions/TransactionDeleteDialog";
import { TransactionForm } from "@/components/transactions/TransactionForm";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { formatAmount } from "@/utils/format";
import { formatDateLong, formatDateTimeShort } from "@/utils/date";
import type { PostingDetail, CreateTransactionInput } from "@/types";

export function TransactionDetailPage() {
  const { t } = useTranslation();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editDialogOpen, setEditDialogOpen] = useState(false);

  console.log("[TransactionDetailPage] render, id:", id);

  const {
    data: transaction,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["transactionDetail", id],
    queryFn: async () => {
      console.log(
        "[TransactionDetailPage] fetching transactionDetail, id:",
        id,
      );
      const result = await transactionDetail(id ?? "");
      console.log("[TransactionDetailPage] transactionDetail result:", result);
      return result;
    },
    enabled: !!id,
    ...QUERY_CONFIG.FINANCIAL,
  });

  console.log("[TransactionDetailPage] query state:", {
    isLoading,
    isError: !!error,
    error: error?.message ?? error,
    hasData: !!transaction,
  });

  const { data: accounts } = useQuery({
    queryKey: ["accountList"],
    queryFn: () => accountList(),
    ...QUERY_CONFIG.FINANCIAL,
  });

  const { data: categories } = useQuery({
    queryKey: ["categoryList"],
    queryFn: () => categoryList(),
    ...QUERY_CONFIG.FINANCIAL,
  });

  const updateMutation = useMutation({
    mutationFn: (input: CreateTransactionInput) =>
      transactionUpdate(id ?? "", {
        date: input.date,
        description: input.description,
        categoryId: input.categoryId,
        postings: input.postings,
      }),
    onSuccess: () => {
      toast.success(t("transactions.updated"));
      queryClient.invalidateQueries({ queryKey: ["transactionDetail", id] });
      queryClient.invalidateQueries({ queryKey: ["transactionListPaginated"] });
      queryClient.invalidateQueries({ queryKey: ["accounts"] });
      setEditDialogOpen(false);
    },
    onError: (err: unknown) => {
      toast.error(translateErrorMessage(err, t));
    },
  });

  const handleBack = () => {
    navigate("/transactions");
  };

  const handleDeleteSuccess = () => {
    navigate("/transactions");
  };

  const getAccountTypeIcon = (type: string) => {
    switch (type) {
      case "asset":
        return "🏦";
      case "liability":
        return "💳";
      case "income":
        return "💰";
      case "expense":
        return "💸";
      case "equity":
        return "⚖️";
      default:
        return "📊";
    }
  };

  const renderPosting = (posting: PostingDetail) => {
    const amountColor = posting.isDebit
      ? "text-emerald-600 dark:text-emerald-400"
      : "text-rose-600 dark:text-rose-400";

    return (
      <div
        key={posting.id}
        className="flex items-center justify-between py-3 border-b last:border-b-0"
      >
        <div className="flex items-center gap-3">
          <span className="text-lg">
            {getAccountTypeIcon(posting.accountType)}
          </span>
          <div>
            <div className="font-medium">{posting.accountName}</div>
            <div className="text-xs text-muted-foreground">
              {t(`accounts.types.${posting.accountType}`)}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Badge
            variant={posting.isDebit ? "outline" : "secondary"}
            className="text-xs"
          >
            {posting.isDebit
              ? t("transactions.detail.debit")
              : t("transactions.detail.credit")}
          </Badge>
          <span className={`font-semibold tabular-nums ${amountColor}`}>
            {posting.amountDisplay}
          </span>
        </div>
      </div>
    );
  };

  if (isLoading) {
    return (
      <div className="h-full flex flex-col">
        <div className="px-6 py-4 border-b flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t("common.back")}
          </Button>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-muted-foreground">{t("common.loading")}</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex flex-col">
        <div className="px-6 py-4 border-b flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t("common.back")}
          </Button>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-destructive">{translateErrorMessage(error, t)}</p>
        </div>
      </div>
    );
  }

  if (!transaction) {
    return (
      <div className="h-full flex flex-col">
        <div className="px-6 py-4 border-b flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t("common.back")}
          </Button>
        </div>
        <div className="flex-1 flex items-center justify-center">
          <p className="text-muted-foreground">
            {t("transactions.detail.notFound")}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <div className="px-6 py-4 border-b flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="sm" onClick={handleBack}>
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t("common.back")}
          </Button>
          <div>
            <h1 className="text-xl font-semibold">
              {transaction.description || t("transactions.noDescription")}
            </h1>
            <p className="text-sm text-muted-foreground">
              {formatDateLong(transaction.date)}
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setEditDialogOpen(true)}
          >
            <Pencil className="h-4 w-4 mr-1" />
            {t("common.edit")}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setDeleteDialogOpen(true)}
          >
            <Trash2 className="h-4 w-4 mr-1" />
            {t("common.delete")}
          </Button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
        <div className="flex items-center gap-2">
          {transaction.isBalanced ? (
            <Badge
              variant="outline"
              className="gap-1 text-emerald-600 dark:text-emerald-400 border-emerald-600 dark:border-emerald-400"
            >
              <CheckCircle2 className="h-3 w-3" />
              {t("transactions.balanced")}
            </Badge>
          ) : (
            <Badge
              variant="outline"
              className="gap-1 text-amber-600 dark:text-amber-400 border-amber-600 dark:border-amber-400"
            >
              <AlertCircle className="h-3 w-3" />
              {t("transactions.unbalanced")}
            </Badge>
          )}
        </div>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">
              {t("transactions.description")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm">
              {transaction.description || t("transactions.noDescription")}
            </p>
          </CardContent>
        </Card>

        {transaction.categoryName && (
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm">
                {t("transactions.category")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              <Badge variant="secondary">{transaction.categoryName}</Badge>
            </CardContent>
          </Card>
        )}

        <Card>
          <CardHeader>
            <CardTitle>{t("transactions.postings")}</CardTitle>
            <CardDescription>
              {t("transactions.detail.postingCount", {
                count: transaction.postingCount,
              })}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {transaction.postings.map((posting) => renderPosting(posting))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">
              {t("transactions.detail.totals")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex justify-between items-center py-2 border-b">
              <span className="text-muted-foreground">
                {t("transactions.detail.debitTotal")}
              </span>
              <span className="font-semibold tabular-nums text-emerald-600 dark:text-emerald-400">
                {formatAmount(transaction.debitTotal)}
              </span>
            </div>
            <div className="flex justify-between items-center py-2">
              <span className="text-muted-foreground">
                {t("transactions.detail.creditTotal")}
              </span>
              <span className="font-semibold tabular-nums text-rose-600 dark:text-rose-400">
                {formatAmount(transaction.creditTotal)}
              </span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm">
              {t("transactions.detail.metadata")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-sm">
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("transactions.createdAt")}
                </span>
                <span>{formatDateTimeShort(transaction.createdAt)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">
                  {t("transactions.detail.updatedAt")}
                </span>
                <span>{formatDateTimeShort(transaction.updatedAt)}</span>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {id && (
        <TransactionDeleteDialog
          transactionId={id}
          open={deleteDialogOpen}
          onOpenChange={setDeleteDialogOpen}
          onSuccess={handleDeleteSuccess}
        />
      )}

      {accounts && categories && transaction && (
        <TransactionForm
          open={editDialogOpen}
          onOpenChange={setEditDialogOpen}
          mode="edit"
          transaction={{
            id: transaction.id,
            date: transaction.date,
            description: transaction.description,
            categoryId: transaction.categoryId,
            isReconciled: false,
            createdAt: transaction.createdAt,
            updatedAt: transaction.updatedAt,
            postings: transaction.postings.map((p) => ({
              id: p.id,
              transactionId: p.transactionId,
              accountId: p.accountId,
              amount: p.amount,
              sequence: p.sequence ?? 0,
              createdAt: p.createdAt,
            })),
          }}
          accounts={accounts}
          categories={categories}
          onSubmit={(data) => updateMutation.mutate(data)}
          isLoading={updateMutation.isPending}
        />
      )}
    </div>
  );
}
