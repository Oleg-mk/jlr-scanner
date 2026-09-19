import { SwapLabel } from "./StableLabel";
import { useEffect, useRef, useState } from "react";
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
import { decimalsOf, withDecimals } from "../liveFormat";
import { withUnit } from "../units";
import { LiveChart } from "./LiveChart";
import { LiveDashboard, type DialControl, type Odometer } from "./LiveDashboard";
import { dialEntries } from "../useLiveReadController";
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
  /** The highest running total the mileage read found: the speedometer's window. */
  odometer?: Odometer | null;
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

/**
 * A figure in a slot of fixed width, padded with figure spaces - each the
 * width of a digit in tabular numerals - so a count that grows a digit
 * moves nothing beside it (the owner, 2026-09-19: the line of rounds and
 * readings twitched on every round).
 */
function slot(text: string, width: number): string {
  return text.padStart(width, " ");
}

/** Elapsed seconds, always to a tenth, in a slot for a ten-minute run. */
function seconds(ms: number): string {
  return slot((ms / 1000).toFixed(1), 5);
}

/** A round, in seconds to the hundredth, in a slot wide enough for a slow bus. */
function cadence(ms: number): string {
  return slot(`${(ms / 1000).toFixed(2)} s`, 7);
}

/** The value as the catalogue decodes it: a number and its unit, a named state, or the raw count. */
function reading(value: LiveReadValue): string {
  if (value.value !== null) {
    return withUnit(decimalText(value.value), value.unit);
  }
  if (value.state !== null) return value.state;
  if (value.raw !== null) return String(value.raw);
  return "—";
}

function span(value: LiveReadValue): string | null {
  if (value.minimum === null || value.maximum === null) return null;
  if (value.minimum === value.maximum) return null;
  const decimals = decimalsOf(value.value);
  const format = (number: number) => withDecimals(number, decimals);
  return `${format(value.minimum)} … ${format(value.maximum)}`;
}

