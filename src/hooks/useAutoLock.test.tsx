import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { renderHook, act } from "@testing-library/react";
import type { ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { evaluateIdle, useAutoLock } from "@/hooks/useAutoLock";
import { useDbStore } from "@/stores/dbStore";
import { useSecurityStore } from "@/stores/securityStore";

const mocks = vi.hoisted(() => ({
  lockSession: vi.fn(() => Promise.resolve()),
}));

vi.mock("@/utils/session", () => ({ lockSession: mocks.lockSession }));

function wrapper({ children }: { children: ReactNode }) {
  return <MemoryRouter>{children}</MemoryRouter>;
}

describe("evaluateIdle", () => {
  const MINUTE = 60_000;

  it("does nothing when auto-lock is off", () => {
    expect(evaluateIdle(10 * MINUTE, 10 * MINUTE - 10_000, 0, 0)).toBe("none");
  });

  it("reports idle once the timeout has elapsed", () => {
    expect(evaluateIdle(6 * MINUTE, 6 * MINUTE - 10_000, 0, 5)).toBe("idle");
  });

  it("reports suspend when the tick arrives far too late", () => {
    // Process was frozen: the previous tick is 5 minutes old, not 10 seconds.
    expect(evaluateIdle(5 * MINUTE, 0, 5 * MINUTE - 1, 30)).toBe("suspend");
  });

  it("stays silent while the user is active", () => {
    expect(
      evaluateIdle(5 * MINUTE, 5 * MINUTE - 10_000, 5 * MINUTE - 1_000, 5),
    ).toBe("none");
  });
});

describe("useAutoLock", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mocks.lockSession.mockClear();
    useDbStore.setState({ isUnlocked: true });
    useSecurityStore.setState({ autoLockMinutes: 5, lockOnBlur: false });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("locks after the configured idle time", async () => {
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5 * 60_000);
    });

    expect(mocks.lockSession).toHaveBeenCalledTimes(1);
  });

  it("does not lock before the timeout", async () => {
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(4 * 60_000);
    });

    expect(mocks.lockSession).not.toHaveBeenCalled();
  });

  it("stays silent when auto-lock is off", async () => {
    useSecurityStore.setState({ autoLockMinutes: 0 });
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(30 * 60_000);
    });

    expect(mocks.lockSession).not.toHaveBeenCalled();
  });

  it("resets the idle clock on user activity", async () => {
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(4 * 60_000);
    });
    act(() => {
      window.dispatchEvent(new Event("pointerdown"));
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(4 * 60_000);
    });

    expect(mocks.lockSession).not.toHaveBeenCalled();
  });

  it("locks on blur when that preference is enabled", async () => {
    useSecurityStore.setState({ autoLockMinutes: 0, lockOnBlur: true });
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      window.dispatchEvent(new Event("blur"));
    });

    expect(mocks.lockSession).toHaveBeenCalledTimes(1);
  });

  it("does nothing while the ledger is locked", async () => {
    useDbStore.setState({ isUnlocked: false });
    renderHook(() => useAutoLock(), { wrapper });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(60 * 60_000);
    });

    expect(mocks.lockSession).not.toHaveBeenCalled();
  });
});
