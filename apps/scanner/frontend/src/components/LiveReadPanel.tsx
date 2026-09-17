import { useEffect, useState } from "react";
import { dataText, decimalText, liveReadReason, parameterNote, t, useLanguage } from "../i18n";
import { readLimits, writeLimits, type LiveLimits } from "../liveLimits";
import { parameterName } from "../parameterNames";
import type { ModuleSurveyEntry } from "../library";
import {
  LIVE_READ_MAX_ENTRIES,
  entryKey,
  type LiveReadEntryRequest,
  type LiveReadSnapshot,
  type LiveReadValue,
} from "../liveRead";
import { chartColour } from "../liveChartColours";
import { LiveChart } from "./LiveChart";
import { LiveTiles } from "./LiveTiles";
import { StatusBadge } from "./StatusBadge";

interface LiveReadPanelProps {
  snapshot: LiveReadSnapshot;
  /** The set as chosen, before a run turns it into entries. */
  set: LiveReadEntryRequest[];
  modules: ModuleSurveyEntry[];
  running: boolean;
  busy: boolean;
  adapterReady: boolean;
  onToggle: (entry: LiveReadEntryRequest) => void;
  /** Put a whole list into the set at once, up to what the set may hold. */
  onChoose: (entries: LiveReadEntryRequest[]) => void;
  onClearSet: () => void;
  onStart: () => void;
  onStop: () => void;
  /** Save the series as a spreadsheet (ADR-0022 §5, amended). */
  onSaveCsv: () => void;
  /** Where the last export went, or why it did not. */
  saved: { path?: string; error?: string } | null;
  /** On the bench (ADR-0020): every sample is synthetic. */
  bench?: boolean;
}

function badge(snapshot: LiveReadSnapshot, running: boolean, bench: boolean) {
  if (running) {
    return <StatusBadge tone="pending">{bench ? t("SYNTHETIC") : t("Reading…")}</StatusBadge>;
  }
  switch (snapshot.state) {
    case "STOPPED":
      return <StatusBadge tone="neutral">{t("Stopped")}</StatusBadge>;
    default:
      return <StatusBadge tone="neutral">{t("Not started")}</StatusBadge>;
  }
}

function seconds(ms: number): string {
  return (ms / 1000).toFixed(ms < 10_000 ? 1 : 0);
}

