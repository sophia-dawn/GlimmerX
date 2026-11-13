import { create } from "zustand";

/**
 * Security preferences: idle auto-lock and the global quick-add shortcut.
 *
 * Stored in localStorage rather than the encrypted `settings` table on purpose —
 * these are *device* behaviours, not ledger data. Keeping them out of the DB
 * also means the idle timer can start before (or without) an unlock, and that
 * opening a different ledger does not silently change how this machine locks.
 */

const AUTO_LOCK_KEY = "glimmerx:security:autoLockMinutes";
const LOCK_ON_BLUR_KEY = "glimmerx:security:lockOnBlur";
const SHORTCUT_KEY = "glimmerx:security:quickAddShortcut";

export const DEFAULT_AUTO_LOCK_MINUTES = 5;
export const DEFAULT_QUICK_ADD_SHORTCUT = "CommandOrControl+Shift+A";

/** 0 disables the timer; everything else is minutes. */
export const AUTO_LOCK_CHOICES = [0, 1, 5, 15, 30] as const;

function readMinutes(): number {
  const raw = localStorage.getItem(AUTO_LOCK_KEY);
  if (raw === null) return DEFAULT_AUTO_LOCK_MINUTES;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 0) return DEFAULT_AUTO_LOCK_MINUTES;
  return parsed;
}

function readBool(key: string, fallback: boolean): boolean {
  const raw = localStorage.getItem(key);
  if (raw === null) return fallback;
  return raw === "true";
}

interface SecurityState {
  /** Idle minutes before locking; 0 = never lock automatically. */
  autoLockMinutes: number;
  /** Lock as soon as the window loses focus. */
  lockOnBlur: boolean;
  /** Accelerator for "open quick add", e.g. `CommandOrControl+Shift+A`. */
  quickAddShortcut: string;
  setAutoLockMinutes: (minutes: number) => void;
  setLockOnBlur: (value: boolean) => void;
  setQuickAddShortcut: (accelerator: string) => void;
}

export const useSecurityStore = create<SecurityState>((set) => ({
  autoLockMinutes: readMinutes(),
  lockOnBlur: readBool(LOCK_ON_BLUR_KEY, false),
  quickAddShortcut:
    localStorage.getItem(SHORTCUT_KEY) ?? DEFAULT_QUICK_ADD_SHORTCUT,

  setAutoLockMinutes: (minutes: number) => {
    const normalized = Number.isFinite(minutes) && minutes > 0 ? minutes : 0;
    localStorage.setItem(AUTO_LOCK_KEY, String(normalized));
    set({ autoLockMinutes: normalized });
  },

  setLockOnBlur: (value: boolean) => {
    localStorage.setItem(LOCK_ON_BLUR_KEY, String(value));
    set({ lockOnBlur: value });
  },

  setQuickAddShortcut: (accelerator: string) => {
    localStorage.setItem(SHORTCUT_KEY, accelerator);
    set({ quickAddShortcut: accelerator });
  },
}));
