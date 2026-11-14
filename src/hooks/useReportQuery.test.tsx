import { describe, it, expect } from "vitest";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { useReportQuery } from "@/hooks/useReportQuery";
import { todayLocalDate } from "@/utils/date";

/**
 * The URL is the report page's state store, so these are the contract tests for
 * it: what a link means, and what a click writes back. Anything that changes
 * the shape of `?r=&p=&g=&d=&s=&e=&a=` has to keep these green, because a link
 * someone saved is a small promise.
 */
function renderAt(initial: string) {
  return renderHook(() => useReportQuery(), {
    wrapper: ({ children }: { children: ReactNode }) => (
      <MemoryRouter initialEntries={[initial]}>{children}</MemoryRouter>
    ),
  });
}

describe("useReportQuery", () => {
  it("starts on cash flow for the current month", () => {
    const { result } = renderAt("/reports");
    expect(result.current.query.report).toBe("cashflow");
    expect(result.current.query.preset).toBe("currentMonth");
    // No tick size of its own: the month implies a daily axis.
    expect(result.current.query.granularity).toBeNull();
    expect(result.current.granularity).toBe("daily");
    expect(result.current.definition.dateControl).toBe("range");
    expect(result.current.range.startDate).toBeTruthy();
    expect(result.current.range.endDate).toBeTruthy();
  });

  it("lets the range decide the tick size", () => {
    const { result } = renderAt("/reports?p=last12Months");
    expect(result.current.query.granularity).toBeNull();
    expect(result.current.granularity).toBe("monthly");
  });

  it("keeps a tick size the range can show", () => {
    const { result } = renderAt("/reports?p=last12Months&g=weekly");
    expect(result.current.query.granularity).toBe("weekly");
    expect(result.current.granularity).toBe("weekly");
  });

  it("replaces a tick size the range cannot show", () => {
    // Yearly over one month is a single column, which reads as no data.
    const { result } = renderAt("/reports?p=currentMonth&g=yearly");
    expect(result.current.granularity).toBe("daily");
    expect(result.current.granularityOptions).toEqual(["daily"]);
  });

  it("records a chosen tick size in the URL", () => {
    const { result } = renderAt("/reports?p=last12Months");
    act(() => result.current.setGranularity("weekly"));
    expect(result.current.query.granularity).toBe("weekly");
  });

  it("compares against the previous period by default", () => {
    const { result } = renderAt("/reports");
    expect(result.current.query.compare).toBe("previousPeriod");
    expect(result.current.filter.compare).toBe("previousPeriod");
  });

  it("reads the comparison basis from the URL", () => {
    const { result } = renderAt("/reports?c=lastYear");
    expect(result.current.query.compare).toBe("lastYear");
    expect(result.current.filter.compare).toBe("lastYear");
  });

  it("records a chosen comparison basis", () => {
    const { result } = renderAt("/reports");
    act(() => result.current.setCompare("none"));
    expect(result.current.query.compare).toBe("none");
  });

  it("ignores an unknown report id instead of rendering nothing", () => {
    const { result } = renderAt("/reports?r=nonsense");
    expect(result.current.query.report).toBe("cashflow");
  });

  it("reads a month anchor for the budget report", () => {
    const { result } = renderAt("/reports?r=budgets&d=2026-03");
    expect(result.current.definition.dateControl).toBe("month");
    expect(result.current.query.anchor).toBe("2026-03");
    expect(result.current.anchorYear).toBe(2026);
    expect(result.current.anchorMonth).toBe(3);
    expect(result.current.anchorMonthValue).toBe("2026-03");
  });

  it("widens a month anchor to a day for the balance sheet", () => {
    const { result } = renderAt("/reports?r=balanceSheet&d=2026-03");
    expect(result.current.definition.dateControl).toBe("snapshot");
    expect(result.current.query.anchor).toBe("2026-03-01");
  });

  it("falls back to the current month when a custom range is half empty", () => {
    const { result } = renderAt("/reports?p=custom&s=2024-01-01");
    const month = renderAt("/reports").result.current.range;
    expect(result.current.range).toEqual(month);
  });

  it("honours a complete custom range", () => {
    const { result } = renderAt("/reports?p=custom&s=2024-01-01&e=2024-03-31");
    expect(result.current.range).toEqual({
      startDate: "2024-01-01",
      endDate: "2024-03-31",
    });
  });

  it("writes the selected report and keeps the period", () => {
    const { result } = renderAt("/reports?p=last6Months");
    act(() => result.current.selectReport("netWorth"));
    expect(result.current.query.report).toBe("netWorth");
    expect(result.current.query.preset).toBe("last6Months");
  });

  it("stores an account so the view can be linked", () => {
    const { result } = renderAt("/reports?r=accounts");
    act(() => result.current.setAccount("acc-1"));
    expect(result.current.query.accountId).toBe("acc-1");
  });

  it("clears a custom range when the preset changes away from it", () => {
    const { result } = renderAt("/reports?p=custom&s=2024-01-01&e=2024-03-31");
    act(() => result.current.setPreset("currentYear"));
    expect(result.current.query.preset).toBe("currentYear");
    expect(result.current.query.start).toBe("");
    expect(result.current.query.end).toBe("");
  });

  it("resets the anchor to today", () => {
    const { result } = renderAt("/reports?r=balanceSheet&d=2020-01-01");
    act(() => result.current.reset());
    expect(result.current.query.anchor).toBe(todayLocalDate());
  });
});
