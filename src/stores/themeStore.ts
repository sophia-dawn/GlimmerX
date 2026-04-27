import { create } from "zustand";

export type ThemeMode = "light" | "dark" | "system";
export type AccentColor =
  | "neutral"
  | "blue"
  | "green"
  | "purple"
  | "orange"
  | "rose"
  | "red"
  | "amber"
  | "yellow"
  | "lime"
  | "emerald"
  | "teal"
  | "cyan"
  | "sky"
  | "indigo"
  | "violet"
  | "fuchsia"
  | "pink";

export type AtmosphericTheme =
  | "none"
  | "warm"
  | "cool"
  | "ocean"
  | "forest"
  | "midnight"
  | "rose-atmosphere";

const THEME_KEY = "glimmerx:theme";
const COLOR_KEY = "glimmerx:color";
const ATMOSPHERE_KEY = "glimmerx:atmosphere";

function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

function resolveEffective(mode: ThemeMode): "light" | "dark" {
  return mode === "system" ? getSystemTheme() : mode;
}

function applyClasses(
  mode: ThemeMode,
  color: AccentColor,
  atmosphere: AtmosphericTheme,
) {
  const root = document.documentElement;
  const effective = resolveEffective(mode);

  root.classList.toggle("dark", effective === "dark");

  // Remove all color classes, add the selected one
  const colorClasses: AccentColor[] = [
    "neutral",
    "blue",
    "green",
    "purple",
    "orange",
    "rose",
    "red",
    "amber",
    "yellow",
    "lime",
    "emerald",
    "teal",
    "cyan",
    "sky",
    "indigo",
    "violet",
    "fuchsia",
    "pink",
  ];
  root.classList.remove(...colorClasses);
  if (color !== "neutral") {
    root.classList.add(color);
  }

  const atmosphereClasses: AtmosphericTheme[] = [
    "warm",
    "cool",
    "ocean",
    "forest",
    "midnight",
    "rose-atmosphere",
  ];
  root.classList.remove(...atmosphereClasses);
  if (atmosphere !== "none") {
    root.classList.add(atmosphere);
  }
}

interface ThemeState {
  mode: ThemeMode;
  color: AccentColor;
  atmosphere: AtmosphericTheme;
  effectiveTheme: "light" | "dark";
  setMode: (mode: ThemeMode) => void;
  setColor: (color: AccentColor) => void;
  setAtmosphere: (atmosphere: AtmosphericTheme) => void;
}

export const useThemeStore = create<ThemeState>((set) => {
  const savedMode =
    (localStorage.getItem(THEME_KEY) as ThemeMode | null) ?? "system";
  const savedColor =
    (localStorage.getItem(COLOR_KEY) as AccentColor | null) ?? "neutral";
  const savedAtmosphere =
    (localStorage.getItem(ATMOSPHERE_KEY) as AtmosphericTheme | null) ?? "none";
  const effective = resolveEffective(savedMode);

  // Apply on init
  applyClasses(savedMode, savedColor, savedAtmosphere);

  // Listen for system theme changes
  if (typeof window !== "undefined") {
    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", () => {
        set((state) => {
          if (state.mode === "system") {
            const newEffective = getSystemTheme();
            applyClasses("system", state.color, state.atmosphere);
            return { effectiveTheme: newEffective };
          }
          return state;
        });
      });
  }

  return {
    mode: savedMode,
    color: savedColor,
    atmosphere: savedAtmosphere,
    effectiveTheme: effective,
    setMode: (mode) => {
      localStorage.setItem(THEME_KEY, mode);
      applyClasses(mode, getColor(), getAtmosphere());
      const effective = resolveEffective(mode);
      set({ mode, effectiveTheme: effective });
    },
    setColor: (color) => {
      localStorage.setItem(COLOR_KEY, color);
      applyClasses(getMode(), color, getAtmosphere());
      set({ color });
    },
    setAtmosphere: (atmosphere) => {
      localStorage.setItem(ATMOSPHERE_KEY, atmosphere);
      applyClasses(getMode(), getColor(), atmosphere);
      set({ atmosphere });
    },
  };
});

// Helpers to read current state from store closure
function getMode(): ThemeMode {
  return (useThemeStore.getState().mode as ThemeMode) ?? "system";
}
function getColor(): AccentColor {
  return (useThemeStore.getState().color as AccentColor) ?? "neutral";
}
function getAtmosphere(): AtmosphericTheme {
  return (useThemeStore.getState().atmosphere as AtmosphericTheme) ?? "none";
}
