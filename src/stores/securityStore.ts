import { create } from "zustand";

/**
 * Security preferences: the idle auto-lock.
 *
 * Stored in localStorage rather than the encrypted `settings` table on purpose —
 * this is a *device* behaviour, not ledger data. Keeping it out of the DB also
 * means the idle timer can start before (or without) an unlock, and that
 * opening a different ledger does not silently change how this machine locks.
 */

const AUTO_LOCK_KEY = "glimmerx:security:autoLockMinutes";

/**
 * Keys of features that are gone. They go with the feature rather than
 * lingering in every profile that ever turned them on.
 */
const REMOVED_KEYS = [
  "glimmerx:security:lockOnBlur",
  "glimmerx:security:quickAddShortcut",
];

export function clearRemovedSecurityKeys() {
  for (const key of REMOVED_KEYS) localStorage.removeItem(key);
}
clearRemovedSecurityKeys();

export const DEFAULT_AUTO_LOCK_MINUTES = 5;

/** 0 disables the timer; everything else is minutes. */
export const AUTO_LOCK_CHOICES = [0, 1, 5, 15, 30] as const;

function readMinutes(): number {
  const raw = localStorage.getItem(AUTO_LOCK_KEY);
  if (raw === null) return DEFAULT_AUTO_LOCK_MINUTES;
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 0) return DEFAULT_AUTO_LOCK_MINUTES;
  return parsed;
}

interface SecurityState {
  /** Idle minutes before locking; 0 = never lock automatically. */
  autoLockMinutes: number;
  setAutoLockMinutes: (minutes: number) => void;
}

export const useSecurityStore = create<SecurityState>((set) => ({
  autoLockMinutes: readMinutes(),

  setAutoLockMinutes: (minutes: number) => {
    const normalized = Number.isFinite(minutes) && minutes > 0 ? minutes : 0;
    localStorage.setItem(AUTO_LOCK_KEY, String(normalized));
    set({ autoLockMinutes: normalized });
  },
}));
