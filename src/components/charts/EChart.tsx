import { useCallback, useEffect, useRef } from "react";
import { echarts } from "./echarts";
import type { EChartsCoreOption, EChartsType } from "./echarts";
import { cn } from "@/lib/utils";

interface EChartProps {
  option: EChartsCoreOption;
  /** Pixel height, or any CSS length. Defaults to filling the parent box. */
  height?: number | string;
  className?: string;
  /** Described for screen readers; the canvas itself is opaque to them. */
  label: string;
}

/**
 * A canvas chart that survives React re-renders and layout changes.
 *
 * Three things make this more than an `echarts.init` call:
 *
 * 1. **Deferred init.** A chart mounted inside a flex column has no measurable
 *    box on its first commit. `init` on a 0×0 host paints nothing and, worse,
 *    keeps a 0×0 backing store until something resizes it — so the instance is
 *    created lazily from the first resize callback that reports real
 *    dimensions.
 * 2. **`notMerge`.** Options are rebuilt from scratch whenever the theme or the
 *    data changes. Merging would leave series from a previous, longer period
 *    behind, because ECharts merges series by index and does not know that four
 *    series became two.
 * 3. **Print.** A canvas is rasterised at the size it was painted, so paper
 *    needs a repaint at the paper size. `beforeprint` is where the print media
 *    query is already in effect but layout has not run yet, hence the explicit
 *    read of the container to force it.
 */
export function EChart({
  option,
  height = "100%",
  className,
  label,
}: EChartProps) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<EChartsType | null>(null);
  const optionRef = useRef(option);
  optionRef.current = option;

  const paint = useCallback(() => {
    const host = hostRef.current;
    if (!host) return;
    if (host.clientWidth === 0 || host.clientHeight === 0) return;
    if (!chartRef.current) {
      chartRef.current = echarts.init(host, undefined, {
        renderer: "canvas",
      });
    }
    chartRef.current.setOption(optionRef.current, { notMerge: true });
  }, []);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    paint();

    // `ResizeObserver` is present in every WebView2 / WKWebView the app runs in,
    // but not in jsdom — and a chart that throws on mount takes the whole page
    // down with it. Falling back to window resize keeps the component usable in
    // tests instead of requiring every caller to stub the global.
    const observer =
      typeof ResizeObserver === "undefined"
        ? null
        : new ResizeObserver(() => {
            if (chartRef.current) chartRef.current.resize();
            else paint();
          });
    observer?.observe(host);
    if (!observer) window.addEventListener("resize", paint);

    const repaint = () => {
      // Force the print layout before measuring, then repaint at paper size.
      void host.clientHeight;
      if (chartRef.current) chartRef.current.resize();
      else paint();
    };
    window.addEventListener("beforeprint", repaint);
    window.addEventListener("afterprint", repaint);

    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", paint);
      window.removeEventListener("beforeprint", repaint);
      window.removeEventListener("afterprint", repaint);
      chartRef.current?.dispose();
      chartRef.current = null;
    };
  }, [paint]);

  // Theme switches and new data both arrive as a new option object.
  useEffect(() => {
    paint();
  }, [option, paint]);

  return (
    <div
      ref={hostRef}
      role="img"
      aria-label={label}
      className={cn("echart w-full", className)}
      style={{ height }}
    />
  );
}
