/**
 * The ECharts build this app actually uses.
 *
 * Importing `echarts` wholesale pulls every series type, the SVG renderer, the
 * geo/map data and the dataZoom tooling — roughly a megabyte in a bundle that
 * ships inside a Tauri binary and never leaves the disk. Only the series and
 * components used anywhere in the app are registered here, once, and imported
 * from this module instead of from the package root. Adding a chart type means
 * adding it to both lists below; forgetting one is a runtime "series not
 * exists" error, which is loud enough.
 *
 * `ChartView`/`install` order does not matter, but the renderer does: the app
 * only ever draws to `<canvas>`. `SVGRenderer` would be the better choice if
 * the charts had to be printed at arbitrary resolution — a canvas is rasterised
 * at the size it was painted — so charts are painted at the size they print at
 * (see the print block in `globals.css`).
 */
import * as echarts from "echarts/core";
import { BarChart, LineChart, PieChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  TitleComponent,
  TooltipComponent,
} from "echarts/components";
import { LegacyGridContainLabel } from "echarts/features";
import { CanvasRenderer } from "echarts/renderers";

echarts.use([
  BarChart,
  LineChart,
  PieChart,
  GridComponent,
  LegendComponent,
  // The donut puts its total in the middle, which is a title. Without this the
  // component is skipped and the number is simply never drawn — ECharts says so
  // once in the console, and the chart looks like it lost its centre.
  TitleComponent,
  TooltipComponent,
  CanvasRenderer,
]);

// ECharts 6 lays the grid out by its new `outerBounds` rules and warns that
// `containLabel` is legacy. The legacy layout is the one these charts were
// tuned against — the y-axis labels sit outside the plot and the bars start at
// the axis — so the old behaviour is what we register, deliberately.
echarts.use([LegacyGridContainLabel]);

export { echarts };
export type { EChartsCoreOption, EChartsType } from "echarts/core";
