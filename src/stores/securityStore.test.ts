import { describe, it, expect, beforeEach } from "vitest";
import {
  useSecurityStore,
  DEFAULT_AUTO_LOCK_MINUTES,
  DEFAULT_QUICK_ADD_SHORTCUT,
} from "@/stores/securityStore";

describe("securityStore", () => {
  beforeEach(() => {
    localStorage.clear();
    useSecurityStore.setState({
      autoLockMinutes: DEFAULT_AUTO_LOCK_MINUTES,
      quickAddShortcut: DEFAULT_QUICK_ADD_SHORTCUT,
    });
  });

  it("defaults to a 5 minute timeout", () => {
    const state = useSecurityStore.getState();
    expect(state.autoLockMinutes).toBe(5);
    expect(state.quickAddShortcut).toBe("CommandOrControl+Shift+A");
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

  it("persists a custom shortcut", () => {
    useSecurityStore.getState().setQuickAddShortcut("Control+Alt+J");
    expect(useSecurityStore.getState().quickAddShortcut).toBe("Control+Alt+J");
    expect(localStorage.getItem("glimmerx:security:quickAddShortcut")).toBe(
      "Control+Alt+J",
    );
  });
});
