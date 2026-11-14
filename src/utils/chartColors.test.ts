import { renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { resolveColor, useChartColors } from "./chartColors";

/**
 * The formats zrender's own colour parser understands
 * (`zrender/lib/tool/color.js`): hex, `rgb()` and `rgba()`. It knows nothing
 * about oklch, and ECharts rebuilds a series' emphasis style from that parser
 * (`liftColor`), so a colour it cannot read becomes `undefined` — which is
 * how a hovered line ends up with no stroke and disappears.
 */
const PARSEABLE =
  /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$|^rgba?\(/i;

describe("resolveColor", () => {
  it("never hands back a colour the parser cannot read", () => {
    // In the app the browser converts the value to the sRGB triple the canvas
    // would paint. jsdom has no canvas to convert with, so the fallback is
    // returned there — which is the same contract: something parseable.
    for (const value of [
      "rgb(1, 2, 3)",
      "#3f9c6d",
      "oklch(0.72 0.16 155)",
      "oklch(0.93 calc(0.019 * 1) 250)",
      "not-a-colour",
    ]) {
      expect(resolveColor(value, "#123456")).toMatch(PARSEABLE);
    }
  });

  it("falls back on an empty value", () => {
    expect(resolveColor("", "#123456")).toBe("#123456");
  });
});

describe("useChartColors", () => {
  it("hands ECharts only colours its own parser can read", () => {
    const { result } = renderHook(() => useChartColors());
    const colors = result.current;
    const every = [
      colors.income,
      colors.expense,
      colors.signal,
      colors.info,
      colors.text,
      colors.neutral,
      colors.surface,
      colors.border,
      colors.grid,
      colors.axis,
      ...colors.series,
    ];
    for (const color of every) {
      expect(color).toMatch(PARSEABLE);
    }
  });
});
