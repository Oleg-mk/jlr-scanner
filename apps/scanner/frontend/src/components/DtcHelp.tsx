import { useHelpLanguage } from "../helpLanguage";
import { helpLines, t, useLanguage } from "../i18n";
import type { DtcSummary } from "../moduleRead";
import { HelpLanguageSwitch } from "./HelpLanguageSwitch";

/**
 * What the data says about a code, folded away. The words are SDD's own
 * (ADR-0034), in English or in Russian as the reader chooses; the library
 * decides which lines this car is given. A screen the library has no Russian
 * for is shown in English and says so. One component for every panel that
 * shows a code, so no panel can drift on its own.
 */
export function DtcHelp({ dtc }: { dtc: DtcSummary }) {
  const language = useLanguage();
  const [helpLanguage] = useHelpLanguage(language);
  if (dtc.help.length === 0) {
    return dtc.helpNote !== null ? (
      <div className="module-validation">{t(dtc.helpNote)}</div>
    ) : null;
  }
  const lines = helpLines(dtc.helpTexts, dtc.help, helpLanguage);
  const missing = helpLanguage !== "eng" && dtc.helpTexts?.[helpLanguage] === undefined;
  return (
    <details className="dtc-help">
      <summary>{t("What the data says about this code")}</summary>
      <HelpLanguageSwitch />
      {missing ? (
        <p className="module-validation">{t("This library carries no Russian text for it.")}</p>
      ) : null}
      {lines.map((line, index) => (
        <p key={`${index}-${line}`}>{line}</p>
      ))}
    </details>
  );
}
