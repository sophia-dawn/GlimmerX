import { create } from "zustand";

/**
 * Small UI store for overlays that must be openable from outside the AppShell —
 * currently only the quick-add dialog, which the global shortcut can summon
 * while the shell is mounted but the user is on another window.
 */
interface UiState {
  quickAddOpen: boolean;
  openQuickAdd: () => void;
  closeQuickAdd: () => void;
  setQuickAddOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  quickAddOpen: false,
  openQuickAdd: () => set({ quickAddOpen: true }),
  closeQuickAdd: () => set({ quickAddOpen: false }),
  setQuickAddOpen: (open: boolean) => set({ quickAddOpen: open }),
}));
