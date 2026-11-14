import { describe, it, expect, beforeEach, vi } from "vitest";

/**
 * The store reads storage and repaints the document once, at module scope, so
 * the migration path cannot be reached through the singleton — each migration
 * case imports a fresh copy instead.
 */
async function loadStore() {
  vi.resetModules();
  return import("./themeStore");
}

function resetDocument() {
  const root = document.documentElement;
  root.className = "";
  delete root.dataset.preset;
}

describe("themeStore", () => {
  beforeEach(() => {
    localStorage.clear();
    resetDocument();
  });

  it("initializes with the system default and the terminal preset", async () => {
    const { useThemeStore } = await loadStore();
    const store = useThemeStore.getState();
    expect(store.mode).toBe("system");
    expect(store.preset).toBe("terminal");
    expect(store.effectiveTheme).toBe("light");
  });

  it("sets mode and persists to localStorage", async () => {
    const { useThemeStore } = await loadStore();
    useThemeStore.getState().setMode("dark");
    expect(useThemeStore.getState().mode).toBe("dark");
    expect(useThemeStore.getState().effectiveTheme).toBe("dark");
    expect(localStorage.getItem("glimmerx:theme")).toBe("dark");
  });

  it("paints the mode class on the document root", async () => {
    const { useThemeStore } = await loadStore();
    useThemeStore.getState().setMode("dark");
    expect(document.documentElement.classList.contains("dark")).toBe(true);
    useThemeStore.getState().setMode("light");
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
});

describe("presets", () => {
  beforeEach(() => {
    localStorage.clear();
    resetDocument();
  });

  it("applies the preset as a data attribute, not a class", async () => {
    const { useThemeStore } = await loadStore();
    useThemeStore.getState().setPreset("paper");
    expect(document.documentElement.dataset.preset).toBe("paper");
    expect(localStorage.getItem("glimmerx:preset")).toBe("paper");
  });

  it("keeps the previous preset when only the mode changes", async () => {
    const { useThemeStore } = await loadStore();
    useThemeStore.getState().setPreset("contrast");
    useThemeStore.getState().setMode("light");
    expect(useThemeStore.getState().preset).toBe("contrast");
    expect(document.documentElement.dataset.preset).toBe("contrast");
  });

  it("restores a saved preset on load", async () => {
    localStorage.setItem("glimmerx:preset", "terminal-cool");
    const { useThemeStore } = await loadStore();
    expect(useThemeStore.getState().preset).toBe("terminal-cool");
    expect(document.documentElement.dataset.preset).toBe("terminal-cool");
  });

  it("ignores a saved preset that no longer exists", async () => {
    localStorage.setItem("glimmerx:preset", "solarized");
    const { useThemeStore } = await loadStore();
    expect(useThemeStore.getState().preset).toBe("terminal");
  });
});

describe("legacy theme migration", () => {
  beforeEach(() => {
    localStorage.clear();
    resetDocument();
  });

  it("maps an old accent onto the nearest preset and drops the old keys", async () => {
    localStorage.setItem("glimmerx:color", "blue");
    localStorage.setItem("glimmerx:atmosphere", "none");

    const { useThemeStore } = await loadStore();

    expect(useThemeStore.getState().preset).toBe("terminal-cool");
    expect(localStorage.getItem("glimmerx:preset")).toBe("terminal-cool");
    // Deleted, not just ignored: a later change to the mapping must not revive
    // a choice the user has since replaced.
    expect(localStorage.getItem("glimmerx:color")).toBeNull();
    expect(localStorage.getItem("glimmerx:atmosphere")).toBeNull();
  });

  it("falls back to the atmosphere when the old accent was neutral", async () => {
    localStorage.setItem("glimmerx:color", "neutral");
    localStorage.setItem("glimmerx:atmosphere", "ocean");

    const { useThemeStore } = await loadStore();

    expect(useThemeStore.getState().preset).toBe("terminal-cool");
  });

  it("sends a warm atmosphere to the paper preset", async () => {
    localStorage.setItem("glimmerx:color", "neutral");
    localStorage.setItem("glimmerx:atmosphere", "warm");

    const { useThemeStore } = await loadStore();

    expect(useThemeStore.getState().preset).toBe("paper");
  });

  it("writes nothing for a first run", async () => {
    const { useThemeStore } = await loadStore();
    expect(useThemeStore.getState().preset).toBe("terminal");
    expect(localStorage.getItem("glimmerx:preset")).toBeNull();
  });
});
