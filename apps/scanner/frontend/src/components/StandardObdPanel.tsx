import { codeText, t, useLanguage } from "../i18n";
import type {
  LabelledValue,
  StandardObdMonitor,
  StandardObdReadKind,
  StandardObdSnapshot,
  StandardObdValue,
} from "../standardObd";
import { StatusBadge } from "./StatusBadge";

interface StandardObdPanelProps {
  snapshot: StandardObdSnapshot;
  /** Every current-data value gathered so far, across reads. */
  values: StandardObdValue[];
  /** Freeze frame 0, gathered PID by PID. */
  frameValues: StandardObdValue[];
  monitors: StandardObdMonitor[];
  information: LabelledValue[];
  busy: boolean;
  adapterReady: boolean;
  responder: number;
  onResponderChange: (responder: number) => void;
  onRead: (kind: StandardObdReadKind, items?: string[]) => void;
  onReadEverything: () => void;
  onReadFreezeFrame: () => void;
  onReadMonitors: () => void;
  onReadVehicleInformation: () => void;
  onClear: () => void;
  /** On the bench (ADR-0020): every answer is synthetic. */
  bench?: boolean;
}

/** ISO 15765-4's eight responders; 0 is the engine controller almost everywhere. */
const RESPONDERS = [0, 1, 2, 3, 4, 5, 6, 7];

/** What each read is, in our words; the shell's own line stays in the details. */
const OPERATIONS: Record<StandardObdReadKind, string> = {
  CURRENT_DATA: "Current data",
  FREEZE_FRAME: "Freeze frame",
  STORED_DTCS: "Stored fault codes",
  PENDING_DTCS: "Pending fault codes",
  PERMANENT_DTCS: "Permanent fault codes",
  MONITOR_RESULTS: "Monitor results",
  VEHICLE_INFORMATION: "Vehicle information",
};

/** The PID whose text is the code that froze the frame; the codec says `none` when no code did. */
const FRAME_CODE_PID = "0x02";
const NO_CODE = "none";

function responderLabel(index: number) {
  const request = 0x7e0 + index;
  const name = index === 0 ? t("engine") : index === 1 ? t("transmission") : t("other");
  return `0x${request.toString(16).toUpperCase()} — ${name}`;
}

function badge(snapshot: StandardObdSnapshot, busy: boolean, bench: boolean) {
  if (busy) return <StatusBadge tone="pending">{t("Reading…")}</StatusBadge>;
  if (snapshot.state === "SUCCEEDED" && bench) {
    return <StatusBadge tone="pending">{t("SYNTHETIC")}</StatusBadge>;
  }
  switch (snapshot.state) {
    case "SUCCEEDED":
      return <StatusBadge tone="positive">{t("Answered")}</StatusBadge>;
    case "FAILED":
      return <StatusBadge tone="negative">{t("Failed")}</StatusBadge>;
    default:
      return <StatusBadge tone="neutral">{t("Not read yet")}</StatusBadge>;
  }
}

function formatNumber(value: number | null, fallback: string | null) {
  if (value === null) return fallback ?? "";
  return Number.isInteger(value) ? String(value) : value.toFixed(3).replace(/\.?0+$/, "");
}

/** The module's refusal as the shell words it: the code, then the standard's text. */
function refusal(text: string) {
  const space = text.indexOf(" ");
  if (space < 0) return <code>{text}</code>;
  return (
    <>
      <code>{text.slice(0, space)}</code> {t(text.slice(space + 1))}
    </>
  );
}

function valueCell(value: StandardObdValue) {
  if (value.kind === "raw") {
    return (
      <>
        <code>{value.rawHex}</code>
        <div className="module-validation">{t("raw: the layout is not carried")}</div>
      </>
    );
  }
  if (value.kind === "flag") return value.value === "yes" ? t("yes") : t("no");
  if (value.pid === FRAME_CODE_PID && value.value === NO_CODE) return t("no code froze the frame");
  if (value.kind === "text") return t(value.value ?? "");
  return value.value;
}

