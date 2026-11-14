import { useCallback, useMemo } from "react";
import { useSearchParams } from "react-router-dom";
import {
  DEFAULT_REPORT,
  REPORT_IDS,
  findReport,
  type ReportDateControl,
  type ReportId,
} from "@/constants/report";
import type {
  DateRangePreset,
  PeriodGranularity,
  ReportFilter,
} from "@/types/report";
import { getDefaultRangePresetDates, todayLocalDate } from "@/utils/date";

/**
 * Report page state, held in the URL.
 *
 * The reports used to keep four independent copies of "which period am I
 * looking at" — one per component — so switching from cash flow to categories
 * silently reset the range, and there was no way to send someone a link to what
 * you were looking at. The range is one axis shared by every report that takes
 * one, so it is lifted to the page and serialised into the query string; the
 * browser's back button then steps through the reports the way the user
 * remembers them.
 *
 * Only non-default values are written, so a fresh visit stays at a bare
 * `/reports` instead of a six-parameter URL.
 */

const DEFAULT_PRESET: DateRangePreset = "currentMonth";
const DEFAULT_GRANULARITY: PeriodGranularity = "monthly";

export interface ReportQuery {
  report: ReportId;
  preset: DateRangePreset;
  /** Custom range endpoints; only meaningful when `preset === "custom"`. */
  start: string;
  end: string;
  granularity: PeriodGranularity;
  /**
   * The point in time a report is anchored to: a snapshot date for the balance
   * sheet, `YYYY-MM` for budgets. Reports that take a range ignore it.
   */
  anchor: string;
  accountId: string;
}

function parsePreset(value: string | null): DateRangePreset {
  const known: DateRangePreset[] = [
    "currentMonth",
    "lastMonth",
    "last3Months",
    "last6Months",
    "last12Months",
    "currentYear",
    "lastYear",
    "custom",
  ];
  return known.includes(value as DateRangePreset)
    ? (value as DateRangePreset)
    : DEFAULT_PRESET;
}

function parseGranularity(value: string | null): PeriodGranularity {
  return value === "daily" || value === "weekly" || value === "yearly"
    ? value
    : DEFAULT_GRANULARITY;
}

function parseReport(value: string | null): ReportId {
  return REPORT_IDS.includes(value as ReportId)
    ? (value as ReportId)
    : DEFAULT_REPORT;
}

/**
 * The anchor is one parameter but two shapes: a day for the balance sheet, a
 * month for budgets. Switching between those two reports must not hand a
 * `YYYY-MM` to a date picker or a full date to a month picker, so the shape is
 * derived from the report that is asking, not from whatever is in the URL.
 */
function normalizeAnchor(value: string, control: ReportDateControl): string {
  const today = todayLocalDate();
  if (!value) return control === "month" ? today.slice(0, 7) : today;
  if (control === "month") return value.slice(0, 7);
  if (control === "snapshot")
    return value.length === 10 ? value : `${value.slice(0, 7)}-01`;
  return value;
}

export function useReportQuery() {
  const [params, setParams] = useSearchParams();

  const query = useMemo<ReportQuery>(() => {
    const report = parseReport(params.get("r"));
    const { dateControl } = findReport(report);
    return {
      report,
      preset: parsePreset(params.get("p")),
      start: params.get("s") ?? "",
      end: params.get("e") ?? "",
      granularity: parseGranularity(params.get("g")),
      anchor: normalizeAnchor(params.get("d") ?? "", dateControl),
      accountId: params.get("a") ?? "",
    };
  }, [params]);

  const update = useCallback(
    (patch: Partial<ReportQuery>) => {
      setParams(
        (previous) => {
          const next = new URLSearchParams(previous);
          const merged = { ...query, ...patch };
          const set = (key: string, value: string, defaultValue: string) => {
            if (value && value !== defaultValue) next.set(key, value);
            else next.delete(key);
          };
          set("r", merged.report, DEFAULT_REPORT);
          set("p", merged.preset, DEFAULT_PRESET);
          set("g", merged.granularity, DEFAULT_GRANULARITY);
          // `anchor` defaults to today, which changes daily — comparing against
          // it keeps a bare URL bare, but a date the user picked is written.
          set("d", merged.anchor === todayLocalDate() ? "" : merged.anchor, "");
          if (merged.preset === "custom") {
            set("s", merged.start, "");
            set("e", merged.end, "");
          } else {
            next.delete("s");
            next.delete("e");
          }
          set("a", merged.accountId, "");
          return next;
        },
        { replace: true },
      );
    },
    [query, setParams],
  );

  const definition = findReport(query.report);

  /** The concrete `startDate`/`endDate` the backend will be asked for. */
  const range = useMemo(() => {
    // "Custom" with one end empty is a half-finished choice, not a request for
    // the whole ledger: fall back to the default period until both dates exist.
    if (query.preset === "custom" && query.start && query.end) {
      return { startDate: query.start, endDate: query.end };
    }
    return getDefaultRangePresetDates(
      query.preset === "custom" ? "currentMonth" : query.preset,
    );
  }, [query.preset, query.start, query.end]);

  const filter = useMemo<ReportFilter>(
    () => ({
      dateRangePreset: query.preset,
      startDate: range.startDate,
      endDate: range.endDate,
      periodGranularity: query.granularity,
    }),
    [query.preset, query.granularity, range.startDate, range.endDate],
  );

  /** `YYYY` and `MM` for the budget report, both from the same anchor. */
  const anchorYear =
    Number(query.anchor.slice(0, 4)) || new Date().getFullYear();
  const anchorMonth =
    Number(query.anchor.slice(5, 7)) || new Date().getMonth() + 1;

  return {
    query,
    definition,
    range,
    filter,
    anchorYear,
    anchorMonth,
    /** Budgets anchor on a month, so they never carry a day. */
    anchorMonthValue: `${anchorYear}-${String(anchorMonth).padStart(2, "0")}`,
    selectReport: (report: ReportId) => update({ report }),
    setPreset: (preset: DateRangePreset) => update({ preset }),
    setCustomRange: (start: string, end: string) => update({ start, end }),
    setGranularity: (granularity: PeriodGranularity) => update({ granularity }),
    setAnchor: (anchor: string) => update({ anchor }),
    setAccount: (accountId: string) => update({ accountId }),
    reset: () =>
      update({
        preset: DEFAULT_PRESET,
        granularity: DEFAULT_GRANULARITY,
        start: "",
        end: "",
        anchor: todayLocalDate(),
      }),
  };
}

export type ReportQueryController = ReturnType<typeof useReportQuery>;
