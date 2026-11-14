import { describe, expect, it } from "vitest";
import { chartTooltip, lineSeriesDefaults } from "./chartTheme";
import type { ChartColors } from "./chartColors";

const colors: ChartColors = {
  income: "#3f9c6d",
  expense: "#c0463f",
  signal: "#ffa02f",
  info: "#3f7f9c",
  text: "#e6edf3",
  neutral: "#7e8b9c",
  surface: "#1b2129",
  border: "#2a323d",
  grid: "#39414d",
  axis: "#7e8b9c",
  series: ["#7ba7c9", "#c9a06a", "#8bbf9f", "#b98fb5"],
  mono: "ui-monospace, monospace",
  sans: "system-ui, sans-serif",
};

describe("lineSeriesDefaults", () => {
  it("does not dim the other series on hover", () => {
    // ECharts' own default is `focus: "series"`, which pushes every other
    // series to 10% opacity. On a two-line chart that reads as the line having
    // been deleted, and every pointer move flips which series is dimmed —
    // measured as a 7% swing in drawn pixels while the cursor barely moves.
    expect(lineSeriesDefaults(colors).emphasis.focus).toBe("none");
  });
});

describe("chartTooltip", () => {
  it("keeps the box inside the canvas", () => {
    // Without confinement the box is placed against the cursor and overflows
    // the plot, where the card's rounded clip cuts it in half.
    expect(chartTooltip(colors).confine).toBe(true);
  });

  it("snaps instead of chasing the cursor", () => {
    // The default is a 0.4 s transform transition; each pointer move restarts
    // it towards a position that has already changed, so the box oscillates.
    expect(chartTooltip(colors).transitionDuration).toBe(0);
  });
});
