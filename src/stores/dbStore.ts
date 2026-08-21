import { create } from "zustand";
import {
  dbCheckAnyExists,
  dbIsUnlocked,
  dbListRecent,
  type RecentDbEntry,
} from "@/utils/api";

const SESSION_DB_PATH_KEY = "glimmerx_db_path";

interface DbState {
  isUnlocked: boolean;
  /** Whether the app is still initializing (checking unlock state) */
  isInitializing: boolean;
  /** Whether any known database exists (false = user should create new) */
  hasExistingDb: boolean | null;
  /** Recent databases from the backend */
  recentDbs: RecentDbEntry[];
  /** Currently selected database path (used for lock/unlock flows) */
  currentDbPath: string | null;
  /** Whether the current session was started by creating a new database */
  isFreshDb: boolean;
  setUnlocked: (path: string, fresh?: boolean) => void;
  setLocked: () => void;
  setInitialized: () => void;
  restoreUnlockState: () => Promise<boolean>;
  checkExistingDb: () => Promise<void>;
  loadRecentDbs: () => Promise<void>;
}

export const useDbStore = create<DbState>((set) => ({
  isUnlocked: false,
  isInitializing: true,
  hasExistingDb: null,
  recentDbs: [],
  currentDbPath: null,
  isFreshDb: false,

  setUnlocked: (path: string, fresh?: boolean) => {
    sessionStorage.setItem(SESSION_DB_PATH_KEY, path);
    set({ isUnlocked: true, currentDbPath: path, isFreshDb: !!fresh });
  },

  setLocked: () => {
    sessionStorage.removeItem(SESSION_DB_PATH_KEY);
    set({ isUnlocked: false, currentDbPath: null, isFreshDb: false });
  },

  setInitialized: () => {
    set({ isInitializing: false });
  },

  restoreUnlockState: async () => {
    const savedPath = sessionStorage.getItem(SESSION_DB_PATH_KEY);
    if (!savedPath) {
      return false;
    }

    try {
      const isUnlocked = await dbIsUnlocked();
      if (isUnlocked) {
        set({ isUnlocked: true, currentDbPath: savedPath, isFreshDb: false });
        return true;
      }
    } catch (err) {
      console.error("[dbStore] dbIsUnlocked error:", err);
    }

    sessionStorage.removeItem(SESSION_DB_PATH_KEY);
    return false;
  },

  checkExistingDb: async () => {
    try {
      const exists = await dbCheckAnyExists();
      set({ hasExistingDb: exists });
    } catch (err) {
      console.error("[dbStore] dbCheckAnyExists error:", err);
      set({ hasExistingDb: false });
    }
  },

  loadRecentDbs: async () => {
    try {
      const recent = await dbListRecent();
      set({ recentDbs: recent });
    } catch (err) {
      console.error("[dbStore] dbListRecent error:", err);
      set({ recentDbs: [] });
    }
  },
}));
