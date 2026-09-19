import { SwapLabel } from "./StableLabel";
import { Fragment, useState } from "react";
import { useReadOpen } from "../useReadOpen";
import { BATTERY_ROLES, type BatteryReadSnapshot, type BatteryRole } from "../battery";
import { parameterNote, t, useLanguage } from "../i18n";
import { parameterName } from "../parameterNames";
import { StatusBadge } from "./StatusBadge";
import { freshness, valueText } from "../batteryFormat";

interface BatteryPanelProps {
  snapshot: BatteryReadSnapshot;
  running: boolean;
  adapterReady: boolean;
  surveyed: boolean;
  /** On the bench (ADR-0020): every reading is synthetic. */
  bench?: boolean;
}

/** Our own short words for the groups; the rows keep SDD's own names. */
const ROLE_TITLE: Record<BatteryRole, string> = {
  CHARGE: "Charge",
  VOLTAGE: "Voltage",
  CURRENT: "Current",
  TEMPERATURE: "Temperature",
  DRAIN: "Parked drain",
  HEALTH: "Ageing",
  HISTORY: "What the car remembers",
  CONFIGURATION: "Declared",
  HYBRID: "Traction battery",
};

function badge(snapshot: BatteryReadSnapshot, running: boolean, bench: boolean) {
  if (running) return <StatusBadge tone="pending">{t("Reading…")}</StatusBadge>;
  if (snapshot.state === "FINISHED" && snapshot.readings.length > 0) {
    return bench ? (
      <StatusBadge tone="pending">{t("SYNTHETIC")}</StatusBadge>
    ) : (
      <StatusBadge tone="positive">{t("Read")}</StatusBadge>
    );
  }
  return null;
}

/**
 * Everything the battery monitor holds (ADR-0030), in the groups a person
 * asks for: how full, what it is doing now, what it leaks while parked, how
 * it has aged, what the car remembers, what it is declared to be, and the
 * traction battery of a hybrid. Thirty-odd rows on a real car, which is why
 * they live here and not on the session rail's card; the card carries the
 * level and the three readings a session turns on, and the button that
 * starts a run.
 *
 * The panel judges nothing. A value the data describes is the number it
 * describes; a value it does not is the bytes it is, with the decoder's own
 * reason beside it.
 */
export function BatteryPanel({
  snapshot,
  running,
  adapterReady,
  surveyed,
  bench = false,
}: BatteryPanelProps) {
  useLanguage();
  const [showSilent, setShowSilent] = useState(false);
  // Shown when the read arrives, away on one button: the rule the passports
  // and the mileage follow (the owner, 2026-09-19).
  const [open, setOpen] = useReadOpen(snapshot.readings.length);
  const answered = snapshot.readings.filter((row) => row.value !== null);
  const shown = showSilent ? snapshot.readings : answered;
  const silent = snapshot.readings.length - answered.length;
  const taken = freshness(snapshot.readUnixMs, Date.now());

  return (
    <section className="battery-panel" aria-labelledby="battery-panel-title">
      <div className="section-heading section-heading--action section-heading--pinned">
        <div>
          <p className="eyebrow">{t("Battery")}</p>
          <h2 id="battery-panel-title">{t("What the battery monitor holds")}</h2>
        </div>
        {snapshot.readings.length > 0 ? (
          <button
            className="button button--quiet"
            type="button"
            aria-expanded={open}
            onClick={() => setOpen((shown) => !shown)}
          >
            <SwapLabel on={open} whenOn={t("Put the readings away")} whenOff={t("Show the readings")} />
          </button>
        ) : null}
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "Every battery parameter the loaded data names for this car, module by module: how full it is, what it is doing now, what it leaks while parked, how it has aged, and what the car remembers about it. The cell on the plate carries the level and starts the read. Read-only; nothing is written anywhere.",
        )}
      </p>

      {snapshot.state !== "IDLE" ? (
        <p className="button-hint" role="status">
          {t("{asked} of {planned} reads, {answered} answered", {
            asked: snapshot.asked,
            planned: snapshot.planned,
            answered: snapshot.answered,
          })}
          {taken === null ? "" : ` · ${taken}`}
        </p>
      ) : null}
      {!adapterReady ? (
        <p className="button-hint">{t("Connect and verify the adapter, or the bench, first.")}</p>
      ) : !surveyed ? (
        <p className="button-hint">
          {t("Survey the vehicle first: the modules to ask come from it.")}
        </p>
      ) : null}
      {snapshot.error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{t(snapshot.error.message)}</h3>
          {snapshot.error.technicalDetails !== null ? (
            <details className="technical-details">
              <summary>{t("Technical details")}</summary>
              <code>{snapshot.error.technicalDetails}</code>
            </details>
          ) : null}
        </div>
      ) : null}

      {silent > 0 ? (
        <label className="check-option">
          <input
            type="checkbox"
            checked={showSilent}
            onChange={(event) => setShowSilent(event.target.checked)}
          />
          <span>{t("Show the {count} rows that answered nothing", { count: silent })}</span>
        </label>
      ) : null}

      {open ? (
        <>
      {/*
        One table, its groups as rows inside it, the way the configuration
        is drawn: a table per group gave every group its own column widths
        and its own heading row, and the eye had nothing to line up on
        (the owner, 2026-09-19).
      */}
      {shown.length > 0 ? (
            <table className="module-table ccf-table">
              <thead>
                <tr>
                  <th scope="col">{t("Parameter")}</th>
                  <th scope="col">{t("Value")}</th>
                  <th scope="col">{t("Module")}</th>
                </tr>
              </thead>
              <tbody>
      {BATTERY_ROLES.map((role) => {
        const rows = shown.filter((row) => row.role === role);
        if (rows.length === 0) return null;
        return (
          <Fragment key={role}>
                <tr className="ccf-group-row">
                  <th scope="rowgroup" colSpan={3}>
                    <h3 className="ccf-subtitle">{t(ROLE_TITLE[role])}</h3>
                  </th>
                </tr>
                {rows.map((row) => (
                  <tr key={`${row.ecuFamily}-${row.identifier}-${row.parameter}`}>
                    <td>
                      {parameterName(row.parameter)}
                      <div className="module-validation">{row.identifier}</div>
                    </td>
                    <td>
                      {valueText(row)}
                      {row.note !== null ? (
                        <div className="module-validation">{parameterNote(row.note)}</div>
                      ) : null}
                      {row.reason !== null ? (
                        <div className="module-validation">{row.reason}</div>
                      ) : null}
                    </td>
                    <td>
                      {row.ecuFamily}
                      <div className="module-validation">{row.routeValidation}</div>
                    </td>
                  </tr>
                ))}
          </Fragment>
        );
      })}
              </tbody>
            </table>
      ) : null}

      {snapshot.refused.length > 0 ? (
        <ul className="battery-refused">
          {snapshot.refused.map((row) => (
            <li key={row.ecuFamily}>
              <strong>{row.ecuFamily}</strong> {row.reason}
            </li>
          ))}
        </ul>
      ) : null}

        </>
      ) : null}

      <p className="module-validation">
        {bench
          ? t("Bench values: nothing here was measured on a car.")
          : t(
              "What the modules hold, as they hold it. This application states no threshold of its own and says nothing about whether the battery is good.",
            )}
      </p>
    </section>
  );
}
