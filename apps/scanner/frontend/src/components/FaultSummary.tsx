import { SwapLabel } from "./StableLabel";
import { useState } from "react";
import { codeText, dataText, t, useLanguage } from "../i18n";
import { useHelpLanguage } from "../helpLanguage";
import { FAULT_CODE_OPERATIONS, type ModuleReadSnapshot } from "../moduleRead";
import type { ModuleSurveyEntry } from "../library";
import { StatusBadge } from "./StatusBadge";

interface FaultSummaryProps {
  /** Every module read this session has made, by module. */
  results: Record<string, ModuleReadSnapshot>;
  /** The surveyed modules, for the names SDD gives them. */
  modules: ModuleSurveyEntry[];
  /** Jump to a module's own panel. */
  onSelect: (ecuFamily: string) => void;
}

/**
 * What the car answered, in one place (2026-09-18).
 *
 * "Check all modules" asks every reachable module for its confirmed fault
 * codes and used to put the answers nowhere: they were kept per module and
 * shown only to whoever clicked that module. So a check of twenty-seven
 * modules that found two dozen codes looked exactly like a check that found
 * none, and the owner went through eight bench scenarios without seeing a
 * single code — the codes were there every time.
 *
 * This is the list the check was for. Modules that answered with codes,
 * their codes worded as the module's own panel words them, and a plain count
 * of what answered clean, so that "nothing found" is said rather than left
 * to be inferred from an empty screen.
 */
export function FaultSummary({ results, modules, onSelect }: FaultSummaryProps) {
  const language = useLanguage();
  const [helpLanguage] = useHelpLanguage(language);
  const [open, setOpen] = useState(true);

  // Every fault-code read this session made, whether the module answered or
  // not: `isFaultCodesRead` keeps only the successes, and a module that did
  // not answer is part of what the person asked for.
  const read = Object.values(results).filter(
    (outcome) =>
      outcome.state !== "IDLE" && FAULT_CODE_OPERATIONS.includes(outcome.operation),
  );
  if (read.length === 0) return null;

  const answered = read.filter((outcome) => outcome.state === "SUCCEEDED");
  const withCodes = answered
    .filter((outcome) => outcome.dtcs.length > 0)
    .sort((left, right) => left.ecuFamily.localeCompare(right.ecuFamily));
  const clean = answered.length - withCodes.length;
  const failed = read.length - answered.length;
  const codes = withCodes.reduce((total, outcome) => total + outcome.dtcs.length, 0);

  const nameOf = (ecuFamily: string) => {
    const module = modules.find((entry) => entry.ecuFamily === ecuFamily);
    if (module === undefined) return null;
    return dataText(module.names, module.name, language);
  };

  return (
    <section className="fault-summary" aria-labelledby="fault-summary-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("What the car answered")}</p>
          <h2 id="fault-summary-title">
            {codes === 0
              ? t("No fault codes")
              : t("{codes} fault code(s) in {modules} module(s)", {
                  codes,
                  modules: withCodes.length,
                })}
          </h2>
        </div>
        {codes > 0 ? (
          <button
            className="button button--quiet"
            type="button"
            aria-expanded={open}
            onClick={() => setOpen((shown) => !shown)}
          >
            <SwapLabel on={open} whenOn={t("Hide the codes")} whenOff={t("Show the codes")} />
          </button>
        ) : null}
      </div>

      <p className="survey-summary">
        {t("{read} module(s) asked, {clean} answered with nothing, {failed} did not answer.", {
          read: read.length,
          clean,
          failed,
        })}
      </p>

      {codes > 0 && open
        ? withCodes.map((outcome) => {
            const name = nameOf(outcome.ecuFamily);
            return (
              <div className="fault-summary-module" key={outcome.ecuFamily}>
                <div className="fault-summary-heading">
                  <button
                    className="button button--quiet"
                    type="button"
                    onClick={() => onSelect(outcome.ecuFamily)}
                  >
                    {outcome.ecuFamily}
                  </button>
                  {name !== null ? <span className="module-validation">{name}</span> : null}
                  <StatusBadge tone="negative">
                    {t("{count} code(s)", { count: outcome.dtcs.length })}
                  </StatusBadge>
                </div>
                <ul className="fault-summary-codes">
                  {outcome.dtcs.map((dtc) => {
                    const wording = codeText(
                      dtc.descriptionTexts,
                      dtc.description,
                      language,
                      dtc.descriptionDataTexts,
                      helpLanguage,
                    );
                    return (
                      <li key={`${dtc.code}-${dtc.failureType}`}>
                        <strong>{dtc.code}</strong>{" "}
                        {wording.shown ?? t("No wording in the loaded data")}
                      </li>
                    );
                  })}
                </ul>
              </div>
            );
          })
        : null}
    </section>
  );
}
