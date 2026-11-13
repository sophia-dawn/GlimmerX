/**
 * Amount parsing that matches the Rust backend exactly.
 *
 * Every amount crosses the IPC boundary as a decimal string and is parsed by
 * `parse_amount_to_cents` (src-tauri/src/utils/money.rs). The form used to
 * check "is this balanced?" with `Math.round(parseFloat(x) * 100)`, which
 * disagrees with that parser on the third decimal: `2.675` rounded to 267 in
 * one place and 268 in the other, so a legal transaction could not be saved
 * while an illegal one was submitted and rejected.
 *
 * The rules below are the same ones, in the same order: optional sign, digits
 * with at most one decimal point, half-away-from-zero rounding on the third
 * decimal, no scientific notation, capped at MAX_AMOUNT_CENTS.
 */

/** Business upper bound for a single amount, in cents — mirrors Rust. */
export const MAX_AMOUNT_CENTS = 10_000_000_000;

/**
 * Parse a decimal amount string into integer cents, or `null` when the backend
 * would reject it.
 */
export function parseAmountToCents(raw: string): number | null {
  const value = raw.trim();
  if (value.length === 0 || value.length > 24) return null;

  const negative = value.startsWith("-");
  const body =
    value.startsWith("-") || value.startsWith("+") ? value.slice(1) : value;
  if (body.length === 0) return null;

  const parts = body.split(".");
  if (parts.length > 2) return null;
  const integer = parts[0] ?? "";
  const fraction = parts[1] ?? "";
  // "." / "-." are rejected, but ".50" and "1." stay valid like the backend.
  if (integer.length === 0 && fraction.length === 0) return null;
  if (!/^\d*$/.test(integer) || !/^\d*$/.test(fraction)) return null;
  // 12 integer digits is already 1e12 yuan, far past MAX_AMOUNT_CENTS; the cap
  // also keeps the arithmetic below inside Number's exact-integer range.
  if (integer.length > 12) return null;

  const yuan = integer.length === 0 ? 0 : Number(integer);
  let cents = yuan * 100;
  const digit = (index: number): number =>
    index < fraction.length ? fraction.charCodeAt(index) - 48 : 0;
  cents += digit(0) * 10 + digit(1);
  if (digit(2) >= 5) cents += 1;
  if (negative) cents = -cents;
  if (Math.abs(cents) > MAX_AMOUNT_CENTS) return null;

  return cents;
}

/** True when the amount carries more precision than the ledger stores. */
export function hasExcessPrecision(raw: string): boolean {
  if (/e/i.test(raw)) return true;
  const parts = raw.split(".");
  if (parts.length === 1) return false;
  const fraction = parts[1];
  return fraction !== undefined && fraction.length > 2;
}
