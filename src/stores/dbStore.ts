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
    console.log("[dbStore] setUnlocked, path:", path, "fresh:", fresh);
    sessionStorage.setItem(SESSION_DB_PATH_KEY, path);
    set({ isUnlocked: true, currentDbPath: path, isFreshDb: !!fresh });
  },

  setLocked: () => {
    console.log("[dbStore] setLocked");
    sessionStorage.removeItem(SESSION_DB_PATH_KEY);
    set({ isUnlocked: false, currentDbPath: null, isFreshDb: false });
  },

  setInitialized: () => {
    console.log("[dbStore] setInitialized");
    set({ isInitializing: false });
  },

  restoreUnlockState: async () => {
    console.log("[dbStore] restoreUnlockState start");
    const savedPath = sessionStorage.getItem(SESSION_DB_PATH_KEY);
    console.log("[dbStore] savedPath from sessionStorage:", savedPath);
    if (!savedPath) {
      console.log("[dbStore] no savedPath, returning false");
      return false;
    }

    try {
      const isUnlocked = await dbIsUnlocked();
      console.log("[dbStore] dbIsUnlocked result:", isUnlocked);
      if (isUnlocked) {
        set({ isUnlocked: true, currentDbPath: savedPath, isFreshDb: false });
        console.log("[dbStore] restored unlock state, returning true");
        return true;
      }
    } catch (err) {
      console.error("[dbStore] dbIsUnlocked error:", err);
    }

    sessionStorage.removeItem(SESSION_DB_PATH_KEY);
    console.log("[dbStore] failed to restore, returning false");
    return false;
  },

  checkExistingDb: async () => {
    console.log("[dbStore] checkExistingDb start");
    try {
      const exists = await dbCheckAnyExists();
      console.log("[dbStore] dbCheckAnyExists result:", exists);
      set({ hasExistingDb: exists });
    } catch (err) {
      console.error("[dbStore] dbCheckAnyExists error:", err);
      set({ hasExistingDb: false });
    }
  },

  loadRecentDbs: async () => {
    console.log("[dbStore] loadRecentDbs start");
    try {
      const recent = await dbListRecent();
      console.log("[dbStore] dbListRecent result:", recent.length, "entries");
      set({ recentDbs: recent });
    } catch (err) {
      console.error("[dbStore] dbListRecent error:", err);
      set({ recentDbs: [] });
    }
  },
}));
