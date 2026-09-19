import { SwapLabel } from "./StableLabel";
import { useState } from "react";
import { codeText, dataText, t, useLanguage } from "../i18n";
import { useHelpLanguage } from "../helpLanguage";
import { FAULT_CODE_OPERATIONS, type ModuleReadSnapshot } from "../moduleRead";
import type { ModuleSurveyEntry } from "../library";
import type { ClearSequence } from "../useServiceModeController";
import { ConfirmDialog } from "./ConfirmDialog";
import { StatusBadge } from "./StatusBadge";

interface FaultSummaryProps {
  /** Every module read this session has made, by module. */
  results: Record<string, ModuleReadSnapshot>;
  /** The surveyed modules, for the names SDD gives them. */
  modules: ModuleSurveyEntry[];
  /** Jump to a module's own panel. */
  onSelect: (ecuFamily: string) => void;
  /**
   * The service mode (ADR-0036): while it is on, the one collective clear
   * is offered here, over every module on the list, after one question
   * that names them all.
   */
  serviceMode?: boolean;
  adapterReady?: boolean;
  clearing?: boolean;
  /** The collective clear as it runs, and what it came to. */
  sequence?: ClearSequence | null;
  /** The low-battery line, repeated inside the question (ADR-0030). */
  batteryNotice?: string | null;
  onClearAll?: (families: string[]) => void;
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
export function FaultSummary({
  results,
  modules,
  onSelect,
  serviceMode = false,
  adapterReady = false,
  clearing = false,
  sequence = null,
  batteryNotice = null,
  onClearAll,
}: FaultSummaryProps) {
  const language = useLanguage();
  const [helpLanguage] = useHelpLanguage(language);
  const [open, setOpen] = useState(true);
  // The one question before the collective clear (ADR-0036, decision 3).
  const [confirmAll, setConfirmAll] = useState(false);

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

      {serviceMode && onClearAll !== undefined && codes > 0 ? (
        <div className="mode-operation">
          <button
            className="button button--service"
            type="button"
            disabled={clearing || !adapterReady}
            onClick={() => setConfirmAll(true)}
          >
            {t("Clear the codes of all {count} modules", { count: withCodes.length })}
          </button>
          <span className="button-hint">
            {t("SERVICE_ROUTINE · one question, then each module in turn; every answer is recorded.")}
          </span>
        </div>
      ) : null}
      {sequence !== null && sequence.current !== null ? (
        <p className="survey-summary clear-sequence" role="status">
          {t("Clearing {module} — {done} of {total}…", {
            module: sequence.current,
            done: sequence.outcomes.length + 1,
            total: sequence.families.length,
          })}
        </p>
      ) : null}
      {sequence !== null && sequence.current === null ? (
        <div className="clear-sequence" role="status">
          <p className="survey-summary">
            {t("Cleared in turn: {cleared} accepted, {refused} refused, {failed} not made.", {
              cleared: sequence.outcomes.filter((outcome) => outcome.state === "CLEARED").length,
              refused: sequence.outcomes.filter((outcome) => outcome.state === "REFUSED").length,
              failed: sequence.outcomes.filter(
                (outcome) => outcome.state !== "CLEARED" && outcome.state !== "REFUSED",
              ).length,
            })}
          </p>
          {sequence.outcomes.some((outcome) => outcome.state !== "CLEARED") ? (
            <ul className="clear-outcome-codes">
              {sequence.outcomes
                .filter((outcome) => outcome.state !== "CLEARED")
                .map((outcome) => (
                  <li key={outcome.ecuFamily}>
                    {outcome.state === "REFUSED"
                      ? t("{module} refused: {refusal}", {
                          module: outcome.ecuFamily,
                          refusal: outcome.refusal ?? "—",
                        })
                      : t("{module}: not made — {message}", {
                          module: outcome.ecuFamily,
                          message: outcome.error !== null ? t(outcome.error.message) : "—",
                        })}
                  </li>
                ))}
            </ul>
          ) : null}
        </div>
      ) : null}
      {confirmAll ? (
        <ConfirmDialog
          title={t("Clear the fault codes of {count} modules", { count: withCodes.length })}
          confirmLabel={t("Clear the codes of {count} modules", { count: withCodes.length })}
          cancelLabel={t("Cancel")}
          onConfirm={() => {
            setConfirmAll(false);
            onClearAll?.(withCodes.map((outcome) => outcome.ecuFamily));
          }}
          onCancel={() => setConfirmAll(false)}
        >
          <p>{t("Operation: clear the fault codes · class SERVICE_ROUTINE")}</p>
          <ul className="clear-outcome-codes">
            {withCodes.map((outcome) => {
              const name = nameOf(outcome.ecuFamily);
              return (
                <li key={outcome.ecuFamily}>
                  <strong>{outcome.ecuFamily}</strong>
                  {name !== null ? ` — ${name}` : ""}
                  {" · "}
                  {t("{count} code(s)", { count: outcome.dtcs.length })}
                </li>
              );
            })}
          </ul>
          <p>
            {t(
              "{codes} code(s) will be erased from {modules} module(s), one module after another; each answer is recorded. They stay in this session's report.",
              { codes, modules: withCodes.length },
            )}
          </p>
          {batteryNotice !== null ? <p className="battery-precondition">{batteryNotice}</p> : null}
          <p>{t("The car stands still, the ignition is on, the engine is off.")}</p>
          <p>{t("After a positive answer each module's codes are read again at once.")}</p>
        </ConfirmDialog>
      ) : null}

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
