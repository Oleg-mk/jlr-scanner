import { useState } from "react";
import { BATTERY_ROLES, type BatteryReadSnapshot, type BatteryRole } from "../battery";
import { t, useLanguage } from "../i18n";
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
  const answered = snapshot.readings.filter((row) => row.value !== null);
  const shown = showSilent ? snapshot.readings : answered;
  const silent = snapshot.readings.length - answered.length;
  const taken = freshness(snapshot.readUnixMs, Date.now());

  return (
    <section className="battery-panel" aria-labelledby="battery-panel-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Battery")}</p>
          <h2 id="battery-panel-title">{t("What the battery monitor holds")}</h2>
        </div>
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "Every battery parameter the loaded data names for this car, module by module: how full it is, what it is doing now, what it leaks while parked, how it has aged, and what the car remembers about it. The card in the session column carries the level and the button that reads it. Read-only; nothing is written anywhere.",
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
            <code>{snapshot.error.technicalDetails}</code>
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

      {BATTERY_ROLES.map((role) => {
        const rows = shown.filter((row) => row.role === role);
        if (rows.length === 0) return null;
        return (
          <div className="battery-group" key={role}>
            <h3 className="ccf-subtitle">{t(ROLE_TITLE[role])}</h3>
            <table className="module-table">
              <thead>
                <tr>
                  <th scope="col">{t("Parameter")}</th>
                  <th scope="col">{t("Value")}</th>
                  <th scope="col">{t("Module")}</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((row) => (
                  <tr key={`${row.ecuFamily}-${row.identifier}-${row.parameter}`}>
                    <td>
                      {parameterName(row.parameter)}
                      <div className="module-validation">{row.identifier}</div>
                    </td>
                    <td>
                      {valueText(row)}
                      {row.note !== null ? (
                        <div className="module-validation">{row.note}</div>
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
              </tbody>
            </table>
          </div>
        );
      })}

      {snapshot.refused.length > 0 ? (
        <ul className="battery-refused">
          {snapshot.refused.map((row) => (
            <li key={row.ecuFamily}>
              <strong>{row.ecuFamily}</strong> {row.reason}
            </li>
          ))}
        </ul>
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
