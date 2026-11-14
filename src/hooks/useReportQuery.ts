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
  CompareMode,
  DateRangePreset,
  PeriodGranularity,
  ReportFilter,
} from "@/types/report";
import { getDefaultRangePresetDates, todayLocalDate } from "@/utils/date";
import { resolveGranularity } from "@/utils/reportGranularity";

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
 *
 * The tick size (`g`) is the exception to "only non-default values": it has no
 * default of its own, it is *derived* from the range (see
 * `utils/reportGranularity`), and the URL only records the tick size when the
 * user overrides that derivation.
 */

const DEFAULT_PRESET: DateRangePreset = "currentMonth";
const DEFAULT_COMPARE: CompareMode = "previousPeriod";

export interface ReportQuery {
  report: ReportId;
  preset: DateRangePreset;
  /** Custom range endpoints; only meaningful when `preset === "custom"`. */
  start: string;
  end: string;
  /**
   * The tick size the user asked for, or null when they have not overridden
   * the one the range implies.
   */
  granularity: PeriodGranularity | null;
  /**
   * What the totals are compared against: the previous period, the same
   * stretch a year ago, or nothing at all.
   */
  compare: CompareMode;
  /**
   * The point in time a report is anchored to: a snapshot date for the balance
   * sheet, `YYYY-MM` for budgets. Reports that take a range ignore it.
   */
  anchor: string;
  accountId: string;
}

/**
 * The concrete dates a preset resolves to.
 *
 * "Custom" with one end empty is a half-finished choice, not a request for
 * the whole ledger: it falls back to the default period until both dates
 * exist.
 */
export function resolveRange(
  preset: DateRangePreset,
  start: string,
  end: string,
): { startDate: string; endDate: string } {
  if (preset === "custom" && start && end) {
    // The backend swaps a reversed range as well; doing it here keeps the scope
    // line and the tick size honest about which stretch is being shown.
    return start > end
      ? { startDate: end, endDate: start }
      : { startDate: start, endDate: end };
  }
  return getDefaultRangePresetDates(
    preset === "custom" ? "currentMonth" : preset,
  );
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

function parseGranularity(value: string | null): PeriodGranularity | null {
  return value === "daily" ||
    value === "weekly" ||
    value === "monthly" ||
    value === "yearly"
    ? value
    : null;
}

function parseCompare(value: string | null): CompareMode {
  return value === "lastYear" || value === "none" ? value : DEFAULT_COMPARE;
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
      compare: parseCompare(params.get("c")),
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
          // The tick size has no default of its own — it is derived from the
          // range — so the URL only records it when the user overrode that.
          if (merged.granularity) next.set("g", merged.granularity);
          else next.delete("g");
          set("c", merged.compare, DEFAULT_COMPARE);
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
  const range = useMemo(
    () => resolveRange(query.preset, query.start, query.end),
    [query.preset, query.start, query.end],
  );

  /**
   * The tick size of the time axis, and the ones worth offering.
   *
   * The tick size follows the range unless the user says otherwise, and a
   * request the range cannot show is replaced by the nearest sensible one —
   * a chart with a single column is a worse answer than a chart that draws
   * something else.
   */
  const scale = useMemo(
    () => resolveGranularity(query.granularity, range.startDate, range.endDate),
    [query.granularity, range.startDate, range.endDate],
  );

  const filter = useMemo<ReportFilter>(
    () => ({
      dateRangePreset: query.preset,
      startDate: range.startDate,
      endDate: range.endDate,
      periodGranularity: scale.granularity,
      compare: query.compare,
    }),
    [
      query.preset,
      query.compare,
      scale.granularity,
      range.startDate,
      range.endDate,
    ],
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
    /** The tick size the charts are drawn with. */
    granularity: scale.granularity,
    /** The tick sizes the current range can show; one means no choice to make. */
    granularityOptions: scale.options,
    anchorYear,
    anchorMonth,
    /** Budgets anchor on a month, so they never carry a day. */
    anchorMonthValue: `${anchorYear}-${String(anchorMonth).padStart(2, "0")}`,
    selectReport: (report: ReportId) => update({ report }),
    setPreset: (preset: DateRangePreset) => update({ preset }),
    setCustomRange: (start: string, end: string) => update({ start, end }),
    setGranularity: (granularity: PeriodGranularity) => update({ granularity }),
    setCompare: (compare: CompareMode) => update({ compare }),
    setAnchor: (anchor: string) => update({ anchor }),
    setAccount: (accountId: string) => update({ accountId }),
    reset: () =>
      update({
        preset: DEFAULT_PRESET,
        // No tick size: the range implies one again.
        granularity: null,
        compare: DEFAULT_COMPARE,
        start: "",
        end: "",
        anchor: todayLocalDate(),
      }),
  };
}

export type ReportQueryController = ReturnType<typeof useReportQuery>;