const DASHBOARD_KEY = "prowlone.liveDashboard";

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
  odometer = null,
}: LiveReadPanelProps) {
  const language = useLanguage();
  const chosen = new Set(set.map(entryKey));
  // The two dials read themselves (ADR-0022, amendment of 2026-09-19): as
  // soon as the survey names the speed and the engine speed they join the
  // set, and once the adapter is there too the run starts without a click
  // and the panel shows - once per survey, so a run the person stopped
  // stays stopped. The caption under a dial is its switch; during a run a
  // click stops the run and starts it again with the set as it now is.
  const dials = dialEntries(modules);
  const dialList = [dials.speed, dials.engine].filter(
    (entry): entry is LiveReadEntryRequest => entry !== null,
  );
  const dialsChosen = dialList.length > 0 && dialList.every((entry) => chosen.has(entryKey(entry)));
  const seededFor = useRef<ModuleSurveyEntry[] | null>(null);
  const startedFor = useRef<ModuleSurveyEntry[] | null>(null);
  const restartPending = useRef(false);
  useEffect(() => {
    if (seededFor.current !== modules) {
      // A survey: seed the set, and let the next render see it.
      seededFor.current = modules;
      startedFor.current = null;
      const wanted = dialList.filter((entry) => !chosen.has(entryKey(entry)));
      if (wanted.length > 0 && !running) onChoose(wanted);
      return;
    }
    if (startedFor.current !== modules && dialsChosen && adapterReady && !running && !busy) {
      startedFor.current = modules;
      showDashboard();
      onStart();
      return;
    }
    if (restartPending.current && !running && !busy) {
      restartPending.current = false;
      if (set.length > 0) onStart();
    }
  });
  const switchDial = (entry: LiveReadEntryRequest) => {
    onToggle(entry);
    if (running) {
      restartPending.current = true;
      onStop();
    }
  };
  const dialControl = (entry: LiveReadEntryRequest | null): DialControl | null =>
    entry === null
      ? null
      : { reading: chosen.has(entryKey(entry)), disabled: busy, onToggle: () => switchDial(entry) };
  const offered = modules.filter((module) => module.readableIdentifiers.length > 0);
  const full = set.length >= LIVE_READ_MAX_ENTRIES;
  // Which identifiers the chooser offers. Quantities — what the catalogue
  // reads as a number with a real unit — come first, because that is what
  // a person watches; "everything" hides nothing. A library that marks
  // none offers everything rather than an empty list. Each choice says
  // what it holds for this car, counted (the owner, 2026-09-19).
  const [onlyQuantities, setOnlyQuantities] = useState(true);
  const isQuantity = (identifier: { quantity?: boolean }) => identifier.quantity === true;
  const quantityModules = offered.filter((module) =>
    module.readableIdentifiers.some(isQuantity),
  ).length;
  const quantityCount = offered.reduce(
    (total, module) => total + module.readableIdentifiers.filter(isQuantity).length,
    0,
  );
  const allCount = offered.reduce((total, module) => total + module.readableIdentifiers.length, 0);
  const anyQuantities = quantityModules > 0;
  const filtering = onlyQuantities && anyQuantities;
  const listed = (module: ModuleSurveyEntry) =>
    module.readableIdentifiers.filter((identifier) => !filtering || isQuantity(identifier));
  // What a module's button will actually put in: the listed addresses
  // not yet chosen, as many as the set has room for. The button says
  // that number, not the module's - on the owner's X250 the PCM offers
  // 81 and the set holds 16, which is what hid the engine speed behind
  // fifteen lower addresses (2026-09-19).
  const room = Math.max(0, LIVE_READ_MAX_ENTRIES - set.length);
  const fresh = (module: ModuleSurveyEntry) =>
    listed(module).filter(
      (identifier) =>
        !chosen.has(entryKey({ ecuFamily: module.ecuFamily, identifier: identifier.identifier })),
    );
  const taking = (module: ModuleSurveyEntry) => Math.min(fresh(module).length, room);
  const chooseReason = (module: ModuleSurveyEntry): string | undefined => {
    if (running) return undefined;
    if (full) return t("The set is full: {max} addresses.", { max: LIVE_READ_MAX_ENTRIES });
    if (listed(module).length > 0 && fresh(module).length === 0) return t("Already in the set.");
    return undefined;
  };
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
  // Rows that say something. A parameter that reads zero and has never
  // moved carries no information during a run, and a module can have dozens
  // of them; the table shows the informative rows by default and everything
  // on request (the owner, 2026-09-17).
  const [showAll, setShowAll] = useState(false);
  const informative = (value: LiveReadValue) => {
    const number = value.value !== null ? Number(value.value) : value.raw;
    if (number !== null && Number.isFinite(number) && number !== 0) return true;
    if (value.minimum !== null && value.maximum !== null && value.minimum !== value.maximum) return true;
    return value.state !== null && value.raw !== 0;
  };
  const shownValues = showAll ? snapshot.values : snapshot.values.filter(informative);
  const hiddenCount = snapshot.values.length - shownValues.length;
  // The instrument panel, on or off, remembered on this machine.
  const [dashboard, setDashboard] = useState<boolean>(() => {
    try {
      return window.localStorage.getItem(DASHBOARD_KEY) === "on";
    } catch {
      return false;
    }
  });
  const showDashboard = () =>
    setDashboard((current) => {
      if (!current) {
        try {
          window.localStorage.setItem(DASHBOARD_KEY, "on");
        } catch {
          // A browser that refuses storage forgets the choice; nothing else.
        }
      }
      return true;
    });
  const toggleDashboard = () =>
    setDashboard((current) => {
      const next = !current;
      try {
        window.localStorage.setItem(DASHBOARD_KEY, next ? "on" : "off");
      } catch {
        // A browser that refuses storage forgets the choice; nothing else.
      }
      return next;
    });
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
        <div className="live-read-filter" role="group" aria-label={t("Which parameters")}>
          <span className="live-read-filter__choice">
            <button
              className={`button button--quiet${onlyQuantities ? " is-current" : ""}`}
              type="button"
              aria-pressed={onlyQuantities}
              onClick={() => setOnlyQuantities(true)}
            >
              {t("Quantities")}
            </button>
            <span className="button-hint">
              {t(
                "Numbers with a unit — engine speed, road speed, temperatures, voltages, pressures, counters of time and distance. Addresses: {count}, in modules: {modules}.",
                { count: quantityCount, modules: quantityModules },
              )}
            </span>
          </span>
          <span className="live-read-filter__choice">
            <button
              className={`button button--quiet${onlyQuantities ? "" : " is-current"}`}
              type="button"
              aria-pressed={!onlyQuantities}
              onClick={() => setOnlyQuantities(false)}
            >
              {t("Everything")}
            </button>
            <span className="button-hint">
              {t(
                "Every address a module declares readable, states, raw counts, texts and blocks included. Addresses: {count}, in modules: {modules}.",
                { count: allCount, modules: offered.length },
              )}
            </span>
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
                  disabled={running || taking(module) === 0}
                  title={chooseReason(module)}
                  onClick={() =>
                    onChoose(
                      fresh(module).map((identifier) => ({
                        ecuFamily: module.ecuFamily,
                        identifier: identifier.identifier,
                      })),
                    )
                  }
                >
                  {filtering ? t("Choose these") : t("Choose all")}{" "}
                  <span className="count-mark">{taking(module)}</span>
                </button>
                {fresh(module).length > taking(module) ? (
                  <span className="button-hint">
                    {t(
                      "{left} more here than the set can take: it holds {max}; tick the rest by hand.",
                      { left: fresh(module).length - taking(module), max: LIVE_READ_MAX_ENTRIES },
                    )}
                  </span>
                ) : null}
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
            rounds: slot(String(snapshot.rounds), 4),
            samples: slot(String(snapshot.samples), 5),
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

      {snapshot.values.length > 0 && pinnedValues.length === 0 && plottedValues.length === 0 ? (
        <p className="button-hint">
          {t("Mark a row to see it in the table above or on the chart.")}
        </p>
      ) : null}

      {snapshot.values.length > 0 ? (
        <div className="live-marks-actions">
          <button
            className="button button--quiet"
            type="button"
            aria-pressed={showAll}
            onClick={() => setShowAll((current) => !current)}
          >
            <SwapLabel on={showAll} whenOn={t("Only informative")} whenOff={t("Show all")} />
          </button>
          {!showAll && hiddenCount > 0 ? (
            <span className="button-hint">{t("{count} rows at zero hidden", { count: hiddenCount })}</span>
          ) : null}
          <button
            className="button button--quiet"
            type="button"
            onClick={() => setPlotted(shownValues.map(markKey))}
          >
            {t("Mark all for the chart")}
          </button>
          {pinned.length > 0 || plotted.length > 0 ? (
            <button
              className="button button--quiet"
              type="button"
              onClick={() => {
                setPinned([]);
                setPlotted([]);
              }}
            >
              {t("Clear marks")}
            </button>
          ) : null}
          <button
            className="button button--quiet live-dash-toggle"
            type="button"
            aria-pressed={dashboard}
            onClick={toggleDashboard}
          >
            {t("Instrument panel")}
          </button>
        </div>
      ) : null}

      {dashboard && (snapshot.values.length > 0 || dials.speed !== null || dials.engine !== null) ? (
        <LiveDashboard
          values={snapshot.values}
          limits={limits}
          dials={{ speed: dialControl(dials.speed), engine: dialControl(dials.engine) }}
          odometer={odometer}
        />
      ) : null}

      {pinnedValues.length > 0 ? (
        <>
          <LiveTiles values={pinnedValues} limits={limits} onLimits={changeLimits} />
          <p className="button-hint">
            {t(
              "A cell of the table colours itself only against limits you set; SDD records no normal range, so none is drawn for you.",
            )}
          </p>
        </>
      ) : null}
      {plottedValues.length > 0 ? <LiveChart values={plottedValues} /> : null}
      {snapshot.values.length > 0 ? (
        <table className="module-table live-values">
          <thead>
            <tr>
              <th scope="col">{t("Parameter")}</th>
              <th scope="col">{t("Value")}</th>
              <th scope="col">{t("Min … max")}</th>
              <th scope="col">{t("Module")}</th>
              <th scope="col">{t("Show")}</th>
            </tr>
          </thead>
          <tbody>
            {shownValues.map((value) => {
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
                  {/* Two marks milled into one plate, like the language
                      switch in the header; each is its own lens, because a
                      row may be a tile and on the chart at once. */}
                  <div className="mark-switch" role="group" aria-label={t("Show")}>
                    <button
                      type="button"
                      className="mark-switch__mark"
                      aria-pressed={onTile}
                      disabled={!onTile && pinned.length >= 8}
                      onClick={() => setPinned((current) => flip(current, key))}
                    >
                      {t("Table")}
                    </button>
                    <button
                      type="button"
                      className="mark-switch__mark"
                      aria-pressed={onChart >= 0}
                      onClick={() => setPlotted((current) => flip(current, key))}
                    >
                      {t("Chart")}
                    </button>
                  </div>
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
