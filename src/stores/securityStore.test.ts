import { describe, it, expect, beforeEach } from "vitest";
import {
  useSecurityStore,
  DEFAULT_AUTO_LOCK_MINUTES,
  clearRemovedSecurityKeys,
} from "@/stores/securityStore";

describe("securityStore", () => {
  beforeEach(() => {
    localStorage.clear();
    useSecurityStore.setState({
      autoLockMinutes: DEFAULT_AUTO_LOCK_MINUTES,
    });
  });

  it("defaults to a 5 minute timeout", () => {
    expect(useSecurityStore.getState().autoLockMinutes).toBe(5);
  });

  it("persists the timeout and accepts 0 as 'off'", () => {
    useSecurityStore.getState().setAutoLockMinutes(0);
    expect(useSecurityStore.getState().autoLockMinutes).toBe(0);
    expect(localStorage.getItem("glimmerx:security:autoLockMinutes")).toBe("0");
  });

  it("normalizes negative and non-finite timeouts to off", () => {
    useSecurityStore.getState().setAutoLockMinutes(-5);
    expect(useSecurityStore.getState().autoLockMinutes).toBe(0);
    useSecurityStore.getState().setAutoLockMinutes(Number.NaN);
    expect(useSecurityStore.getState().autoLockMinutes).toBe(0);
  });

  it("clears the keys of features that are gone", () => {
    // The window-focus lock and the quick-add accelerator were both removed;
    // a profile that still carries either key is carrying a preference for a
    // setting that no longer exists.
    localStorage.setItem("glimmerx:security:lockOnBlur", "true");
    localStorage.setItem(
      "glimmerx:security:quickAddShortcut",
      "CommandOrControl+Shift+A",
    );

    clearRemovedSecurityKeys();

    expect(localStorage.getItem("glimmerx:security:lockOnBlur")).toBeNull();
    expect(
      localStorage.getItem("glimmerx:security:quickAddShortcut"),
    ).toBeNull();
  });
});
