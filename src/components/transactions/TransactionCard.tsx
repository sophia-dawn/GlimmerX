import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { Pencil, Trash2, ArrowRight } from "lucide-react";
import type { TransactionListItem as TransactionListItemType } from "@/types";
import { formatAmount } from "@/utils/format";
import { formatDateShort, formatDateTimeShort } from "@/utils/date";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface TransactionCardProps {
  transaction: TransactionListItemType;
  onEdit?: (transaction: TransactionListItemType) => void;
  onDelete?: (transaction: TransactionListItemType) => void;
}

export function TransactionCard({
  transaction,
  onEdit,
  onDelete,
}: TransactionCardProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [showActions, setShowActions] = useState(false);

  const categoryDisplay = transaction.categoryIcon
    ? `${transaction.categoryIcon} ${transaction.categoryName}`
    : transaction.categoryName;

  return (
    <div
      className={cn(
        "flex items-center justify-between rounded-lg px-3 py-2.5",
        "hover:bg-accent transition-colors cursor-pointer",
      )}
      onMouseEnter={() => setShowActions(true)}
      onMouseLeave={() => setShowActions(false)}
      onClick={() => navigate(`/transactions/${transaction.id}`)}
    >
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">
            <span className="font-medium">{t("transactions.dateLabel")}</span>{" "}
            {formatDateShort(transaction.date)}
          </span>
          <span
            className="text-sm font-medium truncate max-w-[300px]"
            title={transaction.description}
          >
            {transaction.description || "-"}
          </span>
          <span className="text-xs text-muted-foreground">
            <span className="font-medium">
              {t("transactions.createdAtLabel")}
            </span>{" "}
            {formatDateTimeShort(transaction.createdAt)}
          </span>
        </div>

        <div className="flex items-center gap-2 mt-1.5">
          {categoryDisplay && (
            <Badge
              variant="secondary"
              className="text-xs truncate max-w-[100px]"
              title={categoryDisplay}
            >
              {categoryDisplay}
            </Badge>
          )}
          {transaction.postingsSummary.length > 0 && (
            <div className="flex items-center gap-2 text-sm">
              <div className="flex flex-col gap-0.5">
                {transaction.postingsSummary
                  .filter((p) => p.amount < 0)
                  .map((p, idx) => (
                    <Badge
                      key={idx}
                      variant="outline"
                      className="text-xs font-normal tabular-nums text-red-500"
                      title={p.accountName}
                    >
                      {p.accountName} {formatAmount(p.amount)}
                    </Badge>
                  ))}
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
              <div className="flex flex-col gap-0.5">
                {transaction.postingsSummary
                  .filter((p) => p.amount >= 0)
                  .map((p, idx) => (
                    <Badge
                      key={idx}
                      variant="outline"
                      className="text-xs font-normal tabular-nums text-green-500"
                      title={p.accountName}
                    >
                      {p.accountName} {formatAmount(p.amount)}
                    </Badge>
                  ))}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <span
          className={cn(
            "text-sm font-medium tabular-nums",
            transaction.displayAmount >= 0 ? "text-green-600" : "text-red-600",
          )}
        >
          {formatAmount(transaction.displayAmount)}
        </span>

        {showActions && (onEdit || onDelete) && (
          <div className="flex gap-1">
            {onEdit && (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onEdit(transaction);
                }}
              >
                <Pencil className="h-3.5 w-3.5" />
              </Button>
            )}
            {onDelete && (
              <Button
                variant="ghost"
                size="sm"
                onClick={(e) => {
                  e.stopPropagation();
                  onDelete(transaction);
                }}
              >
                <Trash2 className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
