import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { getAccountDisplayName } from "@/utils/accountDisplay";
import type { AccountDto } from "@/types";
import {
  formatDate,
  formatDateTime,
  formatMonthYear,
  formatRelative,
} from "@/utils/date";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------

const makeAccount = (overrides: Partial<AccountDto> = {}): AccountDto => ({
  id: "1",
  name: "Test Account",
  account_type: "asset" as const,
  currency: "CNY",
  description: "",
  account_number: null,
  is_system: false,
  iban: null,
  is_active: true,
  include_net_worth: true,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  meta: [],
  ...overrides,
});

describe("getAccountDisplayName", () => {
  it("returns stored name for regular accounts", () => {
    const account = makeAccount({ name: "My Savings" });
    expect(getAccountDisplayName(account)).toBe("My Savings");
  });

  it("uses raw name for accounts with root-like names", () => {
    const account = makeAccount({
      name: "Assets",
    });
    expect(getAccountDisplayName(account)).toBe("Assets");
  });
});

describe("date utilities", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-13T12:00:00Z"));
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("formatDate", () => {
    it("formats a date in Chinese locale (zh)", () => {
      expect(formatDate("2026-04-13T00:00:00Z")).toBe("2026年4月13日");
    });

    it("handles ISO date strings", () => {
      expect(formatDate("2025-12-25T00:00:00Z")).toBe("2025年12月25日");
    });
  });

  describe("formatDateTime", () => {
    it("formats date-time consistently", () => {
      const result = formatDateTime("2026-04-13T12:30:45Z");
      expect(result).toMatch(/^2026-04-13 \d{2}:30:45$/);
    });
  });

  describe("formatMonthYear", () => {
    it("formats month-year in Chinese", () => {
      expect(formatMonthYear("2026-04-13T00:00:00Z")).toBe("2026年4月");
    });
  });

  describe("formatRelative", () => {
    it("returns relative time in Chinese", () => {
      // 2026-04-10 is a few days before fake system time 2026-04-13
      const result = formatRelative("2026-04-10T00:00:00Z");
      expect(result.length).toBeGreaterThan(0);
      // Should be a Chinese relative string (contains "天前" or "前")
      expect(result).toContain("天");
    });

    it("handles future dates", () => {
      const result = formatRelative("2026-04-20T00:00:00Z");
      expect(result.length).toBeGreaterThan(0);
    });
  });
});

describe("cn utility", () => {
  it("merges simple class names", () => {
    expect(cn("a", "b")).toContain("a");
    expect(cn("a", "b")).toContain("b");
  });

  it("merges Tailwind classes without conflicts", () => {
    // twMerge should deduplicate conflicting bg classes
    const result = cn("bg-red-500", "bg-blue-500");
    expect(result).toBe("bg-blue-500");
  });

  it("handles conditional classes", () => {
    expect(cn("base", { conditional: true, nope: false })).toBe(
      "base conditional",
    );
  });

  it("handles falsy values", () => {
    expect(cn("a", false, undefined, null)).toBe("a");
  });
});
