import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import type { AccountTransaction } from "@/types";
import { accountTransactions } from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { formatAmount } from "@/utils/format";
import { Button } from "@/components/ui/button";
import { DatePicker } from "@/components/ui/date-picker";
import { Label } from "@/components/ui/label";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Calendar as CalendarIcon } from "lucide-react";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface AccountTransactionsProps {
  accountId: string;
  accountName: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AccountTransactions({
  accountId,
  accountName,
  open,
  onOpenChange,
}: AccountTransactionsProps) {
  const { t } = useTranslation();
  const [fromDate, setFromDate] = useState("");
  const [toDate, setToDate] = useState("");

  const {
    data: transactions = [],
    isLoading,
    refetch,
  } = useQuery({
    queryKey: ["account-transactions", accountId, fromDate, toDate],
    queryFn: () =>
      accountTransactions(
        accountId,
        fromDate || undefined,
        toDate || undefined,
      ),
    enabled: open,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const handleFilter = () => {
    refetch();
  };

  const handleClear = () => {
    setFromDate("");
    setToDate("");
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("accounts.transactions.title")}</DialogTitle>
          <DialogDescription>
            {t("accounts.transactions.forAccount", { name: accountName })}
          </DialogDescription>
        </DialogHeader>

        {/* Date range filter */}
        <div className="flex items-end gap-3">
          <div className="flex-1 space-y-1">
            <Label>{t("accounts.transactions.fromDate")}</Label>
            <DatePicker
              value={fromDate}
              onChange={setFromDate}
              placeholder={t("accounts.transactions.fromDate")}
            />
          </div>
          <div className="flex-1 space-y-1">
            <Label>{t("accounts.transactions.toDate")}</Label>
            <DatePicker
              value={toDate}
              onChange={setToDate}
              placeholder={t("accounts.transactions.toDate")}
            />
          </div>
          <Button variant="secondary" size="sm" onClick={handleFilter}>
            <CalendarIcon className="mr-1 h-4 w-4" />
            {t("accounts.transactions.filter")}
          </Button>
          <Button variant="ghost" size="sm" onClick={handleClear}>
            {t("accounts.transactions.clear")}
          </Button>
        </div>

        {/* Transaction list */}
        <div className="max-h-[50vh] overflow-y-auto">
          {isLoading && (
            <div className="py-8 text-center text-sm text-muted-foreground">
              {t("common.loading")}
            </div>
          )}

          {!isLoading && transactions.length === 0 && (
            <div className="flex flex-col items-center justify-center py-8">
              <p className="text-sm text-muted-foreground">
                {t("accounts.transactions.noTransactions")}
              </p>
            </div>
          )}

          {!isLoading && transactions.length > 0 && (
            <div className="space-y-1">
              {transactions.map((tx: AccountTransaction) => (
                <TransactionRow key={tx.id} tx={tx} />
              ))}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("common.close")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ---------------------------------------------------------------------------
// TransactionRow
// ---------------------------------------------------------------------------

interface TransactionRowProps {
  tx: AccountTransaction;
}

function TransactionRow({ tx }: TransactionRowProps) {
  const displayDate = new Date(`${tx.date}T00:00:00`).toLocaleDateString();

  return (
    <div className="flex items-center justify-between rounded-md px-3 py-2.5 hover:bg-accent">
      <div className="min-w-0 flex-1">
        <div className="text-sm font-medium">{tx.description}</div>
        <div className="text-xs text-muted-foreground">{displayDate}</div>
      </div>
      <span
        className={`ml-4 text-sm font-medium tabular-nums ${tx.amount >= 0 ? "text-green-600" : "text-red-600"}`}
      >
        {formatAmount(tx.amount)}
      </span>
    </div>
  );
}
