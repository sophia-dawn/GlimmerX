/**
 * The ECharts build this app actually uses.
 *
 * Importing `echarts` wholesale pulls every series type, the SVG renderer, the
 * geo/map data and the dataZoom tooling — roughly a megabyte in a bundle that
 * ships inside a Tauri binary and never leaves the disk. Only four series and
 * four components are used anywhere in the app, so they are registered here
 * once and imported from this module instead of from the package root. Adding a
 * chart type means adding it to both lists below; forgetting to is a runtime
 * "series not exists" error, which is loud enough.
 *
 * `ChartView`/`install` order does not matter, but the renderer does: the app
 * only ever draws to `<canvas>`. `SVGRenderer` would be the better choice if the
 * charts had to be printed at arbitrary resolution — a canvas is rasterised at
 * the size it was painted — so charts are painted at the size they print at
 * (see the print block in `globals.css`).
 */
import * as echarts from "echarts/core";
import { BarChart, LineChart, PieChart } from "echarts/charts";
import {
  GridComponent,
  LegendComponent,
  MarkLineComponent,
  TooltipComponent,
} from "echarts/components";
import { CanvasRenderer } from "echarts/renderers";

echarts.use([
  BarChart,
  LineChart,
  PieChart,
  GridComponent,
  LegendComponent,
  MarkLineComponent,
  TooltipComponent,
  CanvasRenderer,
]);

export { echarts };
export type { EChartsCoreOption, EChartsType } from "echarts/core";
