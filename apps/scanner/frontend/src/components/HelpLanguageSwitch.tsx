import { useHelpLanguage } from "../helpLanguage";
import { t, useLanguage } from "../i18n";

/**
 * English or Russian for the text SDD wrote (ADR-0034). One switch, shared
 * by every panel that shows such text, so the help under a code and the
 * description of a self test never disagree about which language they are
 * in. Ukrainian is not offered: SDD has none, and this project no longer
 * writes one.
 */
export function HelpLanguageSwitch() {
  const language = useLanguage();
  const [helpLanguage, setHelpLanguage] = useHelpLanguage(language);
  const option = (code: "eng" | "rus", label: string) => (
    <button
      type="button"
      className={`button button--small${helpLanguage === code ? " button--primary" : ""}`}
      aria-pressed={helpLanguage === code}
      onClick={() => setHelpLanguage(code)}
    >
      {t(label)}
    </button>
  );
  return (
    <div className="help-language" role="group" aria-label={t("Text language")}>
      <span className="module-validation">
        {t("SDD's own text, in English or Russian; there is no Ukrainian version.")}
      </span>
      {option("eng", "English")}
      {option("rus", "Russian")}
    </div>
  );
}
