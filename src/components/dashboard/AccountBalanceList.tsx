import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { accountList, accountBalancesBatch } from "@/utils/api";
import { formatAmount } from "@/utils/format";
import {
  getLiabilityDisplayBalance,
  getLiabilityStatusKey,
} from "@/utils/liabilityBalance";
import { ChevronDown, ChevronUp, Wallet, CreditCard } from "lucide-react";
import type { AccountDto } from "@/types";

const ACCOUNT_TYPE_LABELS: Record<string, string> = {
  asset: "accounts.types.asset",
  liability: "accounts.types.liability",
};

const ACCOUNT_TYPE_ICONS: Record<
  string,
  React.ComponentType<{ className?: string }>
> = {
  asset: Wallet,
  liability: CreditCard,
};

export function AccountBalanceList() {
  const { t } = useTranslation();
  const [expandedTypes, setExpandedTypes] = useState<Set<string>>(
    new Set(["asset", "liability"]),
  );

  const {
    data: accounts,
    isLoading,
    error,
  } = useQuery({
    queryKey: ["accounts-for-dashboard"],
    queryFn: () => accountList(),
  });

  const filteredAccounts = (accounts ?? [])
    .filter(
      (acc) => acc.account_type === "asset" || acc.account_type === "liability",
    )
    .filter((acc) => !acc.is_system);

  const accountIds = filteredAccounts.map((acc) => acc.id);

  const { data: balancesData } = useQuery({
    queryKey: ["account-balances-batch", accountIds],
    queryFn: () => accountBalancesBatch(accountIds),
    enabled: accountIds.length > 0,
  });

  const balancesMap: Record<string, number> = {};
  if (balancesData) {
    for (const item of balancesData) {
      balancesMap[item.id] = item.balance;
    }
  }

  const groupedAccounts = filteredAccounts.reduce<Record<string, AccountDto[]>>(
    (acc, account) => {
      const type = account.account_type;
      if (!acc[type]) {
        acc[type] = [];
      }
      acc[type].push(account);
      return acc;
    },
    {},
  );

  const toggleType = (type: string) => {
    setExpandedTypes((prev) => {
      const next = new Set(prev);
      if (next.has(type)) {
        next.delete(type);
      } else {
        next.add(type);
      }
      return next;
    });
  };

  if (error) {
    return (
      <Card elevation="raise">
        <CardHeader>
          <CardTitle>{t("dashboard.accountBalanceList")}</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="text-destructive text-sm">
            {t("common.errorGeneric")}: {String(error)}
          </div>
        </CardContent>
      </Card>
    );
  }

  const hasAccounts = Object.keys(groupedAccounts).length > 0;

  return (
    <Card elevation="raise" className="flex min-h-[120px] flex-1 flex-col">
      <CardHeader>
        <CardTitle>{t("dashboard.accountBalanceList")}</CardTitle>
      </CardHeader>
      <CardContent className="min-h-0 flex-1 overflow-auto">
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="h-8 animate-pulse rounded-sm bg-muted" />
            ))}
          </div>
        ) : !hasAccounts ? (
          <p className="text-muted-foreground text-sm">
            {t("accounts.noAccountsOfType", {
              type: t("accounts.types.asset"),
            })}
          </p>
        ) : (
          <div className="space-y-2">
            {Object.entries(groupedAccounts).map(([type, accountsOfType]) => {
              const isExpanded = expandedTypes.has(type);
              const Icon = ACCOUNT_TYPE_ICONS[type] ?? Wallet;

              return (
                <div key={type}>
                  <Button
                    variant="ghost"
                    className="h-auto w-full justify-between px-1 py-1 text-xs font-medium text-foreground hover:bg-transparent"
                    onClick={() => toggleType(type)}
                  >
                    <div className="flex items-center gap-1.5">
                      <Icon className="size-3.5" strokeWidth={1.75} />
                      <span className="font-medium">
                        {t(ACCOUNT_TYPE_LABELS[type] ?? type)}
                      </span>
                      <span className="text-muted-foreground">
                        ({accountsOfType.length})
                      </span>
                    </div>
                    {isExpanded ? (
                      <ChevronUp className="size-3.5" />
                    ) : (
                      <ChevronDown className="size-3.5" />
                    )}
                  </Button>

                  {isExpanded && (
                    <div className="pl-1">
                      {accountsOfType.map((account) => {
                        const balance = balancesMap[account.id] ?? 0;
                        const displayBalance = getLiabilityDisplayBalance(
                          balance,
                          account.account_type,
                        );
                        const isAsset = account.account_type === "asset";
                        const balanceClass = isAsset
                          ? displayBalance >= 0
                            ? "text-income"
                            : "text-expense"
                          : displayBalance >= 0
                            ? "text-expense"
                            : "text-income";
                        const statusKey = !isAsset
                          ? getLiabilityStatusKey(displayBalance)
                          : null;

                        return (
                          <div
                            key={account.id}
                            className="flex items-center justify-between gap-2 border-b border-border/60 py-1.5 last:border-0"
                          >
                            <span className="truncate">{account.name}</span>
                            <div className="flex items-center gap-2">
                              <span
                                className={`font-mono font-medium ${balanceClass}`}
                              >
                                {formatAmount(displayBalance)}
                              </span>
                              {statusKey && (
                                <span className="text-xs text-muted-foreground">
                                  {t(statusKey)}
                                </span>
                              )}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
