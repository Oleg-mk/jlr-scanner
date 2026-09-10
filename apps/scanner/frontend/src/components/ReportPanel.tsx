import { t, useLanguage } from "../i18n";
import type { SessionReportSnapshot } from "../sessionReport";

interface ReportPanelProps {
  snapshot: SessionReportSnapshot;
  error: string | null;
  saving: boolean;
  onSave: () => void;
  /** On the bench (ADR-0020) the report is shown on screen and never saved. */
  bench?: boolean;
  preview?: string | null;
  onPreview?: () => void;
  onHidePreview?: () => void;
}

/** What the session has recorded so far, and the one file it becomes. */
export function ReportPanel({
  snapshot,
  error,
  saving,
  onSave,
  bench = false,
  preview = null,
  onPreview,
  onHidePreview,
}: ReportPanelProps) {
  useLanguage();
  return (
    <div className="report-panel">
      <p className="session-recorded">
        {t(
          "Recorded in this session: {captures} capture(s), {reads} module read(s), {calibrations} calibration read(s), {standard} standard OBD-II read(s), {live} live read run(s).",
          {
            captures: snapshot.captures,
            reads: snapshot.moduleReads,
            calibrations: snapshot.calibrationReads,
            standard: snapshot.standardObdReads,
            live: snapshot.liveReadRuns,
          },
        )}
      </p>
      <p className="operation-copy">
        {bench
          ? t(
              "Bench session: the report is shown on screen only and never saved. Bench data is illustrative, never evidence.",
            )
          : t(
              "One file with the survey, every capture and every read of this session. Send it, with a few lines about the car and the adapter, through the channel you received the build from.",
            )}
      </p>
      <div className="diagnostic-actions">
        {bench ? (
          <button
            className="button button--secondary"
            type="button"
            onClick={preview === null ? onPreview : onHidePreview}
            disabled={!snapshot.reportAvailable}
          >
            {preview === null ? t("Show the report on screen") : t("Hide the report")}
          </button>
        ) : (
          <button
            className="button button--primary"
            type="button"
            onClick={onSave}
            disabled={!snapshot.reportAvailable || saving}
          >
            {saving ? t("Saving…") : t("Save session report")}
          </button>
        )}
      </div>
      {bench && preview !== null ? (
        <pre className="report-preview" aria-label={t("Session report (bench, not saved)")}>
          {preview}
        </pre>
      ) : null}
      {error ? (
        <p className="diagnostic-error" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
