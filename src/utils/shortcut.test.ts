import { describe, it, expect } from "vitest";
import {
  parseAccelerator,
  matchesAccelerator,
  displayAccelerator,
  acceleratorFromEvent,
} from "@/utils/shortcut";

function keyEvent(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent("keydown", init);
}

describe("parseAccelerator", () => {
  it("splits modifiers and key", () => {
    const parsed = parseAccelerator("CommandOrControl+Shift+A");
    expect(parsed.key).toBe("A");
    expect(parsed.shift).toBe(true);
    // jsdom reports a non-mac platform, so CommandOrControl resolves to Ctrl.
    expect(parsed.ctrl).toBe(true);
    expect(parsed.meta).toBe(false);
  });

  it("treats unknown tokens as noise without dropping the key", () => {
    const parsed = parseAccelerator("CmdOrCtrl+Alt+F5");
    expect(parsed.key).toBe("F5");
    expect(parsed.alt).toBe(true);
  });
});

describe("matchesAccelerator", () => {
  it("matches the exact combination", () => {
    expect(
      matchesAccelerator(
        keyEvent({ key: "a", ctrlKey: true, shiftKey: true }),
        "Control+Shift+A",
      ),
    ).toBe(true);
  });

  it("is case-insensitive about the key", () => {
    expect(
      matchesAccelerator(
        keyEvent({ key: "A", ctrlKey: true, shiftKey: true }),
        "Control+Shift+a",
      ),
    ).toBe(true);
  });

  it("rejects a missing modifier", () => {
    expect(
      matchesAccelerator(
        keyEvent({ key: "a", ctrlKey: true }),
        "Control+Shift+A",
      ),
    ).toBe(false);
  });

  it("rejects an extra modifier", () => {
    expect(
      matchesAccelerator(
        keyEvent({ key: "a", ctrlKey: true, shiftKey: true, altKey: true }),
        "Control+Shift+A",
      ),
    ).toBe(false);
  });

  it("never matches an empty accelerator", () => {
    expect(matchesAccelerator(keyEvent({ key: "a" }), "")).toBe(false);
  });
});

describe("displayAccelerator", () => {
  it("renders Ctrl on non-mac platforms", () => {
    expect(displayAccelerator("CommandOrControl+Shift+A")).toBe("Ctrl+Shift+A");
  });

  it("keeps function keys as-is", () => {
    expect(displayAccelerator("Alt+F5")).toBe("Alt+F5");
  });
});

describe("acceleratorFromEvent", () => {
  it("builds a portable accelerator from a ctrl press", () => {
    expect(
      acceleratorFromEvent(
        keyEvent({ key: "k", ctrlKey: true, shiftKey: true }),
      ),
    ).toBe("CommandOrControl+Shift+K");
  });

  it("ignores bare letters so typing is never hijacked", () => {
    expect(acceleratorFromEvent(keyEvent({ key: "k" }))).toBeNull();
  });

  it("ignores modifier-only presses", () => {
    expect(
      acceleratorFromEvent(keyEvent({ key: "Shift", shiftKey: true })),
    ).toBeNull();
  });
});
