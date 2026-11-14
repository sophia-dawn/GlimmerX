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
    expect(result.current.query.granularity).toBe("monthly");
    expect(result.current.definition.dateControl).toBe("range");
    expect(result.current.range.startDate).toBeTruthy();
    expect(result.current.range.endDate).toBeTruthy();
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
