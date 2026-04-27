import type { AccountType } from "@/types";

/**
 * Liability account balance display helper functions.
 *
 * Internal storage: negative = debt owed (e.g., -7000 = owe 7000)
 * Display logic (KMyMoney style): inverted sign, positive = debt owed
 *
 * This ensures user always sees positive numbers for what they owe,
 * matching credit card statements from financial institutions.
 */

/**
 * Get liability display balance by inverting sign.
 *
 * Internal -7000 → Display +7000 (debt owed)
 * Internal +1000 → Display -1000 (surplus/credit)
 *
 * @param balanceCents Internal balance in cents
 * @param accountType Account type
 * @returns Display balance in cents
 */
export function getLiabilityDisplayBalance(
  balanceCents: number,
  accountType: AccountType,
): number {
  if (accountType !== "liability") {
    return balanceCents;
  }
  // Liability: invert sign for display
  return -balanceCents;
}

/**
 * Get liability status i18n key based on display balance.
 *
 * @param displayBalance Display balance (already inverted)
 * @returns i18n key for status label, or null if no label needed
 */
export function getLiabilityStatusKey(displayBalance: number): string | null {
  if (displayBalance > 0) {
    return "accounts.liabilityStatus.owed";
  } else if (displayBalance < 0) {
    return "accounts.liabilityStatus.surplus";
  } else {
    return "accounts.liabilityStatus.cleared";
  }
}

/**
 * Convert display value to storage value for liability accounts.
 * Used when submitting edit form.
 *
 * Display +7000 → Storage -7000 (debt owed)
 * Display -1000 → Storage +1000 (surplus)
 *
 * @param displayValue Display value in cents
 * @param accountType Account type
 * @returns Storage value in cents
 */
export function liabilityDisplayToStorage(
  displayValue: number,
  accountType: AccountType,
): number {
  if (accountType !== "liability") {
    return displayValue;
  }
  // Liability: invert sign for storage
  return -displayValue;
}

/**
 * Convert storage value to display value for liability accounts.
 * Used when initializing edit form.
 *
 * Storage -7000 → Display +7000 (debt owed)
 * Storage +1000 → Display -1000 (surplus)
 *
 * @param storageValue Storage value in cents
 * @param accountType Account type
 * @returns Display value in cents
 */
export function liabilityStorageToDisplay(
  storageValue: number,
  accountType: AccountType,
): number {
  return getLiabilityDisplayBalance(storageValue, accountType);
}
