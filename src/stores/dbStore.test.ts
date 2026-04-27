import { describe, it, expect, vi, beforeEach } from "vitest";
import { useDbStore } from "./dbStore";

// Mock API
vi.mock("@/utils/api", () => ({
  dbCheckAnyExists: vi.fn(),
  dbListRecent: vi.fn(),
}));

import { dbCheckAnyExists, dbListRecent } from "@/utils/api";
const mockDbCheckAnyExists = vi.mocked(dbCheckAnyExists);
const mockDbListRecent = vi.mocked(dbListRecent);

describe("dbStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const store = useDbStore.getState();
    store.setLocked();
  });

  it("initializes with locked state", () => {
    const store = useDbStore.getState();
    expect(store.isUnlocked).toBe(false);
    expect(store.currentDbPath).toBeNull();
    expect(store.isFreshDb).toBe(false);
    expect(store.hasExistingDb).toBeNull();
    expect(store.recentDbs).toEqual([]);
  });

  it("sets unlocked with database path", () => {
    useDbStore.getState().setUnlocked("/path/to/db.sqlite");
    const store = useDbStore.getState();
    expect(store.isUnlocked).toBe(true);
    expect(store.currentDbPath).toBe("/path/to/db.sqlite");
    expect(store.isFreshDb).toBe(false);
  });

  it("sets unlocked with fresh database flag", () => {
    useDbStore.getState().setUnlocked("/path/to/new.db", true);
    const store = useDbStore.getState();
    expect(store.isFreshDb).toBe(true);
  });

  it("locks the database", () => {
    useDbStore.getState().setUnlocked("/path/to/db.sqlite");
    useDbStore.getState().setLocked();
    const store = useDbStore.getState();
    expect(store.isUnlocked).toBe(false);
    expect(store.currentDbPath).toBeNull();
    expect(store.isFreshDb).toBe(false);
  });

  it("checks existing database", async () => {
    mockDbCheckAnyExists.mockResolvedValue(true);
    await useDbStore.getState().checkExistingDb();
    expect(useDbStore.getState().hasExistingDb).toBe(true);
  });

  it("handles check error gracefully", async () => {
    mockDbCheckAnyExists.mockRejectedValue(new Error("network error"));
    await useDbStore.getState().checkExistingDb();
    expect(useDbStore.getState().hasExistingDb).toBe(false);
  });

  it("loads recent databases", async () => {
    const recent = [
      { path: "/a.db", label: "A", lastOpened: "2026-01-01", exists: true },
    ];
    mockDbListRecent.mockResolvedValue(recent);
    await useDbStore.getState().loadRecentDbs();
    expect(useDbStore.getState().recentDbs).toEqual(recent);
  });

  it("handles loadRecentDbs error gracefully", async () => {
    mockDbListRecent.mockRejectedValue(new Error("failed"));
    await useDbStore.getState().loadRecentDbs();
    expect(useDbStore.getState().recentDbs).toEqual([]);
  });
});
