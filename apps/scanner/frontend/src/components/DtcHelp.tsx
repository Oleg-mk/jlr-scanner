import { helpLines, t, useLanguage } from "../i18n";
import type { DtcSummary } from "../moduleRead";

/**
 * What the data says about a code, folded away. The words are this project's
 * own (ADR-0026); the library decides which lines this car is given. A line
 * we have no wording for stands in English where it belongs, rather than
 * being dropped or guessed at. One component for every panel that shows a
 * code, so no panel can drift back to English on its own.
 */
export function DtcHelp({ dtc }: { dtc: DtcSummary }) {
  const language = useLanguage();
  if (dtc.help.length === 0) {
    return dtc.helpNote !== null ? (
      <div className="module-validation">{t(dtc.helpNote)}</div>
    ) : null;
  }
  const lines = helpLines(dtc.helpTexts, dtc.help, language);
  return (
    <details className="dtc-help">
      <summary>{t("What the data says about this code")}</summary>
      {lines.map((line, index) => (
        <p key={`${index}-${line}`}>{line}</p>
      ))}
    </details>
  );
}
