import { dbLock } from "@/utils/api";
import { queryClient } from "@/lib/queryClient";
import { useDbStore } from "@/stores/dbStore";
import { useUiStore } from "@/stores/uiStore";

/**
 * Lock the ledger: drop the database key, then drop everything derived from it.
 *
 * This is the only correct order. `dbLock()` closes the SQLCipher connection and
 * releases the key, but the React Query cache still holds decrypted balances and
 * account names; if the navigation to the unlock screen happened first, the
 * locked UI could still read them for a frame.
 *
 * Used by the sidebar lock button, the change-password flow and the idle timer —
 * a second copy of this sequence is how one of them eventually forgets a step.
 */
export async function lockSession(): Promise<void> {
  try {
    await dbLock();
  } catch (err) {
    // An already-locked backend is not an error here; the goal is "no key".
    console.error("[lock] dbLock failed:", err);
  }
  queryClient.clear();
  // Dialogs may hold half-filled forms with amounts in them; drop them too.
  useUiStore.getState().closeQuickAdd();
  useDbStore.getState().setLocked();
}
