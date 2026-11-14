/**
 * Shared chart chrome: axes, grid, tooltip, legend.
 *
 * Every chart in the app uses these instead of writing its own styling. The
 * reason is not brevity — it is that eight charts styled independently drift
 * apart, and a dashboard whose two charts disagree about the weight of a
 * hairline looks broken rather than hand-made. The values come from the theme
 * (`useChartColors`), so a preset switch repaints all of them at once.
 *
 * Two conventions worth stating, because they are decisions rather than taste:
 *
 * - **The value axis keeps the full number.** ECharts can abbreviate
 *   (`12.3k`), but a tick reading `1.2万` directly above one reading `9,000`
 *   puts two units on one axis and the reader has to notice. Full digits with
 *   grouping stay comparable at every magnitude.
 * - **Only horizontal gridlines.** Vertical ones add ink that no series uses —
 *   the categories are already separated by position.
 */
import type { ChartColors } from "@/utils/chartColors";

/** Motion is 120 ms elsewhere in the app; charts get a little longer to settle. */
const ANIMATION_MS = 180;

export function prefersReducedMotion(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

/** Grid inset. `right` leaves room for the last x label; `top` for the legend. */
export function chartGrid(options?: {
  top?: number;
  bottom?: number;
  left?: number;
  right?: number;
}) {
  return {
    left: options?.left ?? 4,
    right: options?.right ?? 12,
    top: options?.top ?? 30,
    bottom: options?.bottom ?? 4,
    containLabel: true,
  };
}

export function chartTooltip(
  colors: ChartColors,
  options?: {
    trigger?: "axis" | "item";
    formatter?: (params: unknown) => string;
  },
) {
  return {
    trigger: options?.trigger ?? "axis",
    formatter: options?.formatter,
    backgroundColor: colors.surface,
    borderColor: colors.border,
    borderWidth: 1,
    padding: [8, 10],
    textStyle: {
      color: colors.text,
      fontSize: 12,
      fontFamily: colors.sans,
    },
    extraCssText: "border-radius: 6px; box-shadow: none;",
    axisPointer: {
      type: "line",
      lineStyle: { color: colors.axis, width: 1, type: [2, 4] },
    },
  };
}

export function chartLegend(colors: ChartColors) {
  return {
    top: 0,
    right: 0,
    itemWidth: 8,
    itemHeight: 8,
    itemGap: 14,
    icon: "roundRect",
    textStyle: {
      color: colors.neutral,
      fontSize: 11,
      // Chinese in the mono face breaks column widths, so the legend — which
      // is nearly always Chinese — stays in the UI face.
      fontFamily: colors.sans,
    },
  };
}

/** Categories along the bottom: a baseline, no ticks, muted 11 px labels. */
export function categoryAxis(colors: ChartColors) {
  return {
    type: "category" as const,
    axisLine: { lineStyle: { color: colors.grid } },
    axisTick: { show: false },
    axisLabel: {
      color: colors.axis,
      fontSize: 11,
      fontFamily: colors.mono,
      hideOverlap: true,
    },
  };
}

/**
 * Values up the side: no axis line, no ticks, dashed gridlines only.
 * `formatter` receives the raw number and returns the tick text.
 */
export function valueAxis(
  colors: ChartColors,
  formatter?: (value: number) => string,
) {
  return {
    type: "value" as const,
    axisLine: { show: false },
    axisTick: { show: false },
    splitLine: {
      show: true,
      lineStyle: { color: colors.grid, width: 1, type: [2, 4] as number[] },
    },
    axisLabel: {
      color: colors.axis,
      fontSize: 11,
      fontFamily: colors.mono,
      formatter,
      margin: 8,
    },
  };
}

/** Area / line series default: no point markers, one and a half px stroke. */
export function lineSeriesDefaults(colors: ChartColors) {
  return {
    type: "line" as const,
    smooth: 0.2,
    showSymbol: false,
    symbolSize: 6,
    lineStyle: { width: 1.5 },
    emphasis: { focus: "series" as const },
    itemStyle: { color: colors.text },
  };
}

/** Text style root: everything inherits the UI face unless it says otherwise. */
export function chartTextStyle(colors: ChartColors) {
  return {
    fontFamily: colors.sans,
    color: colors.text,
  };
}

export function chartAnimation() {
  return {
    animation: !prefersReducedMotion(),
    animationDuration: ANIMATION_MS,
    animationEasing: "cubicOut" as const,
  };
}

/** Grouped-thousands tick formatter for a yuan axis. */
export function yuanTick(locale: string) {
  return (value: number) => value.toLocaleString(locale);
}
