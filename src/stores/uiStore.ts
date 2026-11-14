import { create } from "zustand";

/**
 * Small UI store for overlays that must be reachable from outside the
 * AppShell — currently only the quick-add dialog.
 *
 * It is a store rather than component state because the lock sequence has to
 * close that dialog from outside the React tree: a half-filled form with
 * amounts in it must not survive a lock. `utils/session.ts` reaches in through
 * `closeQuickAdd()` when the ledger shuts.
 */
interface UiState {
  quickAddOpen: boolean;
  closeQuickAdd: () => void;
  setQuickAddOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  quickAddOpen: false,
  closeQuickAdd: () => set({ quickAddOpen: false }),
  setQuickAddOpen: (open: boolean) => set({ quickAddOpen: open }),
}));
