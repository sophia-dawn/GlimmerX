import { describe, it, expect } from "vitest";
import {
  centsToYuan,
  yuanToCents,
  formatAmount,
  formatAmountWithSign,
} from "@/utils/format";

describe("centsToYuan", () => {
  it("converts 0 cents to 0 yuan", () => {
    expect(centsToYuan(0)).toBe(0);
  });

  it("converts 10000 cents to 100 yuan", () => {
    expect(centsToYuan(10000)).toBe(100);
  });

  it("handles negative values", () => {
    expect(centsToYuan(-5000)).toBe(-50);
  });
});

describe("yuanToCents", () => {
  it("converts empty string to 0", () => {
    expect(yuanToCents("")).toBe(0);
  });

  it("converts 100 yuan to 10000 cents", () => {
    expect(yuanToCents("100")).toBe(10000);
  });

  it("handles decimal values", () => {
    expect(yuanToCents("9.99")).toBe(999);
  });
});

describe("formatAmount", () => {
  it("formats zero amount with CNY symbol", () => {
    expect(formatAmount(0, "CNY")).toBe("¥0.00");
  });

  it("formats positive amount with symbol", () => {
    expect(formatAmount(123456, "CNY")).toBe("¥1,234.56");
  });

  it("formats negative amount with negative sign", () => {
    expect(formatAmount(-5000, "CNY")).toBe("¥-50.00");
  });

  it("uses USD symbol for USD currency", () => {
    expect(formatAmount(10000, "USD")).toBe("$100.00");
  });

  it("falls back to currency code for unknown currency", () => {
    expect(formatAmount(10000, "THB")).toBe("THB100.00");
  });
});

describe("formatAmountWithSign", () => {
  it("formats zero without sign", () => {
    expect(formatAmountWithSign(0, "CNY")).toBe("¥0.00");
  });

  it("adds + prefix for positive", () => {
    expect(formatAmountWithSign(10000, "CNY")).toBe("+¥100.00");
  });

  it("shows negative sign without extra prefix", () => {
    expect(formatAmountWithSign(-5000, "CNY")).toBe("¥-50.00");
  });
});
