import { useState, useCallback, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  useInfiniteQuery,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import { toast } from "sonner";
import { Plus, Search } from "lucide-react";
import {
  transactionListPaginated,
  transactionCreate,
  accountList,
  categoryList,
} from "@/utils/api";
import { QUERY_CONFIG } from "@/constants/query";
import { translateErrorMessage } from "@/utils/errorTranslation";
import type {
  CreateTransactionInput,
  EnhancedTransactionFilter,
  TransactionDateGroup,
  TransactionListResponse,
} from "@/types";
import { TransactionFilterPanel } from "@/components/transactions/TransactionFilterPanel";
import { VirtualTransactionList } from "@/components/transactions/VirtualTransactionList";
import { flattenToVirtualItems } from "@/utils/virtualList";
import { TransactionForm } from "@/components/transactions/TransactionForm";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { useHeaderActions } from "@/contexts/HeaderContext";

/** Merges transaction date groups from multiple paginated responses,
 * combining items for the same date and preserving sort order. */
function mergeDateGroups(
  pages: TransactionListResponse[],
): TransactionDateGroup[] {
  const groupMap = new Map<string, TransactionDateGroup>();

  for (const page of pages) {
    for (const group of page.dateGroups) {
      const existing = groupMap.get(group.date);
      if (existing) {
        existing.items.push(...group.items);
        existing.dayTotal += group.dayTotal;
      } else {
        groupMap.set(group.date, { ...group, items: [...group.items] });
      }
    }
  }

  return Array.from(groupMap.values()).sort((a, b) =>
    b.date.localeCompare(a.date),
  );
}

export function TransactionsPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const { setActions } = useHeaderActions();
  const [formOpen, setFormOpen] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const headerActions = useMemo(
    () => (
      <Button size="sm" onClick={() => setFormOpen(true)}>
        <Plus className="h-4 w-4 mr-1" />
        {t("transactions.create")}
      </Button>
    ),
    [t],
  );

  useEffect(() => {
    setActions(headerActions);
    return () => setActions(null);
  }, [headerActions, setActions]);

  // Filter state (without page - page managed by infinite query)
  const [searchQuery, setSearchQuery] = useState("");
  const [filter, setFilter] = useState<EnhancedTransactionFilter>({
    pageSize: 20,
    sortBy: "date",
    sortOrder: "desc",
  });

  // Load accounts and categories for filter panel
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

  // Infinite query for transactions
  const infiniteQuery = useInfiniteQuery({
    queryKey: [
      "transactionListPaginated",
      { ...filter, descriptionQuery: searchQuery },
    ],
    queryFn: ({ pageParam = 1 }) =>
      transactionListPaginated({
        ...filter,
        descriptionQuery: searchQuery || undefined,
        page: pageParam,
      }),
    getNextPageParam: (lastPage) =>
      lastPage.pagination.hasNext ? lastPage.pagination.page + 1 : undefined,
    initialPageParam: 1,
    staleTime: 0,
    gcTime: 0,
    refetchOnMount: "always",
  });

  // Extract and merge data
  const pages = infiniteQuery.data?.pages ?? [];
  const allDateGroups = mergeDateGroups(pages);
  const flattenedItems = flattenToVirtualItems(allDateGroups);

  // Handlers
  const handleFilterChange = useCallback(
    (newFilter: Partial<EnhancedTransactionFilter>) => {
      setFilter((prev) => ({ ...prev, ...newFilter }));
    },
    [],
  );

  const handleClearFilter = useCallback(() => {
    setFilter({ pageSize: 20, sortBy: "date", sortOrder: "desc" });
    setSearchQuery("");
  }, []);

  const handleSearchChange = useCallback((query: string) => {
    setSearchQuery(query);
  }, []);

  const handleFormSubmit = async (data: CreateTransactionInput) => {
    setIsSubmitting(true);
    try {
      await transactionCreate(data);
      toast.success(t("transactions.created"));
      setFormOpen(false);
      queryClient.invalidateQueries({ queryKey: ["transactionListPaginated"] });
    } catch (err: unknown) {
      toast.error(translateErrorMessage(err, t));
    } finally {
      setIsSubmitting(false);
    }
  };

  const isLoading = infiniteQuery.isLoading;
  const isError = infiniteQuery.isError;
  const hasNextPage = infiniteQuery.hasNextPage ?? false;
  const isFetchingNextPage = infiniteQuery.isFetchingNextPage;

  return (
    <div className="h-full flex flex-col">
      {/* Search Input */}
      <div className="px-6 py-2 border-b bg-muted/10">
        <div className="flex items-center gap-2">
          <Search className="h-4 w-4 text-muted-foreground" />
          <Input
            type="text"
            placeholder={t("transactions.searchPlaceholder")}
            value={searchQuery}
            onChange={(e) => handleSearchChange(e.target.value)}
            className="h-8 flex-1"
          />
        </div>
      </div>

      {/* Filter Panel */}
      {accounts && categories && (
        <TransactionFilterPanel
          filter={filter}
          accounts={accounts}
          categories={categories}
          onClearFilter={handleClearFilter}
          onChange={handleFilterChange}
        />
      )}

      {/* Main Content */}
      {isLoading && (
        <div className="flex-1 flex items-center justify-center">
          <span className="text-muted-foreground">{t("common.loading")}</span>
        </div>
      )}

      {isError && (
        <div className="flex-1 flex items-center justify-center">
          <span className="text-destructive">{t("common.errorGeneric")}</span>
        </div>
      )}

      {!isLoading && !isError && flattenedItems.length === 0 && (
        <div className="flex-1 flex items-center justify-center">
          <span className="text-muted-foreground">
            {searchQuery
              ? t("transactions.noResults")
              : t("transactions.noTransactions")}
          </span>
        </div>
      )}

      {!isLoading && !isError && flattenedItems.length > 0 && (
        <VirtualTransactionList
          items={flattenedItems}
          hasNextPage={hasNextPage}
          isFetchingNextPage={isFetchingNextPage}
          fetchNextPage={infiniteQuery.fetchNextPage}
        />
      )}

      {/* Create Form */}
      {accounts && categories && (
        <TransactionForm
          open={formOpen}
          onOpenChange={setFormOpen}
          mode="create"
          accounts={accounts}
          categories={categories}
          onSubmit={handleFormSubmit}
          isLoading={isSubmitting}
        />
      )}
    </div>
  );
}
