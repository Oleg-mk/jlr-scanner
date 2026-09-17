import { useState } from "react";
import { decimalText, t } from "../i18n";
import {
  EMPTY_LIMITS,
  hasLimits,
  limitKey,
  numberOf,
  starterFor,
  toneFor,
  type LiveLimits,
} from "../liveLimits";
import type { LiveReadValue } from "../liveRead";
import { parameterName } from "../parameterNames";

/**
 * The chosen parameters as tiles: the name, the reading large with its unit,
 * where it came from, the run so far as a thin line, and the smallest and
 * largest seen. This is how a workshop tool shows live data — a number you
 * read at a glance, not a needle — and how the owner's own references lay
 * out everything that is not speed or engine speed.
 *
 * A tile colours itself only against limits the person has set (or the one
 * starter suggestion, marked as ours). SDD records no normal range, so
 * nothing here pretends to know one.
 */

interface LiveTilesProps {
  values: LiveReadValue[];
  limits: Record<string, LiveLimits>;
  onLimits: (key: string, limits: LiveLimits | null) => void;
}

/** A number as the language writes it, with the unit beside it. */
function shown(value: LiveReadValue): { number: string; unit: string | null } {
  if (value.value !== null) return { number: decimalText(value.value), unit: value.unit };
  if (value.state !== null) return { number: value.state, unit: null };
  if (value.raw !== null) return { number: String(value.raw), unit: null };
  return { number: "—", unit: null };
}

function compact(number: number): string {
  const text = Number.isInteger(number) ? String(number) : number.toFixed(2).replace(/\.?0+$/, "");
  return decimalText(text);
}

/** The run so far, scaled to what it has actually seen. */
function Sparkline({ series }: { series: Array<{ atMs: number; value: number }> }) {
  const values = series.map((point) => point.value);
  const low = Math.min(...values);
  const high = Math.max(...values);
  const span = high - low || 1;
  const first = series[0].atMs;
  const width = series[series.length - 1].atMs - first || 1;
  const points = series
    .map((point) => {
      const x = ((point.atMs - first) / width) * 100;
      const y = 22 - ((point.value - low) / span) * 20;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
  return (
    <svg
      viewBox="0 0 100 24"
      className="live-tile-line"
      preserveAspectRatio="none"
      role="img"
      aria-label={t("the run so far")}
    >
      <polyline points={points} />
    </svg>
  );
}

function LimitField({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number | null;
  onChange: (next: number | null) => void;
}) {
  return (
    <label className="live-limit-field">
      <span>{label}</span>
      <input
        type="number"
        step="any"
        value={value === null ? "" : value}
        onChange={(event) => {
          const text = event.target.value.trim().replace(",", ".");
          onChange(text === "" ? null : Number(text));
        }}
      />
    </label>
  );
}

export function LiveTiles({ values, limits, onLimits }: LiveTilesProps) {
  const [editing, setEditing] = useState<string | null>(null);

  return (
    <div className="live-tiles">
      {values.map((value) => {
        const key = limitKey(value);
        const own = limits[key];
        const starter = own === undefined ? starterFor(value) : null;
        const effective = own ?? starter ?? undefined;
        const number = numberOf(value);
        const tone = toneFor(number, effective);
        const { number: text, unit } = shown(value);
        const open = editing === key;
        const draft = own ?? starter ?? EMPTY_LIMITS;
        return (
          <article
            className={`live-tile live-tile--${tone}`}
            key={key}
            aria-label={parameterName(value.name)}
          >
            <h3 className="live-tile-name">{parameterName(value.name)}</h3>
            <p className="live-tile-reading">
              <strong>{text}</strong>
              {unit !== null ? <span className="live-tile-unit">{unit}</span> : null}
            </p>
            <p className="live-tile-source">
              {value.ecuFamily} · <code>{value.identifier}</code>
            </p>
            {value.series !== undefined && value.series.length > 1 ? (
              <Sparkline series={value.series} />
            ) : null}
            <p className="live-tile-seen">
              {value.minimum !== null && value.maximum !== null && value.minimum !== value.maximum
                ? t("over the run {low} … {high}", {
                    low: compact(value.minimum),
                    high: compact(value.maximum),
                  })
                : t("{count} reading(s)", { count: value.samples })}
            </p>
            <div className="live-tile-limits">
              {hasLimits(effective) ? (
                <span className="live-tile-limit-note">
                  {t("limits {low} … {high}", {
                    low: effective?.warnLow ?? effective?.alarmLow ?? "—",
                    high: effective?.warnHigh ?? effective?.alarmHigh ?? "—",
                  })}
                  {own === undefined && starter !== null ? ` · ${t("our suggestion")}` : ""}
                </span>
              ) : null}
              <button
                className="button button--quiet live-tile-limit-button"
                type="button"
                onClick={() => setEditing(open ? null : key)}
              >
                {open ? t("Done") : t("Limits")}
              </button>
            </div>
            {open ? (
              <div className="live-limit-editor">
                <LimitField
                  label={t("warn below")}
                  value={draft.warnLow}
                  onChange={(next) => onLimits(key, { ...draft, warnLow: next })}
                />
                <LimitField
                  label={t("warn above")}
                  value={draft.warnHigh}
                  onChange={(next) => onLimits(key, { ...draft, warnHigh: next })}
                />
                <LimitField
                  label={t("alarm below")}
                  value={draft.alarmLow}
                  onChange={(next) => onLimits(key, { ...draft, alarmLow: next })}
                />
                <LimitField
                  label={t("alarm above")}
                  value={draft.alarmHigh}
                  onChange={(next) => onLimits(key, { ...draft, alarmHigh: next })}
                />
                <button
                  className="button button--quiet"
                  type="button"
                  onClick={() => {
                    onLimits(key, null);
                    setEditing(null);
                  }}
                >
                  {t("Clear limits")}
                </button>
                <span className="button-hint">
                  {t("Your limits, kept on this machine. SDD records no normal range.")}
                </span>
              </div>
            ) : null}
          </article>
        );
      })}
    </div>
  );
}
