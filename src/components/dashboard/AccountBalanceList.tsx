import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
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
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
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
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
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
      <Card>
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
    <Card>
      <CardHeader>
        <CardTitle>{t("dashboard.accountBalanceList")}</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3">
            {[...Array(3)].map((_, i) => (
              <div key={i} className="h-12 bg-muted rounded animate-pulse" />
            ))}
          </div>
        ) : !hasAccounts ? (
          <p className="text-muted-foreground text-sm">
            {t("accounts.noAccountsOfType", {
              type: t("accounts.types.asset"),
            })}
          </p>
        ) : (
          <div className="space-y-4">
            {Object.entries(groupedAccounts).map(([type, accountsOfType]) => {
              const isExpanded = expandedTypes.has(type);
              const Icon = ACCOUNT_TYPE_ICONS[type] ?? Wallet;

              return (
                <div key={type}>
                  <Button
                    variant="ghost"
                    className="w-full flex items-center justify-between p-2"
                    onClick={() => toggleType(type)}
                  >
                    <div className="flex items-center gap-2">
                      <Icon className="h-4 w-4" />
                      <span className="font-medium">
                        {t(ACCOUNT_TYPE_LABELS[type] ?? type)}
                      </span>
                      <span className="text-muted-foreground text-sm">
                        ({accountsOfType.length})
                      </span>
                    </div>
                    {isExpanded ? (
                      <ChevronUp className="h-4 w-4" />
                    ) : (
                      <ChevronDown className="h-4 w-4" />
                    )}
                  </Button>

                  {isExpanded && (
                    <div className="mt-2 space-y-2 pl-4">
                      {accountsOfType.map((account) => {
                        const balance = balancesMap[account.id] ?? 0;
                        const displayBalance = getLiabilityDisplayBalance(
                          balance,
                          account.account_type,
                        );
                        const isAsset = account.account_type === "asset";
                        const balanceClass = isAsset
                          ? displayBalance >= 0
                            ? "text-green-600"
                            : "text-red-600"
                          : displayBalance >= 0
                            ? "text-red-600"
                            : "text-green-600";
                        const statusKey = !isAsset
                          ? getLiabilityStatusKey(displayBalance)
                          : null;

                        return (
                          <Link
                            key={account.id}
                            to={`/accounts/${account.id}`}
                            className="flex items-center justify-between p-2 rounded-lg bg-muted/50 hover:bg-muted transition-colors group"
                          >
                            <span className="font-medium truncate">
                              {account.name}
                            </span>
                            <div className="flex items-center gap-2">
                              <span className={`font-medium ${balanceClass}`}>
                                {formatAmount(displayBalance)}
                              </span>
                              {statusKey && (
                                <span className="text-xs text-muted-foreground">
                                  {t(statusKey)}
                                </span>
                              )}
                              <ChevronDown className="h-4 w-4 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity rotate-[-90deg]" />
                            </div>
                          </Link>
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
