import { t, useLanguage } from "../i18n";
import type { DiagnosticSnapshot, SupportedVehicleProfile } from "../diagnostic";
import { StatusBadge } from "./StatusBadge";

interface DiagnosticPanelProps {
  snapshot: DiagnosticSnapshot;
  adapterReady: boolean;
  saving: boolean;
  onRead: () => void;
  onSaveReport: () => void;
}

function badge(snapshot: DiagnosticSnapshot) {
  switch (snapshot.state) {
    case "READY":
      return <StatusBadge tone="positive">{t("Ready")}</StatusBadge>;
    case "RUNNING":
      return <StatusBadge tone="pending">{t("Reading…")}</StatusBadge>;
    case "SUCCEEDED":
      return <StatusBadge tone="positive">{t("Complete")}</StatusBadge>;
    case "FAILED":
      return <StatusBadge tone="negative">{t("Failed")}</StatusBadge>;
    default:
      return <StatusBadge tone="neutral">{t("Adapter required")}</StatusBadge>;
  }
}

function evidenceVehicle(profile: SupportedVehicleProfile) {
  return `${profile.make} ${profile.model} / ${profile.vehicleProgram}, ${profile.modelYear}, ${profile.powertrain}`;
}

export function DiagnosticPanel({
  snapshot,
  adapterReady,
  saving,
  onRead,
  onSaveReport,
}: DiagnosticPanelProps) {
  useLanguage();
  const running = snapshot.state === "RUNNING";
  const evidence = evidenceVehicle(snapshot.profile);
  return (
    <section className="diagnostic-panel" aria-labelledby="diagnostic-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Standard OBD-II read")}</p>
          <h2 id="diagnostic-title">{t("Engine calibration identifier")}</h2>
        </div>
        {badge(snapshot)}
      </div>

      <p className="operation-copy">
        {t(
          "Asks the engine module for its calibration identifier with a standard OBD-II request (mode 09) on hs-can, the same on every car with CAN diagnostics. Nothing is transmitted until you press the button.",
        )}
      </p>
      <p className="validation-note">
        {t(
          "Evidence-backed so far on {vehicle} only; on other cars the answer follows the standard and is not yet confirmed.",
          { vehicle: evidence },
        )}
      </p>

      {snapshot.error !== null ? (
        <div className="error-banner diagnostic-error" role="alert">
          <h3>{snapshot.error.message}</h3>
          <p>{t("Review the adapter connection and save the report for support.")}</p>
          {snapshot.error.technicalDetails !== null ? (
            <details>
              <summary>{t("Technical details")}</summary>
              <code>{snapshot.error.technicalDetails}</code>
            </details>
          ) : null}
        </div>
      ) : null}

      <div className="diagnostic-result" aria-live="polite">
        <span>{t("Result")}</span>
        <strong>
          {snapshot.calibrationId === null
            ? "Calibration ID: —"
            : `Calibration ID: ${snapshot.calibrationId}`}
        </strong>
      </div>

      <div className="diagnostic-actions">
        <button
          className="button button--primary"
          type="button"
          onClick={onRead}
          disabled={!adapterReady || running}
        >
          {running ? t("Reading…") : t("Read Calibration ID")}
        </button>
        <button
          className="button button--secondary"
          type="button"
          onClick={onSaveReport}
          disabled={!snapshot.reportAvailable || saving}
        >
          {saving ? t("Saving…") : t("Save Diagnostic Report")}
        </button>
      </div>
      {!adapterReady ? (
        <p className="button-hint">{t("Connect and verify MongoosePro JLR to enable the read.")}</p>
      ) : null}
    </section>
  );
}
