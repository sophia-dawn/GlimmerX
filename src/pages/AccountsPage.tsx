import { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import type { AccountDto } from "@/types";
import {
  accountList,
  accountBalance,
  accountDelete as apiAccountDelete,
} from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { formatAmount } from "@/utils/format";
import { getAccountDisplayName } from "@/utils/accountDisplay";
import {
  getLiabilityDisplayBalance,
  getLiabilityStatusKey,
} from "@/utils/liabilityBalance";
import { translateErrorMessage } from "@/utils/errorTranslation";
import { AccountForm } from "@/components/accounts/AccountForm";
import { AccountWizard } from "@/components/accounts/AccountWizard";
import { AccountTransactions } from "@/components/accounts/AccountTransactions";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
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
import {
  Plus,
  Wallet,
  CreditCard,
  TrendingUp,
  TrendingDown,
  Edit,
  Trash2,
} from "lucide-react";
import { useHeaderActions } from "@/contexts/HeaderContext";

// ---------------------------------------------------------------------------
// Constants (icons only — labels come from i18n)
// ---------------------------------------------------------------------------

const TAB_ICONS: Record<string, typeof Wallet | null> = {
  all: null,
  asset: Wallet,
  liability: CreditCard,
  income: TrendingUp,
  expense: TrendingDown,
};

const TAB_KEYS = Object.keys(TAB_ICONS) as string[];

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AccountsPage() {
  const { t } = useTranslation();
  const { setActions } = useHeaderActions();
  const [formOpen, setFormOpen] = useState(false);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [editNode, setEditNode] = useState<AccountDto | null>(null);
  const [deleteNode, setDeleteNode] = useState<AccountDto | null>(null);
  const [transactionsNode, setTransactionsNode] = useState<AccountDto | null>(
    null,
  );
  const [searchQuery, setSearchQuery] = useState("");

  const {
    data: allAccounts = [],
    refetch,
    isLoading,
  } = useQuery({
    queryKey: ["accounts"],
    queryFn: accountList,
    ...QUERY_CONFIG.FINANCIAL,
  });

  const headerActions = useMemo(
    () => (
      <>
        <Input
          placeholder={t("accounts.searchPlaceholder")}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="max-w-xs h-8"
        />
        <Button size="sm" onClick={() => setWizardOpen(true)}>
          <Plus className="mr-1 h-4 w-4" />
          {t("accounts.newAccount")}
        </Button>
      </>
    ),
    [t, searchQuery],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  const handleEdit = (account: AccountDto) => {
    setEditNode(account);
    setFormOpen(true);
  };

  const handleDelete = (account: AccountDto) => {
    setDeleteNode(account);
  };

  const handleViewTransactions = (account: AccountDto) => {
    setTransactionsNode(account);
  };

  const confirmDeleteAccount = async () => {
    if (!deleteNode) return;

    try {
      await apiAccountDelete(deleteNode.id);
      toast.success(t("accounts.accountDeleted"));
      refetch();
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setDeleteNode(null);
    }
  };

  const handleFormSuccess = () => {
    refetch();
    setEditNode(null);
  };

  const handleDialogChange = (open: boolean) => {
    setFormOpen(open);
    if (!open) {
      setEditNode(null);
    }
  };

  // Filter based on tab and search query
  const filterAccounts = (accountsList: AccountDto[]): AccountDto[] => {
    let filtered = accountsList;

    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      filtered = filtered.filter((a) => a.name.toLowerCase().includes(q));
    }

    return filtered;
  };

  // Filtered accounts for current tab

  const filteredAccounts = filterAccounts(allAccounts);

  const hasAccounts = allAccounts.length > 0;

  return (
    <div className="h-full overflow-auto p-6">
      {isLoading && (
        <div className="py-12 text-center text-muted-foreground">
          {t("common.loading")}
        </div>
      )}

      {/* Empty state */}
      {!isLoading && !hasAccounts && (
        <div className="flex flex-col items-center justify-center py-16">
          <div className="mb-6 text-4xl">💰</div>
          <h2 className="mb-2 text-lg font-medium">
            {t("accounts.setupYourAccounts")}
          </h2>
          <p className="mb-6 text-sm text-muted-foreground">
            {t("accounts.setupYourAccountsDesc")}
          </p>
          <Button variant="default" onClick={() => setWizardOpen(true)}>
            {t("accounts.addAccount")}
          </Button>
        </div>
      )}

      {/* Account list with tabs */}
      {!isLoading && hasAccounts && (
        <Tabs defaultValue="all">
          <TabsList className="mb-4 h-auto flex-wrap gap-1 bg-transparent p-0">
            {TAB_KEYS.map((key) => {
              const Icon = TAB_ICONS[key];
              return (
                <TabsTrigger
                  key={key}
                  value={key}
                  className="data-[state=active]:bg-background"
                >
                  {Icon && <Icon className="mr-1 h-3.5 w-3.5" />}
                  {t(`accounts.types.${key}`)}
                </TabsTrigger>
              );
            })}
          </TabsList>

          {TAB_KEYS.map((key) => {
            const filtered =
              key === "all"
                ? filteredAccounts
                : filteredAccounts.filter((a) => a.account_type === key);

            return (
              <TabsContent key={key} value={key} className="mt-0">
                {filtered.length === 0 ? (
                  <div className="py-8 text-center text-sm text-muted-foreground">
                    {t("accounts.noAccountsOfType", {
                      type: t(`accounts.types.${key}`),
                    })}
                  </div>
                ) : (
                  <div className="space-y-0.5">
                    {filtered.map((account) => (
                      <AccountRow
                        key={account.id}
                        account={account}
                        onEdit={handleEdit}
                        onDelete={handleDelete}
                        onViewTransactions={handleViewTransactions}
                      />
                    ))}
                  </div>
                )}
              </TabsContent>
            );
          })}
        </Tabs>
      )}

      {/* Create/Edit form dialog */}
      <AccountForm
        open={formOpen}
        onOpenChange={handleDialogChange}
        editNode={editNode}
        onSuccess={handleFormSuccess}
      />

      {/* Create account wizard */}
      <AccountWizard
        open={wizardOpen}
        onOpenChange={setWizardOpen}
        onSuccess={refetch}
      />

      {/* Delete confirmation dialog */}
      <AlertDialog
        open={!!deleteNode}
        onOpenChange={(open) => {
          if (!open) setDeleteNode(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("accounts.confirmDeleteTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("accounts.confirmDeleteDesc", {
                name: deleteNode ? getAccountDisplayName(deleteNode) : "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDeleteAccount}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              {t("accounts.deleteAccount")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Transaction history dialog */}
      {transactionsNode && (
        <AccountTransactions
          accountId={transactionsNode.id}
          accountName={getAccountDisplayName(transactionsNode)}
          open={!!transactionsNode}
          onOpenChange={(open) => {
            if (!open) setTransactionsNode(null);
          }}
        />
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// AccountRow
// ---------------------------------------------------------------------------

interface AccountRowProps {
  account: AccountDto;
  onEdit: (account: AccountDto) => void;
  onDelete: (account: AccountDto) => void;
  onViewTransactions: (account: AccountDto) => void;
}

function AccountRow({
  account,
  onEdit,
  onDelete,
  onViewTransactions,
}: AccountRowProps) {
  const { t } = useTranslation();

  const { data: balance } = useQuery({
    queryKey: ["account-balance", account.id],
    queryFn: () => accountBalance(account.id),
    enabled:
      account.account_type === "asset" || account.account_type === "liability",
    staleTime: 0,
  });
  return (
    <div
      className={`group flex items-center gap-2 rounded-md px-3 py-2.5 hover:bg-accent cursor-pointer`}
      onClick={() => onViewTransactions(account)}
    >
      {/* Type and subtype badges */}
      <div className="flex items-center gap-1">
        <span className="rounded bg-muted px-1.5 py-0.5 text-xs text-muted-foreground">
          {t(`accounts.types.${account.account_type}`)}
        </span>
        {(() => {
          const accountRole =
            account.meta?.find((m) => m.key === "account_role")?.value ?? "";
          const liabilityType =
            account.meta?.find((m) => m.key === "liability_type")?.value ?? "";
          const subtypeValue = accountRole || liabilityType;
          return subtypeValue ? (
            <span className="rounded bg-primary/10 px-1.5 py-0.5 text-xs text-primary">
              {t(`accounts.subtypes.${subtypeValue}`)}
            </span>
          ) : null;
        })()}
      </div>

      {/* Name */}
      <span className="min-w-0 flex-1 truncate text-sm font-medium">
        {account.name}
      </span>

      {(account.account_type === "asset" ||
        account.account_type === "liability") && (
        <div className="flex items-center gap-1">
          <span
            className={`text-sm font-medium tabular-nums ${
              account.account_type === "asset"
                ? balance !== undefined && balance >= 0
                  ? "text-green-600"
                  : "text-red-600"
                : (() => {
                    const displayBalance = getLiabilityDisplayBalance(
                      balance ?? 0,
                      account.account_type,
                    );
                    return displayBalance >= 0
                      ? "text-red-600"
                      : "text-green-600";
                  })()
            }`}
          >
            {balance !== undefined
              ? formatAmount(
                  getLiabilityDisplayBalance(balance, account.account_type),
                )
              : "..."}
          </span>
          {account.account_type === "liability" &&
            balance !== undefined &&
            (() => {
              const displayBalance = getLiabilityDisplayBalance(
                balance,
                account.account_type,
              );
              const statusKey = getLiabilityStatusKey(displayBalance);
              return statusKey ? (
                <span className="text-xs text-muted-foreground">
                  {t(statusKey)}
                </span>
              ) : null;
            })()}
        </div>
      )}

      {/* Account number */}
      {account.account_number && (
        <span className="text-xs text-muted-foreground font-mono">
          {account.account_number}
        </span>
      )}

      {/* Action buttons */}
      <div className="flex gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={(e) => {
            e.stopPropagation();
            onEdit(account);
          }}
        >
          <Edit className="h-3 w-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 text-destructive"
          onClick={(e) => {
            e.stopPropagation();
            onDelete(account);
          }}
        >
          <Trash2 className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}
