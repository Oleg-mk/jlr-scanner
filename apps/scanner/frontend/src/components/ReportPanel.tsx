import { t, useLanguage } from "../i18n";
import type { SessionReportSnapshot } from "../sessionReport";

interface ReportPanelProps {
  snapshot: SessionReportSnapshot;
  error: string | null;
  saving: boolean;
  onSave: () => void;
}

/** What the session has recorded so far, and the one file it becomes. */
export function ReportPanel({ snapshot, error, saving, onSave }: ReportPanelProps) {
  useLanguage();
  return (
    <div className="report-panel">
      <p className="session-recorded">
        {t(
          "Recorded in this session: {captures} capture(s), {reads} module read(s), {calibrations} calibration read(s).",
          {
            captures: snapshot.captures,
            reads: snapshot.moduleReads,
            calibrations: snapshot.calibrationReads,
          },
        )}
      </p>
      <p className="operation-copy">
        {t(
          "One file with the survey, every capture and every read of this session. Send it, with a few lines about the car and the adapter, through the channel you received the build from.",
        )}
      </p>
      <div className="diagnostic-actions">
        <button
          className="button button--primary"
          type="button"
          onClick={onSave}
          disabled={!snapshot.reportAvailable || saving}
        >
          {saving ? t("Saving…") : t("Save session report")}
        </button>
      </div>
      {error ? (
        <p className="diagnostic-error" role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