/** A round the way a tester reads it: milliseconds while it is quick, then seconds. */
function cadence(ms: number): string {
  return ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`;
}

/** The value as the catalogue decodes it: a number and its unit, a named state, or the raw count. */
function reading(value: LiveReadValue): string {
  if (value.value !== null) {
    const number = decimalText(value.value);
    return value.unit !== null ? `${number} ${value.unit}` : number;
  }
  if (value.state !== null) return value.state;
  if (value.raw !== null) return String(value.raw);
  return "—";
}

function span(value: LiveReadValue): string | null {
  if (value.minimum === null || value.maximum === null) return null;
  if (value.minimum === value.maximum) return null;
  const format = (number: number) =>
    decimalText(Number.isInteger(number) ? String(number) : number.toFixed(2).replace(/\.?0+$/, ""));
  return `${format(value.minimum)} … ${format(value.maximum)}`;
}

export function LiveReadPanel({
  snapshot,
  set,
  modules,
  running,
  busy,
  adapterReady,
  onToggle,
  onChoose,
  onClearSet,
  onStart,
  onStop,
  onSaveCsv,
  saved,
  bench = false,
}: LiveReadPanelProps) {
  const language = useLanguage();
  const chosen = new Set(set.map(entryKey));
  const offered = modules.filter((module) => module.readableIdentifiers.length > 0);
  const full = set.length >= LIVE_READ_MAX_ENTRIES;
  // Which identifiers the chooser offers. Quantities — what the catalogue
  // reads as a number with a unit — come first, because that is what a
  // person watches; "everything" hides nothing. A library that marks none
  // offers everything rather than an empty list.
  const [onlyQuantities, setOnlyQuantities] = useState(true);
  const anyQuantities = offered.some((module) =>
    module.readableIdentifiers.some((identifier) => identifier.quantity === true),
  );
  const filtering = onlyQuantities && anyQuantities;
  const listed = (module: ModuleSurveyEntry) =>
    module.readableIdentifiers.filter((identifier) => !filtering || identifier.quantity === true);
  // What the person marked in the table: which rows become tiles, which
  // rows go on the chart. The table is the one list; these are picks from
  // it, so nothing is shown twice.
  const [pinned, setPinned] = useState<string[]>([]);
  const [plotted, setPlotted] = useState<string[]>([]);
  const markKey = (value: LiveReadValue) => `${value.ecuFamily}|${value.identifier}|${value.name}`;
  const flip = (list: string[], key: string) =>
    list.includes(key) ? list.filter((held) => held !== key) : [...list, key];
  const pinnedValues = pinned
    .map((key) => snapshot.values.find((value) => markKey(value) === key))
    .filter((value): value is LiveReadValue => value !== undefined);
  const plottedValues = plotted
    .map((key) => snapshot.values.find((value) => markKey(value) === key))
    .filter((value): value is LiveReadValue => value !== undefined);
  // The person's own warning and alarm limits, kept on this machine.
  const [limits, setLimits] = useState<Record<string, LiveLimits>>({});
  useEffect(() => setLimits(readLimits()), []);
  const changeLimits = (key: string, next: LiveLimits | null) => {
    setLimits((current) => {
      const updated = { ...current };
      if (next === null) delete updated[key];
      else updated[key] = next;
      writeLimits(updated);
      return updated;
    });
  };

  return (
    <section className="live-read-panel" aria-labelledby="live-read-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Live reading")}</p>
          <h2 id="live-read-title">{t("The same reads, repeated")}</h2>
        </div>
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "Choose parameters from the modules the data describes and watch them: the application asks for each one in turn, round after round, and shows what it achieves. Read-only, one request at a time, and never faster than ten a second. A standing vehicle, engine running or not — not a moving one.",
        )}
      </p>

      {offered.length > 0 && anyQuantities ? (
        <div className="live-read-filter">
          <div className="live-read-filter-choice" role="group" aria-label={t("Which parameters")}>
            <button
              className={`button button--quiet${onlyQuantities ? " is-current" : ""}`}
              type="button"
              aria-pressed={onlyQuantities}
              onClick={() => setOnlyQuantities(true)}
            >
              {t("Quantities")}
            </button>
            <button
              className={`button button--quiet${onlyQuantities ? "" : " is-current"}`}
              type="button"
              aria-pressed={!onlyQuantities}
              onClick={() => setOnlyQuantities(false)}
            >
              {t("Everything")}
            </button>
          </div>
          <span className="button-hint">
            {onlyQuantities
              ? t("Those the catalogue reads as a number with a unit.")
              : t("Everything the module declares, raw counts and texts included.")}
          </span>
        </div>
      ) : null}

      {offered.length === 0 ? (
        <p className="button-hint">{t("Survey the vehicle first: the set is chosen from what the data describes.")}</p>
      ) : (
        <div className="live-read-set">
          {offered.map((module) => (
            <details key={module.ecuFamily} className="live-read-module">
              <summary>
                {module.ecuFamily}
                {dataText(module.names, module.name, language) !== null ? (
                  <span className="module-validation"> {dataText(module.names, module.name, language)}</span>
                ) : null}
              </summary>
              <div className="live-read-module-actions">
                <button
                  className="button button--quiet"
                  type="button"
                  disabled={running || full || listed(module).length === 0}
                  onClick={() =>
                    onChoose(
                      listed(module).map((identifier) => ({
                        ecuFamily: module.ecuFamily,
                        identifier: identifier.identifier,
                      })),
                    )
                  }
                >
                  {t("Choose these")}
                </button>
                <span className="button-hint">{t("{count} offered", { count: listed(module).length })}</span>
              </div>
              <table className="live-read-identifiers">
                <thead>
                  <tr>
                    <th scope="col" aria-label={t("Choose")} />
                    <th scope="col">{t("Address")}</th>
                    <th scope="col">{t("Description")}</th>
                  </tr>
                </thead>
                <tbody>
                  {listed(module).map((identifier) => {
                    const entry = {
                      ecuFamily: module.ecuFamily,
                      identifier: identifier.identifier,
                    };
                    const picked = chosen.has(entryKey(entry));
                    const id = `live-${module.ecuFamily}-${identifier.identifier}`;
                    return (
                      <tr key={identifier.identifier}>
                        <td>
                          <input
                            id={id}
                            type="checkbox"
                            checked={picked}
                            disabled={running || (!picked && full)}
                            onChange={() => onToggle(entry)}
                          />
                        </td>
                        <td>
                          {/* An address is an address in every language. */}
                          <label htmlFor={id}>
                            <code>{identifier.identifier}</code>
                          </label>
                        </td>
                        <td>
                          <label htmlFor={id}>
                            {identifier.parameters.map(parameterName).join(", ")}
                          </label>
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            </details>
          ))}
        </div>
      )}

      <div className="live-read-actions">
        <button
          className="button button--primary"
          type="button"
          disabled={!adapterReady || busy || running || set.length === 0}
          onClick={onStart}
        >
          {t("Start")}
        </button>
        <button className="button button--secondary" type="button" disabled={!running} onClick={onStop}>
          {t("Stop")}
        </button>
        {set.length > 0 && !running ? (
          <button className="button button--quiet" type="button" onClick={onClearSet}>
            {t("Clear the set")}
          </button>
        ) : null}
        {snapshot.samples > 0 && !running ? (
          <button className="button button--quiet" type="button" onClick={onSaveCsv}>
            {t("Save the series (CSV)")}
          </button>
        ) : null}
        <span className="button-hint">
          {t("{count} of {max} chosen", { count: set.length, max: LIVE_READ_MAX_ENTRIES })}
        </span>
      </div>
      {!adapterReady ? <p className="button-hint">{t("Connect and verify the adapter, or the bench, first.")}</p> : null}
      {saved?.path !== undefined ? (
        <p className="button-hint" role="status">
          {t("Series written to {path}", { path: saved.path })}
        </p>
      ) : null}
      {saved?.error !== undefined ? (
        <p className="button-hint" role="status">
          {saved.error}
        </p>
      ) : null}
      {snapshot.error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{t(snapshot.error.message)}</h3>
          {snapshot.error.technicalDetails !== null ? <code>{snapshot.error.technicalDetails}</code> : null}
        </div>
      ) : null}

      {snapshot.state !== "IDLE" ? (
        <p className="live-read-cadence" role="status">
          {t("{rounds} round(s), {samples} reading(s), {elapsed} s", {
            rounds: snapshot.rounds,
            samples: snapshot.samples,
            elapsed: seconds(snapshot.elapsedMs),
          })}
          {snapshot.roundMs !== null ? (
            <>
              {" · "}
              <strong>{t("a round every {round}", { round: cadence(snapshot.roundMs) })}</strong>
            </>
          ) : null}
          {snapshot.stoppedReason !== null ? ` · ${t(snapshot.stoppedReason)}` : null}
        </p>
      ) : null}

      {pinnedValues.length > 0 ? (
        <>
          <LiveTiles values={pinnedValues} limits={limits} onLimits={changeLimits} />
          <p className="button-hint">
            {t(
              "A tile colours itself only against limits you set; SDD records no normal range, so none is drawn for you.",
            )}
          </p>
        </>
      ) : null}
      {plottedValues.length > 0 ? <LiveChart values={plottedValues} /> : null}
      {snapshot.values.length > 0 && pinnedValues.length === 0 && plottedValues.length === 0 ? (
        <p className="button-hint">
          {t("Mark a row in the table to see it as a tile or on the chart.")}
        </p>
      ) : null}

      {snapshot.values.length > 0 ? (
        <table className="module-table">
          <thead>
            <tr>
              <th scope="col">{t("Parameter")}</th>
              <th scope="col">{t("Value")}</th>
              <th scope="col">{t("Seen")}</th>
              <th scope="col">{t("Module")}</th>
              <th scope="col">{t("Show")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.values.map((value) => {
              const key = markKey(value);
              const onChart = plotted.indexOf(key);
              const onTile = pinned.includes(key);
              return (
              <tr key={`${value.ecuFamily}-${value.identifier}-${value.name}`}>
                <td>
                  {onChart >= 0 ? (
                    <span
                      className="live-chart-swatch"
                      style={{ background: chartColour(onChart) }}
                      aria-hidden="true"
                    />
                  ) : null}
                  {parameterName(value.name)}
                  {value.note !== null ? (
                    <div className="module-validation">{parameterNote(value.note)}</div>
                  ) : null}
                </td>
                <td>
                  <strong>{reading(value)}</strong>
                </td>
                <td>
                  {span(value) ?? "—"}
                  <div className="module-validation">
                    {t("{count} reading(s)", { count: value.samples })}
                  </div>
                </td>
                <td>
                  {value.ecuFamily} <code>{value.identifier}</code>
                </td>
                <td className="live-marks">
                  <button
                    type="button"
                    className={`button button--quiet live-mark${onTile ? " is-on" : ""}`}
                    aria-pressed={onTile}
                    disabled={!onTile && pinned.length >= 8}
                    onClick={() => setPinned((current) => flip(current, key))}
                  >
                    {t("Tile")}
                  </button>
                  <button
                    type="button"
                    className={`button button--quiet live-mark${onChart >= 0 ? " is-on" : ""}`}
                    aria-pressed={onChart >= 0}
                    onClick={() => setPlotted((current) => flip(current, key))}
                  >
                    {t("Chart")}
                  </button>
                </td>
              </tr>
              );
            })}
          </tbody>
        </table>
      ) : null}

      {snapshot.entries.some((entry) => entry.dropped) ? (
        <ul className="live-read-dropped">
          {snapshot.entries
            .filter((entry) => entry.dropped)
            .map((entry) => (
              <li key={`${entry.ecuFamily}-${entry.identifier}`}>
                {entry.ecuFamily} <code>{entry.identifier}</code> —{" "}
                {entry.reason !== null ? liveReadReason(entry.reason) : t("dropped from the set")}
              </li>
            ))}
        </ul>
      ) : null}
    </section>
  );
}