function valuesTable(rows: StandardObdValue[]) {
  return (
    <table className="module-table">
      <thead>
        <tr>
          <th scope="col">{t("Parameter")}</th>
          <th scope="col">{t("Value")}</th>
          <th scope="col">{t("Unit")}</th>
          <th scope="col">PID</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((value) => (
          <tr key={`${value.pid}-${value.name}`}>
            <td>{t(value.name)}</td>
            <td>{valueCell(value)}</td>
            <td>{value.unit}</td>
            <td>
              <code>{value.pid}</code>
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

export function StandardObdPanel({
  snapshot,
  values,
  frameValues,
  monitors,
  information,
  busy,
  adapterReady,
  responder,
  onResponderChange,
  onRead,
  onReadEverything,
  onReadFreezeFrame,
  onReadMonitors,
  onReadVehicleInformation,
  onClear,
  bench = false,
}: StandardObdPanelProps) {
  const language = useLanguage();
  const disabled = busy || !adapterReady;
  const gathered =
    values.length > 0 || frameValues.length > 0 || monitors.length > 0 || information.length > 0;
  const frameCode = frameValues.find((value) => value.pid === FRAME_CODE_PID)?.value ?? null;

  return (
    <section className="standard-obd-panel" aria-labelledby="standard-obd-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Standard OBD-II")}</p>
          <h2 id="standard-obd-title">{t("Legislated services, SAE J1979")}</h2>
        </div>
        {badge(snapshot, busy, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "What every OBD-II car answers at the same addresses, JLR or not: current data, fault codes, freeze frame, monitors and vehicle information. Read-only; nothing here clears a code.",
        )}
      </p>

      <div className="standard-obd-controls">
        <label className="field">
          <span>{t("Responder")}</span>
          <select
            value={responder}
            disabled={busy}
            onChange={(event) => onResponderChange(Number(event.target.value))}
          >
            {RESPONDERS.map((index) => (
              <option key={index} value={index}>
                {responderLabel(index)}
              </option>
            ))}
          </select>
        </label>
        <div className="standard-obd-buttons">
          <button className="button button--primary" type="button" disabled={disabled} onClick={onReadEverything}>
            {t("Read everything supported")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={() => onRead("STORED_DTCS")}>
            {t("Stored codes")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={() => onRead("PENDING_DTCS")}>
            {t("Pending codes")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={() => onRead("PERMANENT_DTCS")}>
            {t("Permanent codes")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={onReadFreezeFrame}>
            {t("Freeze frame")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={onReadMonitors}>
            {t("Monitors")}
          </button>
          <button className="button button--secondary" type="button" disabled={disabled} onClick={onReadVehicleInformation}>
            {t("Vehicle information")}
          </button>
          {gathered ? (
            <button className="button button--quiet" type="button" disabled={busy} onClick={onClear}>
              {t("Clear the tables")}
            </button>
          ) : null}
        </div>
      </div>
      {!adapterReady ? <p className="button-hint">{t("Connect and verify the adapter, or the bench, first.")}</p> : null}

      {snapshot.state !== "IDLE" && snapshot.kind !== null ? (
        <p className="standard-obd-operation">
          <strong>{t(OPERATIONS[snapshot.kind])}</strong>
          {snapshot.responder !== "" ? <span className="module-validation"> {snapshot.responder}</span> : null}
        </p>
      ) : null}
      {snapshot.error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{t(snapshot.error.message)}</h3>
          {snapshot.error.technicalDetails !== null ? <code>{snapshot.error.technicalDetails}</code> : null}
        </div>
      ) : null}
      {snapshot.negativeResponse !== null ? (
        <p className="standard-obd-refusal" role="status">
          {t("The module declined:")} {refusal(snapshot.negativeResponse)}
        </p>
      ) : null}

      {values.length > 0 ? (
        <>
          <h3 className="standard-obd-subtitle">{t("Current data")}</h3>
          {valuesTable(values)}
        </>
      ) : null}

      {frameValues.length > 0 ? (
        <>
          <h3 className="standard-obd-subtitle">
            {t("Freeze frame 0")}
            {frameCode !== null && frameCode !== NO_CODE ? ` — ${t("at code")} ${frameCode}` : ""}
          </h3>
          {valuesTable(frameValues)}
        </>
      ) : null}

      {snapshot.dtcKind !== null ? (
        snapshot.dtcs.length > 0 ? (
          <table className="module-table">
            <thead>
              <tr>
                <th scope="col">{t("Fault code")}</th>
                <th scope="col">{t("Description")}</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.dtcs.map((dtc) => {
                const wording = codeText(dtc.descriptionTexts, dtc.description, language);
                return (
                  <tr key={dtc.code}>
                    <td>
                      <strong>{dtc.code}</strong>
                    </td>
                    <td>
                      {wording.shown ?? t("No wording in the loaded data")}
                      {wording.original !== null ? <div className="module-validation">{wording.original}</div> : null}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        ) : (
          <p className="button-hint" role="status">
            {t("No fault codes of this kind.")}
          </p>
        )
      ) : null}

      {information.length > 0 ? (
        <dl className="detail-list">
          {information.map((item, index) => (
            <div key={`${item.label}-${index}`}>
              <dt>{t(item.label)}</dt>
              <dd>{item.value}</dd>
            </div>
          ))}
        </dl>
      ) : null}

      {monitors.length > 0 ? (
        <table className="module-table">
          <thead>
            <tr>
              <th scope="col">{t("Monitor")}</th>
              <th scope="col">{t("Value")}</th>
              <th scope="col">{t("Limits")}</th>
              <th scope="col">{t("Result")}</th>
            </tr>
          </thead>
          <tbody>
            {monitors.map((monitor) => (
              <tr key={`${monitor.mid}-${monitor.tid}`}>
                <td>
                  {t(monitor.monitor)}
                  <div className="module-validation">
                    MID {monitor.mid} · TID {monitor.tid}
                  </div>
                </td>
                <td>
                  {formatNumber(monitor.value, String(monitor.rawValue))} {monitor.unit}
                </td>
                <td>
                  {formatNumber(monitor.minimum, String(monitor.rawMinimum))} … {formatNumber(monitor.maximum, String(monitor.rawMaximum))}
                </td>
                <td>
                  {monitor.passed ? (
                    <StatusBadge tone="positive">{t("passed")}</StatusBadge>
                  ) : (
                    <StatusBadge tone="negative">{t("outside the limits")}</StatusBadge>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}

      {snapshot.supported.length > 0 && snapshot.kind === "MONITOR_RESULTS" && monitors.length === 0 ? (
        <p className="button-hint">
          {t("Monitors the module reports:")} {snapshot.supported.join(", ")}
        </p>
      ) : null}

      {snapshot.rawResponseHex !== null ? (
        <details>
          <summary>{t("Technical details")}</summary>
          <p>{snapshot.operation}</p>
          <p>
            {t("Request")}: <code>{snapshot.requestHex}</code>
          </p>
          <p>
            {t("Answer")}: <code>{snapshot.rawResponseHex}</code>
          </p>
        </details>
      ) : null}
    </section>
  );
}
