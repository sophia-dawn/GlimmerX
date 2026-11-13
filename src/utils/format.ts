/**
 * Formatting helpers for money and quantities.
 *
 * The ledger is single-currency (CNY). There is no currency parameter anywhere:
 * a second currency is not a supported state, so leaving the door open in the
 * API only invites callers to pass one.
 */

import { parseAmountToCents } from "./money";

/** Every amount renders with this symbol — the ledger is CNY-only. */
export const MONEY_SYMBOL = "¥";

/** Segments are grouped the same way in zh-CN and en-US. */
const AMOUNT_LOCALE = "zh-CN";

/**
 * Convert cents (i64 from Rust) to a decimal number (yuan).
 */
export function centsToYuan(cents: number): number {
  return cents / 100;
}

/**
 * Convert a yuan amount to cents.
 *
 * Delegates to the shared parser so a value the form accepted is the same value
 * the backend computes; an unparseable string stays 0 for callers that only
 * need a fallback.
 */
export function yuanToCents(yuan: string): number {
  return parseAmountToCents(yuan) ?? 0;
}

/**
 * Format an amount in cents to a display string.
 *
 * The minus sign goes *before* the currency symbol (`-¥50.00`, not `¥-50.00`):
 * the sign is part of how the number reads, and burying it between the symbol
 * and the digits makes a liability look like a malformed positive. Callers that
 * want a `+` prefix for inflow should use {@link formatAmountWithSign}.
 */
export function formatAmount(cents: number): string {
  const formatted = Math.abs(centsToYuan(cents)).toLocaleString(AMOUNT_LOCALE, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  return `${cents < 0 ? "-" : ""}${MONEY_SYMBOL}${formatted}`;
}

/**
 * Format an amount with sign prefix: "+" for positive, "-" for negative.
 */
export function formatAmountWithSign(cents: number): string {
  const formatted = formatAmount(cents);
  if (cents === 0) return formatted;
  return cents > 0 ? `+${formatted}` : formatted;
}
