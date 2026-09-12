import { t, useLanguage } from "../i18n";
import type { PrintableReport as ReportModel } from "../printableReport";
import type { SessionReportSnapshot } from "../sessionReport";
import { PrintableReport } from "./PrintableReport";

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
  /**
   * The readable report (ADR-0031): the same session as a document to
   * print, save as a web page, and hand over.
   */
  document?: ReportModel | null;
  mask?: boolean;
  onMaskChange?: (masked: boolean) => void;
  onShowDocument?: () => void;
  onHideDocument?: () => void;
  onPrint?: () => void;
  onSaveDocument?: () => void;
  savedPath?: string | null;
  onReveal?: () => void;
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
  document = null,
  mask = false,
  onMaskChange,
  onShowDocument,
  onHideDocument,
  onPrint,
  onSaveDocument,
  savedPath = null,
  onReveal,
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
          <div className="report-document">
        <h3>{t("Readable report")}</h3>
        <p className="operation-copy">
          {t(
            "The same session as a document: the car, the library, the modules and every read, in the language of this window. Print it, or save it as a web page any browser turns into a PDF. The bundle beside it stays the record.",
          )}
        </p>
        <label className="check-option">
          <input
            type="checkbox"
            checked={mask}
            onChange={(event) => onMaskChange?.(event.target.checked)}
          />
          <span>{t("Mask the VIN and the adapter's serial in the document")}</span>
        </label>
        <div className="diagnostic-actions">
          <button
            className="button button--secondary"
            type="button"
            onClick={document === null ? onShowDocument : onHideDocument}
            disabled={!snapshot.reportAvailable}
          >
            {document === null ? t("Prepare the document") : t("Hide the document")}
          </button>
          <button
            className="button button--secondary"
            type="button"
            onClick={onPrint}
            disabled={!snapshot.reportAvailable}
          >
            {t("Print / save as PDF")}
          </button>
          <button
            className="button button--secondary"
            type="button"
            onClick={onSaveDocument}
            disabled={!snapshot.reportAvailable || bench}
            title={bench ? t("Bench session: nothing is written to disk.") : undefined}
          >
            {t("Save as a web page")}
          </button>
        </div>
        {savedPath === null ? null : (
          <p className="button-hint" role="status">
            {t("Document written to {path}", { path: savedPath })}{" "}
            <button className="button button--quiet" type="button" onClick={onReveal}>
              {t("Show in folder")}
            </button>
          </p>
        )}
      </div>
      {document === null ? null : (
        <div className="print-portal">
          <PrintableReport report={document} />
        </div>
      )}
    </div>
  );
}
