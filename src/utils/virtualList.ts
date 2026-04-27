import type { TransactionListItem, TransactionDateGroup } from "@/types";

export type VirtualListItem =
  | { type: "date-header"; date: string; dayTotal: number; isFirst: boolean }
  | { type: "transaction"; data: TransactionListItem; date: string };

export function flattenToVirtualItems(
  groups: TransactionDateGroup[],
): VirtualListItem[] {
  const items: VirtualListItem[] = [];
  let isFirst = true;

  for (const group of groups) {
    items.push({
      type: "date-header",
      date: group.date,
      dayTotal: group.dayTotal,
      isFirst,
    });
    isFirst = false;

    for (const tx of group.items) {
      items.push({ type: "transaction", data: tx, date: group.date });
    }
  }

  return items;
}
