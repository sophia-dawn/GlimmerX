import type { AccountDto } from "@/types";

/**
 * Returns the display name for an account.
 * In the flat model, all accounts use their stored name directly.
 */
export function getAccountDisplayName(account: AccountDto): string {
  return account.name;
}
