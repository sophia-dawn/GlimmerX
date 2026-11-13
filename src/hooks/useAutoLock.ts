import { useCallback, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useDbStore } from "@/stores/dbStore";
import { useSecurityStore } from "@/stores/securityStore";
import { lockSession } from "@/utils/session";

/** How often the idle clock is evaluated. */
const CHECK_INTERVAL_MS = 10_000;
/** Activity events are high-frequency; one timestamp update per second is plenty. */
const ACTIVITY_THROTTLE_MS = 1_000;
/** A gap this large means the process was suspended, not that the user was idle. */
const SLEEP_DRIFT_MS = CHECK_INTERVAL_MS * 2;

const ACTIVITY_EVENTS = [
  "pointerdown",
  "keydown",
  "wheel",
  "touchstart",
] as const;

export type IdleVerdict = "none" | "idle" | "suspend";

/**
 * Decide whether a tick should lock the ledger.
 *
 * Exported as a pure function so the two reasons stay testable without waiting
 * on real timers: `idle` is "no input for the configured time", `suspend` is
 * "the last tick is far older than the interval", i.e. the process was frozen.
 */
export function evaluateIdle(
  now: number,
  lastTick: number,
  lastActivity: number,
  autoLockMinutes: number,
): IdleVerdict {
  if (autoLockMinutes <= 0) return "none";
  if (now - lastTick > SLEEP_DRIFT_MS) return "suspend";
  if (now - lastActivity >= autoLockMinutes * 60_000) return "idle";
  return "none";
}

/**
 * Lock the ledger after a period without input, and optionally the moment the
 * window loses focus.
 *
 * Two details matter:
 *
 * 1. **Sleep detection.** `setInterval` does not fire while the machine is
 *    suspended, so a laptop closed for an hour wakes up with an idle clock that
 *    looks fresh. A tick that arrives much later than expected is treated as
 *    "we were away" and locks immediately.
 * 2. **Ordering.** Locking goes through `lockSession`, which drops the database
 *    key *before* clearing the cache and navigating. See that module.
 */
export function useAutoLock() {
  const navigate = useNavigate();
  const isUnlocked = useDbStore((state) => state.isUnlocked);
  const autoLockMinutes = useSecurityStore((state) => state.autoLockMinutes);
  const lockOnBlur = useSecurityStore((state) => state.lockOnBlur);

  const lastActivityRef = useRef(Date.now());
  const lastTickRef = useRef(Date.now());
  const lockingRef = useRef(false);

  const lock = useCallback(async () => {
    if (lockingRef.current) return;
    lockingRef.current = true;
    await lockSession();
    navigate("/unlock", { replace: true });
  }, [navigate]);

  // Re-arm whenever the ledger is (re)opened or the timeout changes, so
  // unlocking does not immediately lock again with a stale timestamp.
  useEffect(() => {
    if (!isUnlocked) return;
    lockingRef.current = false;
    lastActivityRef.current = Date.now();
    lastTickRef.current = Date.now();
  }, [isUnlocked, autoLockMinutes]);

  // Any real input counts as activity, including typing in a long form.
  useEffect(() => {
    if (!isUnlocked || autoLockMinutes <= 0) return;

    let lastRecorded = 0;
    const onActivity = () => {
      const now = Date.now();
      if (now - lastRecorded < ACTIVITY_THROTTLE_MS) return;
      lastRecorded = now;
      lastActivityRef.current = now;
    };

    for (const event of ACTIVITY_EVENTS) {
      window.addEventListener(event, onActivity, { passive: true });
    }
    return () => {
      for (const event of ACTIVITY_EVENTS) {
        window.removeEventListener(event, onActivity);
      }
    };
  }, [isUnlocked, autoLockMinutes]);

  // The idle clock.
  useEffect(() => {
    if (!isUnlocked || autoLockMinutes <= 0) return;

    const timer = window.setInterval(() => {
      const now = Date.now();
      const verdict = evaluateIdle(
        now,
        lastTickRef.current,
        lastActivityRef.current,
        autoLockMinutes,
      );
      lastTickRef.current = now;

      if (verdict !== "none") {
        void lock();
      }
    }, CHECK_INTERVAL_MS);

    return () => window.clearInterval(timer);
  }, [isUnlocked, autoLockMinutes, lock]);

  // Optional paranoid mode: leaving the window locks immediately.
  useEffect(() => {
    if (!isUnlocked || !lockOnBlur) return;
    const onBlur = () => void lock();
    window.addEventListener("blur", onBlur);
    return () => window.removeEventListener("blur", onBlur);
  }, [isUnlocked, lockOnBlur, lock]);
}
