import type { PeriodGranularity } from "@/types/report";

/**
 * How a time axis is cut into buckets.
 *
 * The granularity is not a second time dimension — it is the tick size of the
 * axis the range already defines. 386 days drawn day by day is 386 bars; the
 * same span drawn by month is 13. Letting the user pick the two independently
 * produced twenty-eight combinations, of which eight cut the axis into a
 * single bucket: a chart with one lonely column, which reads as "this month
 * had no transactions" rather than as "that choice made no sense".
 *
 * So the tick size is derived from the span, and the only thing left to choose
 * is the tick size among the ones that actually draw.
 */

/** Fewer buckets than this and the chart is a handful of lonely bars. */
const MIN_BUCKETS = 6;
/** More buckets than this and the bars stop being bars. */
const MAX_BUCKETS = 120;

/** How many days one bucket of each granularity covers. */
const BUCKET_DAYS: Record<PeriodGranularity, number> = {
  daily: 1,
  weekly: 7,
  monthly: 30,
  yearly: 365,
};

/** Finest first: the order the options are offered in. */
const ORDER: PeriodGranularity[] = ["daily", "weekly", "monthly", "yearly"];

/**
 * Days in a span, both ends included.
 *
 * The dates are parsed as local midnight rather than through `toISOString`,
 * which would shift the day across the UTC boundary and make a one-day range
 * look like zero or two.
 */
export function spanDays(startDate: string, endDate: string): number {
  const start = Date.parse(`${startDate}T00:00:00`);
  const end = Date.parse(`${endDate}T00:00:00`);
  if (!Number.isFinite(start) || !Number.isFinite(end)) return 0;
  return Math.max(1, Math.round(Math.abs(end - start) / 86_400_000) + 1);
}

/** How many buckets a granularity cuts a span into. */
export function bucketCount(
  granularity: PeriodGranularity,
  days: number,
): number {
  return Math.ceil(days / BUCKET_DAYS[granularity]);
}

/**
 * The granularities worth offering for this span.
 *
 * A span of three days has no weekly or monthly reading — two bars and one
 * column are not charts — so only "daily" survives, and the control that offers
 * a single choice is not worth showing.
 */
export function availableGranularities(
  startDate: string,
  endDate: string,
): PeriodGranularity[] {
  const days = spanDays(startDate, endDate);
  return ORDER.filter((granularity) => {
    const buckets = bucketCount(granularity, days);
    return buckets >= MIN_BUCKETS && buckets <= MAX_BUCKETS;
  });
}

/**
 * The tick size a span gets when nobody has an opinion: daily for a month,
 * weekly for a quarter, monthly for anything longer.
 */
export function deriveGranularity(
  startDate: string,
  endDate: string,
): PeriodGranularity {
  const days = spanDays(startDate, endDate);
  if (days <= 45) return "daily";
  if (days <= 120) return "weekly";
  return "monthly";
}

/**
 * The granularity to draw with, and the ones worth offering beside it.
 *
 * A requested granularity that the span cannot show is not honoured — it is
 * replaced by the nearest sensible one. The old behaviour kept the choice and
 * let the chart come out empty, which is a worse answer than a chart that
 * draws something else.
 */
export function resolveGranularity(
  requested: PeriodGranularity | null,
  startDate: string,
  endDate: string,
): { granularity: PeriodGranularity; options: PeriodGranularity[] } {
  const options = availableGranularities(startDate, endDate);
  const derived = deriveGranularity(startDate, endDate);

  if (requested && options.includes(requested)) {
    return { granularity: requested, options };
  }
  if (options.length === 0) return { granularity: derived, options };
  if (options.includes(derived)) return { granularity: derived, options };

  // The derived default is not on offer for this span (three days has no
  // valid tick), so take the option whose bucket count is closest to it.
  const days = spanDays(startDate, endDate);
  const target = bucketCount(derived, days);
  const closest = options.reduce((best, option) =>
    Math.abs(bucketCount(option, days) - target) <
    Math.abs(bucketCount(best, days) - target)
      ? option
      : best,
  );
  return { granularity: closest, options };
}
