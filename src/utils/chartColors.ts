import { useMemo } from "react";
import { useThemeStore } from "@/stores/themeStore";

/**
 * Chart paint, read from the theme instead of hardcoded.
 *
 * ECharts paints into a `<canvas>`, which resolves neither `var()` nor CSS
 * classes, so the palette cannot live in CSS alone — it is read out of the
 * computed style and handed to the chart as literal values, keyed on the theme
 * so a switch repaints with the new ones.
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
  /** Body ink, for tooltip text and any label that is not an axis stub. */
  text: string;
  /** Muted ink — legends, axis labels, secondary values. */
  neutral: string;
  /** Floating surface, for the tooltip box. */
  surface: string;
  border: string;
  grid: string;
  axis: string;
  series: string[];
  /** Font stacks. Numerals get the mono face; Chinese must not. */
  mono: string;
  sans: string;
}

function readVar(styles: CSSStyleDeclaration, name: string, fallback: string) {
  const value = styles.getPropertyValue(name).trim();
  return value || fallback;
}

/**
 * The probe canvas used to let the browser resolve a colour.
 *
 * It is never attached to the document and never thrown away: one 1×1 canvas
 * costs nothing, and creating one per call would put pixel reads in a render
 * path.
 */
interface ProbeContext {
  fillStyle: string | CanvasGradient | CanvasPattern;
  clearRect: (x: number, y: number, w: number, h: number) => void;
  fillRect: (x: number, y: number, w: number, h: number) => void;
  getImageData: (
    x: number,
    y: number,
    w: number,
    h: number,
  ) => { data: Uint8ClampedArray };
}

let probe: ProbeContext | null | undefined;

function probeContext(): ProbeContext | null {
  if (probe !== undefined) return probe;
  let context: ProbeContext | null = null;
  try {
    if (typeof OffscreenCanvas !== "undefined") {
      // jsdom has no OffscreenCanvas, so the test environment never reaches
      // the noisy "getContext is not implemented" path.
      context = new OffscreenCanvas(1, 1).getContext("2d", {
        willReadFrequently: true,
      });
    } else {
      const canvas = document.createElement("canvas");
      canvas.width = 1;
      canvas.height = 1;
      context = canvas.getContext("2d", { willReadFrequently: true });
    }
  } catch {
    context = null;
  }
  probe = context;
  return context;
}

/**
 * Turn a token's value into a colour ECharts can actually read back.
 *
 * The canvas would happily paint `oklch()`, but ECharts does not hand colours
 * over as they are: zrender re-reads them with its own parser
 * (`zrender/lib/tool/color.js`), which knows hex, rgb, hsl and hsv and
 * nothing else. Our palette is `oklch()`, so `parse()` returns nothing for it
 * — and `lift()`, the function ECharts uses to rebuild the emphasis style,
 * returns `undefined` rather than the colour it failed to parse. The hovered
 * series then ends up with no stroke at all, which reads as a line that
 * disappears the moment the pointer arrives.
 *
 * Two things make the raw token value unusable beyond that: a custom
 * property's computed value keeps `calc()` unevaluated
 * (`oklch(0.23 calc(0.006 * 1) 250)`), and the browser is the only thing here
 * that knows how to turn that into a colour.
 *
 * So the browser does the conversion — but not through `getComputedStyle` or
 * `ctx.fillStyle`, both of which serialise a modern colour space back in its
 * own notation (`oklch(0.72 0.16 155)`), which is exactly what the parser
 * cannot read. Painting the colour into one pixel and reading the pixel back
 * gives the sRGB triple the canvas would actually use, in `rgb()` notation.
 */
export function resolveColor(value: string, fallback: string): string {
  if (!value || typeof document === "undefined") return fallback;
  const context = probeContext();
  if (!context) return fallback;

  // A value the browser rejects leaves the previous fill in place.
  const sentinel = "#010203";
  context.fillStyle = sentinel;
  context.fillStyle = value;
  if (context.fillStyle === sentinel && value.toLowerCase() !== sentinel) {
    return fallback;
  }

  context.clearRect(0, 0, 1, 1);
  context.fillStyle = value;
  context.fillRect(0, 0, 1, 1);
  const pixel = context.getImageData(0, 0, 1, 1).data;
  const alpha = pixel[3]! / 255;
  return alpha >= 1
    ? `rgb(${pixel[0]}, ${pixel[1]}, ${pixel[2]})`
    : `rgba(${pixel[0]}, ${pixel[1]}, ${pixel[2]}, ${alpha.toFixed(3)})`;
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
    const color = (name: string, fallback: string) =>
      resolveColor(readVar(styles, name, fallback), fallback);
    return {
      income: color("--color-income", "#3f9c6d"),
      expense: color("--color-expense", "#c0463f"),
      signal: color("--color-signal", "#c4761a"),
      info: color("--color-info", "#3f7f9c"),
      text: color("--color-foreground", "#1c2530"),
      neutral: color("--color-muted-foreground", "#7a889a"),
      surface: color("--color-popover", "#ffffff"),
      border: color("--color-border", "rgba(0,0,0,.12)"),
      grid: color("--chart-grid", "#39414d"),
      axis: color("--color-muted-foreground", "#7a889a"),
      series: Array.from({ length: 8 }, (_, i) =>
        color(`--chart-${i + 1}`, "#7a889a"),
      ),
      mono: readVar(styles, "--font-mono", "ui-monospace, monospace"),
      sans: readVar(styles, "--font-sans", "system-ui, sans-serif"),
    };
  }, [effectiveTheme, preset]);
  /* eslint-enable react-hooks/exhaustive-deps */
}
