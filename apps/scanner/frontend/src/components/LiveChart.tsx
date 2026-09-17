import { useState } from "react";
import { decimalText, t } from "../i18n";
import type { LiveReadValue } from "../liveRead";
import { parameterName } from "../parameterNames";

/**
 * One chart for the run: every chosen parameter that has readings, drawn
 * over the same stretch of time. Each line is scaled to its own smallest and
 * largest, the way a workshop tool merges graphs of different units onto one
 * screen — a voltage and an engine speed share the axis of time and nothing
 * else. The legend says what each colour is, its latest reading, and the
 * span the line covers; a click on a legend entry hides or shows its line.
 */

/** Ten colours a person tells apart on the plate; the eleventh repeats. */
const COLOURS = [
  "#0b63ce",
  "#c8362f",
  "#1d7a38",
  "#a66b00",
  "#6d3fb3",
  "#0e8a8a",
  "#b34a8f",
  "#5c6b1f",
  "#8a4b0e",
  "#3a4d6b",
];

const WIDTH = 100;
const HEIGHT = 40;

interface LiveChartProps {
  values: LiveReadValue[];
}

function compact(number: number): string {
  const text = Number.isInteger(number) ? String(number) : number.toFixed(2).replace(/\.?0+$/, "");
  return decimalText(text);
}

function seriesKey(value: LiveReadValue): string {
  return `${value.ecuFamily}|${value.identifier}|${value.name}`;
}

export function LiveChart({ values }: LiveChartProps) {
  const [hidden, setHidden] = useState<Set<string>>(new Set());
  const plotted = values.filter((value) => (value.series?.length ?? 0) > 1);
  if (plotted.length === 0) return null;

  // The stretch of time every line is drawn over: from the earliest reading
  // any of them has to the latest, so that the lines line up in time.
  const first = Math.min(...plotted.map((value) => value.series![0].atMs));
  const last = Math.max(...plotted.map((value) => value.series![value.series!.length - 1].atMs));
  const width = last - first || 1;

  const toggle = (key: string) =>
    setHidden((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });

  return (
    <figure className="live-chart">
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        preserveAspectRatio="none"
        className="live-chart-plot"
        role="img"
        aria-label={t("the run, every chosen parameter over time")}
      >
        {plotted.map((value, index) => {
          const key = seriesKey(value);
          if (hidden.has(key)) return null;
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
          return (
            <polyline
              key={key}
              points={points}
              stroke={COLOURS[index % COLOURS.length]}
              data-series={key}
            />
          );
        })}
      </svg>
      <figcaption className="live-chart-axis">
        <span>0 {t("sec")}</span>
        <span>{compact(Math.round(width / 100) / 10)} {t("sec")}</span>
      </figcaption>
      <ul className="live-chart-legend">
        {plotted.map((value, index) => {
          const key = seriesKey(value);
          const numbers = value.series!.map((point) => point.value);
          const off = hidden.has(key);
          return (
            <li key={key}>
              <button
                type="button"
                className={`live-chart-legend-item${off ? " is-off" : ""}`}
                aria-pressed={!off}
                onClick={() => toggle(key)}
              >
                <span
                  className="live-chart-swatch"
                  style={{ background: COLOURS[index % COLOURS.length] }}
                />
                <span className="live-chart-legend-name">{parameterName(value.name)}</span>
                <span className="live-chart-legend-reading">
                  {value.value !== null ? decimalText(value.value) : "—"}
                  {value.unit !== null ? ` ${value.unit}` : ""}
                </span>
                <span className="live-chart-legend-span">
                  {compact(Math.min(...numbers))} … {compact(Math.max(...numbers))}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
    </figure>
  );
}
