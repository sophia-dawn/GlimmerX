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
 *
 * @returns whether the key was actually dropped. The UI is cleared either way (a
 *   half-locked screen still showing balances is worse than a stale unlock
 *   prompt), but callers must not report "locked" when this is `false`: the
 *   backend may still hold an open, keyed connection, and that is something the
 *   user has to be told about rather than have swallowed.
 */
export async function lockSession(): Promise<boolean> {
  // Only call the backend when there is something to close: `dbLock` on an
  // already-locked ledger errors out, and counting that as a failure would make
  // every redundant lock report a problem that does not exist.
  const wasUnlocked = useDbStore.getState().isUnlocked;

  let keyDropped = true;
  if (wasUnlocked) {
    try {
      await dbLock();
    } catch (first) {
      // One retry: a transport hiccup is not the same as a key that refuses to
      // go, and the difference decides whether the ledger is really shut.
      try {
        await dbLock();
      } catch (second) {
        keyDropped = false;
        console.error("[lock] dbLock failed twice:", first, second);
      }
    }
  }

  queryClient.clear();
  // Dialogs may hold half-filled forms with amounts in them; drop them too.
  useUiStore.getState().closeQuickAdd();
  useDbStore.getState().setLocked();

  return keyDropped;
}
