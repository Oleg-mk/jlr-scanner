import { parameterNote, passportLabel, t, useLanguage } from "../i18n";
import { parameterName } from "../parameterNames";
import type { ModulePassportSnapshot, PassportReading } from "../passport";
import { StatusBadge } from "./StatusBadge";

interface PassportPanelProps {
  snapshot: ModulePassportSnapshot;
  running: boolean;
  adapterReady: boolean;
  surveyed: boolean;
  /** On the bench (ADR-0020): every reading is synthetic. */
  bench?: boolean;
}

function badge(snapshot: ModulePassportSnapshot, running: boolean, bench: boolean) {
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

/**
 * The label a reader sees for an identifier is this project's own wording
 * where it has one (ADR-0027, decision 5); SDD's name otherwise. SDD's name
 * stays underneath either way, because it is the row's identity in the report.
 */
function label(row: PassportReading): string {
  return passportLabel(row.identifier) ?? parameterName(row.parameter);
}

export function PassportPanel({
  snapshot,
  running,
  adapterReady,
  surveyed,
  bench = false,
}: PassportPanelProps) {
  useLanguage();
  const families = Array.from(new Set(snapshot.readings.map((row) => row.ecuFamily)));

  return (
    <section className="passport-panel" aria-labelledby="passport-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Module passport")}</p>
          <h2 id="passport-title">{t("What every module says it is")}</h2>
        </div>
        {badge(snapshot, running, bench)}
      </div>
      <p className="operation-copy">
        {t(
          "The part numbers fitted, the serial, the software and hardware levels — read from the identifiers the data declares for each module, once each. The text is shown as the module holds it; nothing is compared with a catalogue. Read-only; nothing is written anywhere.",
        )}
      </p>

      {snapshot.state !== "IDLE" ? (
        <p className="button-hint" role="status">
          {t("{asked} of {planned} identifier reads over {modules} modules, {answered} answered", {
            asked: snapshot.asked,
            planned: snapshot.planned,
            modules: snapshot.modules,
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

      {families.map((family) => (
        <div key={family} className="passport-module">
          <h3 className="passport-subtitle">{family}</h3>
          <table className="module-table">
            <thead>
              <tr>
                <th scope="col">{t("What")}</th>
                <th scope="col">{t("Address")}</th>
                <th scope="col">{t("The module holds")}</th>
              </tr>
            </thead>
            <tbody>
              {snapshot.readings
                .filter((row) => row.ecuFamily === family)
                .map((row) => (
                  <tr key={`${row.ecuFamily}-${row.identifier}-${row.parameter}`}>
                    <td>
                      <strong>{label(row)}</strong>
                      <div className="module-validation">{row.parameter}</div>
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
                      ) : row.value !== null ? (
                        <code className="passport-value">{row.value}</code>
                      ) : (
                        "—"
                      )}
                      {row.note !== null ? (
                        <div className="module-validation">{parameterNote(row.note)}</div>
                      ) : null}
                    </td>
                  </tr>
                ))}
            </tbody>
          </table>
        </div>
      ))}

      {snapshot.readings.length > 0 ? (
        <p className="button-hint passport-caveat">
          {t(
            "These are the texts the modules hold, as they hold them. Whether a software level is current is a question for a catalogue this product does not carry; nothing here says so.",
          )}
        </p>
      ) : null}
    </section>
  );
}
