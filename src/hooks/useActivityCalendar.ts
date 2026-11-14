import { useMemo } from "react";
import { useQuery } from "@tanstack/react-query";
import type { TransactionDto } from "@/types";
import { transactionList } from "@/utils/api";
import { formatLocalDate } from "@/utils/date";

/** 13 columns × 7 rows, so every column is one week on the 32 px substrate. */
export const ACTIVITY_WEEKS = 13;
export const ACTIVITY_DAYS = ACTIVITY_WEEKS * 7;

export interface ActivityCell {
  date: string;
  count: number;
  /** Days after today in the current week are drawn as empty slots. */
  future: boolean;
}

export interface ActivityCalendarData {
  cells: ActivityCell[];
  max: number;
  total: number;
  /** `null` when there is no entry in the window — not zero, which would lie. */
  daysSinceLastEntry: number | null;
  weekLabels: string[];
}

/**
 * The substrate is a scale, not a wallpaper: each cell is one day's postings,
 * so erasing it would lose the shape of the month. Read from the unpaginated
 * transaction list because `transaction_list_paginated` clamps `page_size` to
 * 100, and a busy quarter easily exceeds that — the calendar must not silently
 * truncate its own history.
 */
function buildCalendar(
  rows: TransactionDto[],
  start: Date,
  todayKey: string,
): ActivityCalendarData {
  const counts = new Map<string, number>();
  for (const row of rows) {
    const key = formatLocalDate(new Date(row.date));
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }

  const cells: ActivityCell[] = [];
  for (let i = 0; i < ACTIVITY_DAYS; i++) {
    const date = new Date(start);
    date.setDate(start.getDate() + i);
    const key = formatLocalDate(date);
    cells.push({
      date: key,
      count: counts.get(key) ?? 0,
      future: key > todayKey,
    });
  }

  const past = [...counts.keys()].filter((key) => key <= todayKey).sort();
  const last = past.length > 0 ? past[past.length - 1] : undefined;
  const daysSinceLastEntry = last
    ? Math.round(
        (new Date(`${todayKey}T00:00:00`).getTime() -
          new Date(`${last}T00:00:00`).getTime()) /
          86_400_000,
      )
    : null;

  return {
    cells,
    max: Math.max(1, ...cells.map((cell) => cell.count)),
    total: cells.reduce((sum, cell) => sum + cell.count, 0),
    daysSinceLastEntry,
    weekLabels: [0, 6, 12].map((week) => cells[week * 7]?.date.slice(5) ?? ""),
  };
}

export function useActivityCalendar() {
  const range = useMemo(() => {
    const now = new Date();
    const mondayOffset = (now.getDay() + 6) % 7;
    // End on Sunday so the last column is the current week, then walk back
    // exactly 90 days to land on a Monday: 13 complete columns, no remainder.
    const end = new Date(now);
    end.setDate(now.getDate() + (6 - mondayOffset));
    const start = new Date(end);
    start.setDate(end.getDate() - (ACTIVITY_DAYS - 1));
    return {
      fromDate: formatLocalDate(start),
      toDate: formatLocalDate(end),
      todayKey: formatLocalDate(now),
      start,
    };
  }, []);

  return useQuery({
    queryKey: ["activity-calendar", range.fromDate, range.toDate],
    queryFn: () =>
      transactionList({
        fromDate: range.fromDate,
        toDate: range.toDate,
      }),
    staleTime: 5 * 60 * 1000,
    select: (rows) => buildCalendar(rows, range.start, range.todayKey),
  });
}
