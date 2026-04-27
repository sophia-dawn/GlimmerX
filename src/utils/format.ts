import { DEFAULT_CURRENCY } from "@/constants";

// Currency symbol lookup for common currencies
export const CURRENCY_SYMBOLS: Record<string, string> = {
  CNY: "¥",
  USD: "$",
  EUR: "€",
  GBP: "£",
  JPY: "¥",
};

/**
 * Convert cents (i64 from Rust) to a decimal number (yuan/dollars).
 */
export function centsToYuan(cents: number): number {
  return cents / 100;
}

/**
 * Convert a yuan/dollar amount to cents.
 */
export function yuanToCents(yuan: string): number {
  const value = parseFloat(yuan);
  if (isNaN(value)) return 0;
  return Math.round(value * 100);
}

/**
 * Format an amount in cents to a display string.
 *
 * Positive amounts are rendered in green (income/debit),
 * negative amounts in red (expense/credit).
 */
export function formatAmount(
  cents: number,
  currency: string = DEFAULT_CURRENCY,
): string {
  const symbol = CURRENCY_SYMBOLS[currency] ?? currency;
  const yuanAmount = centsToYuan(cents);
  const formatted = yuanAmount.toLocaleString("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  // yuanAmount includes sign, toLocaleString preserves it
  return `${symbol}${formatted}`;
}

/**
 * Format an amount with sign prefix: "+" for positive, "-" for negative.
 */
export function formatAmountWithSign(
  cents: number,
  currency: string = DEFAULT_CURRENCY,
): string {
  const formatted = formatAmount(cents, currency);
  if (cents === 0) return formatted;
  return cents > 0 ? `+${formatted}` : formatted;
}
