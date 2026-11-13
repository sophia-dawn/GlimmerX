/**
 * Formatting helpers for money and quantities.
 *
 * The ledger is single-currency (CNY). There is no currency parameter anywhere:
 * a second currency is not a supported state, so leaving the door open in the
 * API only invites callers to pass one.
 */

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
 */
export function yuanToCents(yuan: string): number {
  const value = parseFloat(yuan);
  if (isNaN(value)) return 0;
  return Math.round(value * 100);
}

/**
 * Format an amount in cents to a display string.
 *
 * Negative amounts keep their own minus sign (`¥-50.00`); callers that want a
 * `+` prefix for inflow should use {@link formatAmountWithSign}.
 */
export function formatAmount(cents: number): string {
  const formatted = centsToYuan(cents).toLocaleString(AMOUNT_LOCALE, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
  // yuanAmount includes sign, toLocaleString preserves it
  return `${MONEY_SYMBOL}${formatted}`;
}

/**
 * Format an amount with sign prefix: "+" for positive, "-" for negative.
 */
export function formatAmountWithSign(cents: number): string {
  const formatted = formatAmount(cents);
  if (cents === 0) return formatted;
  return cents > 0 ? `+${formatted}` : formatted;
}
