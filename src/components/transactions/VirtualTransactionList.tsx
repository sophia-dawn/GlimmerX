import { useRef, useEffect, memo } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { TransactionCard } from "./TransactionCard";
import { formatAmount } from "@/utils/format";
import { formatDateLong } from "@/utils/date";
import type { VirtualListItem } from "@/utils/virtualList";

interface VirtualTransactionListProps {
  items: VirtualListItem[];
  hasNextPage: boolean;
  isFetchingNextPage: boolean;
  fetchNextPage: () => void;
  onScrollTop?: () => void;
}

export const VirtualTransactionList = memo(function VirtualTransactionList({
  items,
  hasNextPage,
  isFetchingNextPage,
  fetchNextPage,
}: VirtualTransactionListProps) {
  const { t } = useTranslation();
  const scrollContainerRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => scrollContainerRef.current,
    estimateSize: (index) => {
      const item = items[index];
      if (!item) return 64;
      return item.type === "date-header" ? 44 : 64;
    },
    overscan: 10,
  });

  const sentinelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!hasNextPage || isFetchingNextPage) return;

    const observer = new IntersectionObserver(
      (entries) => {
        const [entry] = entries;
        if (entry?.isIntersecting && hasNextPage && !isFetchingNextPage) {
          fetchNextPage();
        }
      },
      {
        root: scrollContainerRef.current,
        threshold: 0.1,
        rootMargin: "100px",
      },
    );

    if (sentinelRef.current) {
      observer.observe(sentinelRef.current);
    }

    return () => observer.disconnect();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);

  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div
      ref={scrollContainerRef}
      className="flex-1 overflow-y-auto px-6 py-4"
      style={{ contain: "strict" }}
    >
      <div
        style={{
          height: `${virtualizer.getTotalSize()}px`,
          width: "100%",
          position: "relative",
        }}
      >
        {virtualItems.map((virtualItem) => {
          const item = items[virtualItem.index];
          if (!item) return null;

          return (
            <div
              key={virtualItem.key}
              style={{
                position: "absolute",
                top: 0,
                left: 0,
                width: "100%",
                height: `${virtualItem.size}px`,
                transform: `translateY(${virtualItem.start}px)`,
              }}
            >
              {item.type === "date-header" ? (
                <DateHeader date={item.date} dayTotal={item.dayTotal} />
              ) : (
                <TransactionCard transaction={item.data} />
              )}
            </div>
          );
        })}
      </div>

      {hasNextPage && items.length > 0 && (
        <div
          ref={sentinelRef}
          className="h-4"
          style={{ marginTop: `${Math.max(0, items.length - 5) * 64}px` }}
        />
      )}

      {isFetchingNextPage && (
        <div className="flex items-center justify-center py-4">
          <Loader2 className="h-4 w-4 animate-spin mr-2" />
          <span className="text-sm text-muted-foreground">
            {t("common.loadingMore")}
          </span>
        </div>
      )}
    </div>
  );
});

const DateHeader = memo(function DateHeader({
  date,
  dayTotal,
}: {
  date: string;
  dayTotal: number;
}) {
  const { t } = useTranslation();

  return (
    <div className="flex items-center justify-between mb-2 px-2 pt-2">
      <span className="text-lg font-bold text-primary">
        {formatDateLong(date)}
      </span>
      <span className="text-sm text-muted-foreground tabular-nums">
        {t("transactions.dayTotal")}: {formatAmount(dayTotal)}
      </span>
    </div>
  );
});
