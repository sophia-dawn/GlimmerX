import { describe, expect, it } from "vitest";
import {
  availableGranularities,
  bucketCount,
  deriveGranularity,
  resolveGranularity,
  spanDays,
} from "./reportGranularity";

/**
 * The tick size of a time axis.
 *
 * The rule these tests lock: the tick follows the range, and the only thing
 * the user may change is the tick size among the ones that actually draw. A
 * chart cut into a single bucket reads as "there was no money in this month",
 * which is a claim about the ledger rather than about the choice — so the
 * choice is not allowed to produce one.
 */
describe("spanDays", () => {
  it("counts both ends", () => {
    expect(spanDays("2026-11-01", "2026-11-14")).toBe(14);
    expect(spanDays("2026-11-01", "2026-11-01")).toBe(1);
  });

  it("survives a reversed range", () => {
    expect(spanDays("2026-11-14", "2026-11-01")).toBe(14);
  });
});

describe("deriveGranularity", () => {
  it("draws a month day by day", () => {
    expect(deriveGranularity("2026-11-01", "2026-11-30")).toBe("daily");
  });

  it("draws a quarter by week", () => {
    expect(deriveGranularity("2026-08-01", "2026-10-31")).toBe("weekly");
  });

  it("draws a year by month", () => {
    expect(deriveGranularity("2025-11-01", "2026-10-31")).toBe("monthly");
  });
});

describe("availableGranularities", () => {
  it("offers only daily for a month", () => {
    // Weekly would be four lonely bars, monthly one column.
    expect(availableGranularities("2026-11-01", "2026-11-30")).toEqual([
      "daily",
    ]);
  });

  it("offers week and month for half a year", () => {
    expect(availableGranularities("2026-05-01", "2026-10-31")).toEqual([
      "weekly",
      "monthly",
    ]);
  });

  it("never offers a tick that would draw a single bucket", () => {
    for (const granularity of availableGranularities(
      "2026-01-01",
      "2026-12-31",
    )) {
      expect(bucketCount(granularity, 365)).toBeGreaterThan(1);
    }
  });
});

describe("resolveGranularity", () => {
  it("keeps a request the range can show", () => {
    const resolved = resolveGranularity("weekly", "2026-05-01", "2026-10-31");
    expect(resolved.granularity).toBe("weekly");
    expect(resolved.options).toEqual(["weekly", "monthly"]);
  });

  it("replaces a request the range cannot show", () => {
    // Yearly over one month is a single column, which reads as no data.
    const resolved = resolveGranularity("yearly", "2026-11-01", "2026-11-30");
    expect(resolved.granularity).toBe("daily");
    expect(resolved.options).toEqual(["daily"]);
  });

  it("derives one when nobody asked", () => {
    expect(
      resolveGranularity(null, "2026-11-01", "2026-11-30").granularity,
    ).toBe("daily");
  });

  it("falls back to the derived tick when the span offers none", () => {
    // Three days has no valid tick at all; the chart still draws by day.
    const resolved = resolveGranularity(null, "2026-11-01", "2026-11-03");
    expect(resolved.options).toEqual([]);
    expect(resolved.granularity).toBe("daily");
  });
});
