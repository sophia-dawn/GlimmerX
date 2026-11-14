import { useTranslation } from "react-i18next";
import {
  ACTIVITY_DAYS,
  ACTIVITY_WEEKS,
  useActivityCalendar,
} from "@/hooks/useActivityCalendar";

const LEVELS = [0.06, 0.18, 0.32, 0.52, 0.78];

function cellPaint(count: number, max: number, future: boolean) {
  if (future) return "transparent";
  if (count === 0)
    return "color-mix(in srgb, var(--color-info) 6%, var(--color-card))";
  const level = Math.min(
    LEVELS.length - 1,
    Math.ceil((count / max) * LEVELS.length) - 1,
  );
  const alpha = LEVELS[level] ?? 0.06;
  return `color-mix(in srgb, var(--color-info) ${Math.round(alpha * 100)}%, var(--color-card))`;
}

/**
 * A calendar heatmap, not a decoration. 13 columns = 13 weeks on the 32 px
 * substrate, 7 rows = weekdays; the vertical major lines behind the page line
 * up with these columns. Blue-grey rather than amber on purpose — the signal
 * colour is budgeted for things that ask for attention, and "the shape of the
 * last quarter" is not one of them.
 *
 * No chart library here: 91 divs in a CSS grid render at zero cost, survive
 * print, and sit exactly on the substrate's 13 week columns — which a canvas
 * chart, painted at whatever pixel size its container happens to be, does not.
 * This is a grid, not a plot: there are no axes, no scales and no tooltip to
 * share with the ECharts build in `components/charts`.
 */
export function ActivityCalendar() {
  const { t } = useTranslation();
  const { data, isLoading } = useActivityCalendar();

  const cells =
    data?.cells ?? Array.from({ length: ACTIVITY_DAYS }, () => null);
  const max = data?.max ?? 1;

  return (
    <div className="flex w-full min-w-0 flex-col justify-center gap-1.5">
      <div className="flex items-baseline justify-between gap-2 text-xs text-muted-foreground">
        <span>{t("dashboard.activityCalendar")}</span>
        <span className="flex items-center gap-1 whitespace-nowrap">
          {t("dashboard.activityLess")}
          {LEVELS.map((level) => (
            <span
              key={level}
              aria-hidden
              className="h-1.5 w-1.5 rounded-[1px]"
              style={{
                background: `color-mix(in srgb, var(--color-info) ${Math.round(level * 100)}%, var(--color-card))`,
              }}
            />
          ))}
          {t("dashboard.activityMore")}
        </span>
      </div>

      <div
        role="img"
        aria-label={
          isLoading
            ? t("common.loading")
            : t("dashboard.activitySummary", {
                entries: data?.total ?? 0,
                days: ACTIVITY_DAYS,
              })
        }
        className="w-full"
        style={{
          display: "grid",
          gridAutoFlow: "column",
          // Fixed 14 px columns keep the cells square-ish; stretched `1fr`
          // columns turned each day into a 40 px bar and the calendar read as
          // a bar chart. `space-between` spreads the 13 weeks across the pane
          // so they still line up with the major substrate lines behind.
          gridTemplateColumns: `repeat(${ACTIVITY_WEEKS}, 14px)`,
          gridTemplateRows: "repeat(7, 8px)",
          justifyContent: "space-between",
          gap: "3px",
        }}
      >
        {cells.map((cell, i) => (
          <span
            key={cell?.date ?? i}
            title={
              cell && !cell.future ? `${cell.date} · ${cell.count}` : undefined
            }
            className="rounded-[1px]"
            style={{
              background: isLoading
                ? "var(--color-muted)"
                : cellPaint(cell?.count ?? 0, max, !!cell?.future),
            }}
          />
        ))}
      </div>

      <div className="flex justify-between text-xs text-muted-foreground">
        {(data?.weekLabels ?? ["", "", ""]).map((label, i) => (
          <span key={i} className="font-mono">
            {label}
          </span>
        ))}
      </div>
    </div>
  );
}
