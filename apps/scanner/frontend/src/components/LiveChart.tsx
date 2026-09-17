import { decimalText, t } from "../i18n";
import { chartColour } from "../liveChartColours";
import type { LiveReadValue } from "../liveRead";

/**
 * One chart for the run: the parameters the person marked in the table,
 * drawn over the same stretch of time. Each line is scaled to its own
 * smallest and largest, the way a workshop tool merges graphs of different
 * units onto one screen — a voltage and an engine speed share the axis of
 * time and nothing else.
 *
 * There is no legend here on purpose. The table below the chart already
 * lists every parameter with its reading and its span; a legend repeated
 * that list and, with forty rows marked, buried the plot (the owner,
 * 2026-09-17). The table row carries the line's colour instead.
 */

const WIDTH = 100;
const HEIGHT = 40;

interface LiveChartProps {
  /** The parameters marked for the chart, in the order they were marked. */
  values: LiveReadValue[];
}

function compact(number: number): string {
  const text = Number.isInteger(number) ? String(number) : number.toFixed(2).replace(/\.?0+$/, "");
  return decimalText(text);
}

export function LiveChart({ values }: LiveChartProps) {
  const plotted = values.filter((value) => (value.series?.length ?? 0) > 1);
  if (plotted.length === 0) return null;

  // The stretch of time every line is drawn over: from the earliest reading
  // any of them has to the latest, so that the lines line up in time.
  const first = Math.min(...plotted.map((value) => value.series![0].atMs));
  const last = Math.max(...plotted.map((value) => value.series![value.series!.length - 1].atMs));
  const width = last - first || 1;

  return (
    <figure className="live-chart">
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        preserveAspectRatio="none"
        className="live-chart-plot"
        role="img"
        aria-label={t("the run, every marked parameter over time")}
      >
        {plotted.map((value) => {
          const index = values.indexOf(value);
          const series = value.series!;
          const numbers = series.map((point) => point.value);
          const low = Math.min(...numbers);
          const high = Math.max(...numbers);
          const span = high - low || 1;
          const points = series
            .map((point) => {
              const x = ((point.atMs - first) / width) * WIDTH;
              const y = HEIGHT - 2 - ((point.value - low) / span) * (HEIGHT - 4);
              return `${x.toFixed(2)},${y.toFixed(2)}`;
            })
            .join(" ");
          return <polyline key={`${value.ecuFamily}|${value.identifier}|${value.name}`} points={points} stroke={chartColour(index)} />;
        })}
      </svg>
      <figcaption className="live-chart-axis">
        <span>0 {t("sec")}</span>
        <span>
          {compact(Math.round(width / 100) / 10)} {t("sec")}
        </span>
      </figcaption>
    </figure>
  );
}
