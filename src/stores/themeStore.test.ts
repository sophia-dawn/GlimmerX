import { describe, it, expect, beforeEach } from "vitest";

import { useThemeStore } from "./themeStore";

describe("themeStore", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("initializes with system default when no saved theme", () => {
    const store = useThemeStore.getState();
    expect(store.mode).toBe("system");
    expect(store.color).toBe("neutral");
    expect(store.effectiveTheme).toBe("light");
  });

  it("sets mode and persists to localStorage", () => {
    useThemeStore.getState().setMode("dark");
    expect(useThemeStore.getState().mode).toBe("dark");
    expect(useThemeStore.getState().effectiveTheme).toBe("dark");
    expect(localStorage.getItem("glimmerx:theme")).toBe("dark");
  });

  it("sets color and persists to localStorage", () => {
    useThemeStore.getState().setColor("green");
    expect(useThemeStore.getState().color).toBe("green");
    expect(localStorage.getItem("glimmerx:color")).toBe("green");
  });
});

describe("atmosphere", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("defaults to 'none'", () => {
    const { atmosphere } = useThemeStore.getState();
    expect(atmosphere).toBe("none");
  });

  it("persists atmosphere to localStorage", () => {
    useThemeStore.getState().setAtmosphere("warm");
    expect(localStorage.getItem("glimmerx:atmosphere")).toBe("warm");
  });

  it("applies atmosphere class to document root", () => {
    useThemeStore.getState().setAtmosphere("ocean");
    expect(document.documentElement.classList.contains("ocean")).toBe(true);
  });

  it("removes previous atmosphere class when changing", () => {
    useThemeStore.getState().setAtmosphere("warm");
    useThemeStore.getState().setAtmosphere("cool");
    expect(document.documentElement.classList.contains("warm")).toBe(false);
    expect(document.documentElement.classList.contains("cool")).toBe(true);
  });

  it("removes all atmosphere classes when set to 'none'", () => {
    useThemeStore.getState().setAtmosphere("forest");
    useThemeStore.getState().setAtmosphere("none");
    expect(document.documentElement.classList.contains("forest")).toBe(false);
  });
});
