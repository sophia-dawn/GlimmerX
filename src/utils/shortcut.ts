/**
 * Accelerator helpers shared by the global shortcut and the in-app key handler.
 *
 * One string ("CommandOrControl+Shift+A") is the single source of truth: Tauri
 * registers it system-wide, and `matchesAccelerator` lets the same combination
 * work inside the window when registration failed or the app runs in a browser
 * during development.
 */

const MAC_PLATFORM = /Mac|iPhone|iPad|iPod/;

export function isMacPlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  return MAC_PLATFORM.test(navigator.platform || navigator.userAgent);
}

interface ParsedAccelerator {
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
  meta: boolean;
}

/**
 * Split `CommandOrControl+Shift+A` into its key and modifier flags.
 *
 * Unknown modifiers are ignored rather than rejected: an accelerator that Tauri
 * accepts but we cannot parse should degrade to "key alone", not silently bind
 * every key.
 */
export function parseAccelerator(accelerator: string): ParsedAccelerator {
  const parts = accelerator
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);

  const parsed: ParsedAccelerator = {
    key: "",
    ctrl: false,
    shift: false,
    alt: false,
    meta: false,
  };

  for (const part of parts) {
    const token = part.toLowerCase();
    switch (token) {
      case "commandorcontrol":
      case "cmdorctrl":
        // "Cmd on macOS, Ctrl elsewhere" — resolve it now so the matcher does
        // not have to know which platform it runs on.
        if (isMacPlatform()) {
          parsed.meta = true;
        } else {
          parsed.ctrl = true;
        }
        break;
      case "control":
      case "ctrl":
        parsed.ctrl = true;
        break;
      case "shift":
        parsed.shift = true;
        break;
      case "alt":
      case "option":
      case "altgr":
        parsed.alt = true;
        break;
      case "super":
      case "cmd":
      case "command":
      case "meta":
        parsed.meta = true;
        break;
      default:
        parsed.key = part;
    }
  }

  return parsed;
}

/** True when the keyboard event is exactly the configured combination. */
export function matchesAccelerator(
  event: KeyboardEvent,
  accelerator: string,
): boolean {
  const parsed = parseAccelerator(accelerator);
  if (!parsed.key) return false;

  if (parsed.ctrl !== event.ctrlKey) return false;
  if (parsed.shift !== event.shiftKey) return false;
  if (parsed.alt !== event.altKey) return false;
  if (parsed.meta !== event.metaKey) return false;

  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  return key.toUpperCase() === parsed.key.toUpperCase();
}

/** Human-readable form: `Ctrl+Shift+A` on Windows/Linux, `⌘⇧A` on macOS. */
export function displayAccelerator(accelerator: string): string {
  const parsed = parseAccelerator(accelerator);
  const mac = isMacPlatform();
  const parts: string[] = [];

  if (parsed.ctrl) parts.push(mac ? "⌃" : "Ctrl");
  if (parsed.meta) parts.push(mac ? "⌘" : "Win");
  if (parsed.alt) parts.push(mac ? "⌥" : "Alt");
  if (parsed.shift) parts.push(mac ? "⇧" : "Shift");
  if (parsed.key) parts.push(parsed.key.toUpperCase());

  return mac ? parts.join("") : parts.join("+");
}

/**
 * Build an accelerator from a keypress, for the settings recorder.
 *
 * Returns `null` when the press cannot be a global shortcut: a bare letter would
 * hijack normal typing, and modifier-only presses have no key to trigger on.
 */
export function acceleratorFromEvent(event: KeyboardEvent): string | null {
  if (["Control", "Shift", "Alt", "Meta", "CapsLock"].includes(event.key)) {
    return null;
  }
  if (!event.ctrlKey && !event.metaKey && !event.altKey) {
    return null;
  }

  const parts: string[] = [];
  // Record the platform-neutral name so the stored combination keeps working if
  // the ledger folder is copied to another OS.
  if (event.ctrlKey)
    parts.push(isMacPlatform() ? "Control" : "CommandOrControl");
  if (event.metaKey) parts.push("Command");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");

  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  parts.push(key);

  return parts.join("+");
}
