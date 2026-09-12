import { t, useLanguage } from "../i18n";
import {
  REPORT_STYLES,
  type PrintableReport as ReportModel,
  type ReportSection,
} from "../printableReport";

interface PrintableReportProps {
  report: ReportModel;
}

function Facts({ rows }: { rows: Array<[string, string]> }) {
  return (
    <dl className="print-facts">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

function Section({ section }: { section: ReportSection }) {
  return (
    <section className="print-section">
      <h2>
        {section.title}
        <span className="print-worth">{section.worth}</span>
      </h2>
      <table>
        <thead>
          <tr>
            {section.columns.map((column) => (
              <th key={column} scope="col">
                {column}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {section.rows.map((row, index) => (
            <tr key={`${section.id}-${index}`}>
              {row.map((cell, column) => (
                <td key={`${section.id}-${index}-${column}`}>{cell}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {section.note === undefined ? null : <p className="print-note">{section.note}</p>}
    </section>
  );
}

/**
 * The session as a document (ADR-0031): the car, the library it was read
 * with, the modules and their reasons, and every read of the session. It
 * says on every section what its values are worth and draws no conclusion
 * of its own; the bundle beside it stays the evidence.
 */
export function PrintableReport({ report }: PrintableReportProps) {
  useLanguage();
  return (
    <article className="print-report" id="printable-report">
      <style>{REPORT_STYLES}</style>
      <header className="print-head">
        <h1>{t("Diagnostic session report")}</h1>
        <p className="print-sub">
          {t("ProwlOne {version} · build {build} · saved {saved}", {
            version: report.applicationVersion,
            build: report.applicationBuild,
            saved: report.savedAt,
          })}
        </p>
        {report.bench ? (
          <p className="print-bench">
            {t(
              "BENCH SESSION — a virtual vehicle answered from the data library. Every value in this document is synthetic and is evidence of nothing about any car.",
            )}
            {report.benchScenario === null ? "" : ` (${t("Scenario")} ${report.benchScenario})`}
          </p>
        ) : null}
      </header>

      <section className="print-section print-section--facts">
        <h2>{t("Vehicle")}</h2>
        <Facts rows={report.vehicle} />
        <h2>{t("Adapter")}</h2>
        <Facts rows={report.adapter} />
        <h2>{t("Data library")}</h2>
        <Facts rows={report.library} />
      </section>

      {report.survey === null ? null : <Section section={report.survey} />}
      {report.sections.map((section) => (
        <Section key={section.id} section={section} />
      ))}

      {report.empty ? (
        <p className="print-note">{t("This session recorded nothing yet.")}</p>
      ) : null}

      <footer className="print-foot">
        <p>
          {t(
            "This document shows what the modules answered. It states no threshold of its own, judges nothing, and nothing in it is vehicle-confirmed by being here: the session bundle beside it is the record.",
          )}
        </p>
      </footer>
    </article>
  );
}
