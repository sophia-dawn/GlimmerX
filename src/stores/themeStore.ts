import { create } from "zustand";

export type ThemeMode = "light" | "dark" | "system";

/**
 * Four looks instead of eighteen accents times seven atmospheres.
 *
 * The old picker offered two independent axes that both rewrote the same two
 * variables, so most of the 126 combinations were variations of each other and
 * a dozen of them were unusable (a fuchsia accent on a rose tint). A preset
 * fixes the atmosphere and the one accent together, which is the only way they
 * were ever going to look deliberate. Each preset is four lines of CSS; adding
 * a fifth is an afternoon, not a palette audit.
 *
 * Mode stays a separate axis: a preset says *which* look, `mode` says whether
 * it is the night or the paper version of it.
 */
export type ThemePreset = "terminal" | "terminal-cool" | "paper" | "contrast";

export const THEME_PRESETS: readonly ThemePreset[] = [
  "terminal",
  "terminal-cool",
  "paper",
  "contrast",
];

const MODE_KEY = "glimmerx:theme";
const PRESET_KEY = "glimmerx:preset";
const LEGACY_COLOR_KEY = "glimmerx:color";
const LEGACY_ATMOSPHERE_KEY = "glimmerx:atmosphere";
const DEFAULT_MODE: ThemeMode = "system";
const DEFAULT_PRESET: ThemePreset = "terminal";

/**
 * Storage from before presets still exists in the wild, so it is read once,
 * mapped, and deleted. Deleting matters: if the mapping is ever adjusted, a
 * stale key must not resurrect a choice the user has since replaced.
 */
const LEGACY_ACCENT: Record<string, ThemePreset> = {
  neutral: "terminal",
  amber: "terminal",
  orange: "paper",
  yellow: "terminal",
  red: "paper",
  rose: "paper",
  pink: "paper",
  purple: "paper",
  fuchsia: "paper",
  blue: "terminal-cool",
  sky: "terminal-cool",
  cyan: "terminal-cool",
  teal: "terminal-cool",
  emerald: "terminal-cool",
  green: "terminal-cool",
  lime: "terminal-cool",
  indigo: "terminal-cool",
  violet: "terminal-cool",
};

const LEGACY_ATMOSPHERE: Record<string, ThemePreset> = {
  none: "terminal",
  cool: "terminal",
  midnight: "terminal",
  ocean: "terminal-cool",
  warm: "paper",
  forest: "paper",
  "rose-atmosphere": "paper",
};

function isPreset(value: string | null): value is ThemePreset {
  return value !== null && (THEME_PRESETS as readonly string[]).includes(value);
}

function readPreset(): ThemePreset {
  const saved = localStorage.getItem(PRESET_KEY);
  if (isPreset(saved)) return saved;

  const accent = localStorage.getItem(LEGACY_COLOR_KEY);
  const atmosphere = localStorage.getItem(LEGACY_ATMOSPHERE_KEY);
  if (accent === null && atmosphere === null) return DEFAULT_PRESET;

  // The accent was the more deliberate of the two old choices, but only when it
  // was not the neutral default — someone on `neutral` + `ocean` wants the tint,
  // not the accent.
  const migrated =
    (accent && accent !== "neutral" ? LEGACY_ACCENT[accent] : undefined) ??
    (atmosphere ? LEGACY_ATMOSPHERE[atmosphere] : undefined) ??
    DEFAULT_PRESET;

  localStorage.setItem(PRESET_KEY, migrated);
  localStorage.removeItem(LEGACY_COLOR_KEY);
  localStorage.removeItem(LEGACY_ATMOSPHERE_KEY);
  return migrated;
}

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function resolveEffective(mode: ThemeMode): "light" | "dark" {
  return mode === "system" ? getSystemTheme() : mode;
}

/**
 * The preset is an attribute, not a class: `paper`/`contrast` are words a
 * utility class could plausibly collide with, and one attribute is a single
 * place for the CSS to key on.
 */
function syncDocument(mode: ThemeMode, preset: ThemePreset) {
  const root = document.documentElement;
  root.classList.toggle("dark", resolveEffective(mode) === "dark");
  root.dataset.preset = preset;
}

interface ThemeState {
  mode: ThemeMode;
  preset: ThemePreset;
  effectiveTheme: "light" | "dark";
  setMode: (mode: ThemeMode) => void;
  setPreset: (preset: ThemePreset) => void;
}

export const useThemeStore = create<ThemeState>((set) => {
  const savedMode =
    (localStorage.getItem(MODE_KEY) as ThemeMode | null) ?? DEFAULT_MODE;
  const savedPreset = readPreset();
  const effective = resolveEffective(savedMode);

  // Apply on init, before the first paint of any component that reads tokens.
  syncDocument(savedMode, savedPreset);

  if (typeof window !== "undefined") {
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        set((state) => {
          if (state.mode !== "system") return state;
          syncDocument("system", state.preset);
          return { effectiveTheme: getSystemTheme() };
        });
      });
  }

  return {
    mode: savedMode,
    preset: savedPreset,
    effectiveTheme: effective,
    setMode: (mode) => {
      localStorage.setItem(MODE_KEY, mode);
      syncDocument(mode, useThemeStore.getState().preset);
      set({ mode, effectiveTheme: resolveEffective(mode) });
    },
    setPreset: (preset) => {
      localStorage.setItem(PRESET_KEY, preset);
      syncDocument(useThemeStore.getState().mode, preset);
      set({ preset });
    },
  };
});
