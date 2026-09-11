import { parameterNote, t, useLanguage } from "../i18n";
import type { MileageReading, MileageSurveySnapshot } from "../mileage";
import { StatusBadge } from "./StatusBadge";

interface MileagePanelProps {
  snapshot: MileageSurveySnapshot;
  running: boolean;
  adapterReady: boolean;
  surveyed: boolean;
  /** On the bench (ADR-0020): every reading is synthetic. */
  bench?: boolean;
}

function badge(snapshot: MileageSurveySnapshot, running: boolean, bench: boolean) {
  if (running) return <StatusBadge tone="pending">{t("Reading…")}</StatusBadge>;
  if (snapshot.state === "FINISHED") {
    return bench ? (
      <StatusBadge tone="pending">{t("SYNTHETIC")}</StatusBadge>
    ) : (
      <StatusBadge tone="positive">{t("Read")}</StatusBadge>
    );
  }
  return <StatusBadge tone="neutral">{t("Not started")}</StatusBadge>;
}

/** A whole number with thin spaces, the way a mileage is read. */
function grouped(value: number): string {
  return Math.round(value).toLocaleString("uk-UA").replace(/\u00a0/g, "\u202f");
}

/** What one module answered, or what it did instead. */
function reading(row: MileageReading): string {
  if (row.value !== null) return row.unit !== null ? `${row.value} ${row.unit}` : row.value;
  if (row.raw !== null) return String(row.raw);
  return "—";
}

/**
 * The difference from the highest running total on the car. Arithmetic over
 * two numbers this product read itself; the reader draws the conclusion.
 */
function difference(row: MileageReading): string {
  if (row.difference === null) return "—";
  if (row.difference === 0) return t("the highest");
  const sign = row.difference > 0 ? "+" : "−";
  return `${sign}${grouped(Math.abs(row.difference))}`;
}

export function MileagePanel({
  snapshot,
  running,
  adapterReady,
  surveyed,
  bench = false,
}: MileagePanelProps) {
  useLanguage();
  const current = snapshot.readings.filter((row) => row.kind === "CURRENT");
  const events = snapshot.readings.filter((row) => row.kind === "EVENT");

  const rows = (list: MileageReading[]) =>
    list.map((row) => (
      <tr key={`${row.ecuFamily}-${row.identifier}-${row.parameter}`}>
        <td>
          <strong>{row.ecuFamily}</strong>
        </td>
        <td>
          <code>{row.identifier}</code>
        </td>
        <td>
          {row.reason !== null ? (
            <span className="module-validation">{t("no answer")}</span>
          ) : row.negativeResponse !== null ? (
            <span className="module-validation">
              {t("the module declined: {code}", { code: row.negativeResponse })}
            </span>
          ) : (
            reading(row)
          )}
          {row.note !== null ? (
            <div className="module-validation">{parameterNote(row.note)}</div>
          ) : null}
          {row.kind === "EVENT" ? (
            <div className="module-validation">{row.parameter}</div>
          ) : null}
        </td>
        <td className="mileage-difference">{difference(row)}</td>
      </tr>
    ));

  return (
    <section className="mileage-panel" aria-labelledby="mileage-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Mileage")}</p>
          <h2 id="mileage-title">{t("What every module says about the distance")}</h2>
        </div>
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "A car keeps its mileage in dozens of modules, each counting for its own reasons. This asks every one of them the data describes, once, and puts the answers side by side. Read-only; nothing is written anywhere.",
        )}
      </p>

      {snapshot.state !== "IDLE" ? (
        <p className="button-hint" role="status">
          {t("{asked} of {planned} module reads, {answered} answered", {
            asked: snapshot.asked,
            planned: snapshot.planned,
            answered: snapshot.answered,
          })}
        </p>
      ) : null}
      {!adapterReady ? (
        <p className="button-hint">{t("Connect and verify the adapter, or the bench, first.")}</p>
      ) : !surveyed ? (
        <p className="button-hint">{t("Survey the vehicle first: the modules to ask come from it.")}</p>
      ) : null}
      {snapshot.error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{t(snapshot.error.message)}</h3>
          {snapshot.error.technicalDetails !== null ? (
            <code>{snapshot.error.technicalDetails}</code>
          ) : null}
        </div>
      ) : null}

      {snapshot.highest !== null ? (
        <p className="mileage-highest">
          {t("The highest reading on this car: {value} {unit}, from {module}", {
            value: grouped(snapshot.highest),
            unit: snapshot.unit ?? "",
            module: snapshot.highestModule ?? "",
          })}
        </p>
      ) : null}

      {current.length > 0 ? (
        <table className="module-table">
          <thead>
            <tr>
              <th scope="col">{t("Module")}</th>
              <th scope="col">{t("Address")}</th>
              <th scope="col">{t("Reading")}</th>
              <th scope="col">{t("Difference")}</th>
            </tr>
          </thead>
          <tbody>{rows(current)}</tbody>
        </table>
      ) : null}

      {events.length > 0 ? (
        <>
          <h3 className="mileage-subtitle">{t("Recorded at an event")}</h3>
          <p className="button-hint">
            {t(
              "These are not running totals: each is the odometer as it stood when the module recorded something. A reading above the highest total is an event the car has not driven to.",
            )}
          </p>
          <table className="module-table">
            <thead>
              <tr>
                <th scope="col">{t("Module")}</th>
                <th scope="col">{t("Address")}</th>
                <th scope="col">{t("Reading")}</th>
                <th scope="col">{t("Difference")}</th>
              </tr>
            </thead>
            <tbody>{rows(events)}</tbody>
          </table>
        </>
      ) : null}

      {snapshot.readings.length > 0 ? (
        <p className="button-hint mileage-caveat">
          {t(
            "Modules disagree for honest reasons too: a replaced cluster, a gearbox or an airbag module fitted after a repair all carry their own count. These are readings, not a conclusion.",
          )}
        </p>
      ) : null}
    </section>
  );
}
