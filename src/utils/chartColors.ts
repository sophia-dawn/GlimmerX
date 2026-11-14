import { useMemo } from "react";
import { useThemeStore } from "@/stores/themeStore";

/**
 * Chart paint, read from the theme instead of hardcoded.
 *
 * Recharts writes `fill` and `stroke` as SVG *presentation attributes*, and
 * those do not resolve `var()` — so the palette cannot live in CSS alone. It is
 * read out of the computed style instead, keyed on the theme so a switch
 * re-renders the charts with the new values.
 *
 * The series colours are a fixed eight-hue cycle defined in `globals.css`: low
 * enough in chroma that ten slices do not look like a highlighter set, ordered
 * so the first four (which carry most charts) are the most distinguishable.
 */
export interface ChartColors {
  income: string;
  expense: string;
  signal: string;
  info: string;
  neutral: string;
  grid: string;
  axis: string;
  series: string[];
}

function readVar(styles: CSSStyleDeclaration, name: string, fallback: string) {
  const value = styles.getPropertyValue(name).trim();
  return value || fallback;
}

export function useChartColors(): ChartColors {
  // Re-read on any change that repaints the token ladder.
  const effectiveTheme = useThemeStore((state) => state.effectiveTheme);
  const preset = useThemeStore((state) => state.preset);

  // `effectiveTheme` / `preset` are the trigger, not the source: the values come
  // out of the computed style, which only changes after the theme store has
  // repainted the class list and the `data-preset` attribute. Listing them is
  // therefore what makes the memo correct, even though the body never names them.
  /* eslint-disable react-hooks/exhaustive-deps -- the dependency pair is the
     theme switch because the palette is read from the computed style, not from
     those variables; naming them is what makes the memo re-run. */
  return useMemo(() => {
    const styles = getComputedStyle(document.documentElement);
    return {
      income: readVar(styles, "--color-income", "#3f9c6d"),
      expense: readVar(styles, "--color-expense", "#c0463f"),
      signal: readVar(styles, "--color-signal", "#c4761a"),
      info: readVar(styles, "--color-info", "#3f7f9c"),
      neutral: readVar(styles, "--color-muted-foreground", "#7a889a"),
      grid: readVar(styles, "--chart-grid", "#39414d"),
      axis: readVar(styles, "--color-muted-foreground", "#7a889a"),
      series: Array.from({ length: 8 }, (_, i) =>
        readVar(styles, `--chart-${i + 1}`, "#7a889a"),
      ),
    };
  }, [effectiveTheme, preset]);
  /* eslint-enable react-hooks/exhaustive-deps */
}
