import { useHelpLanguage } from "../helpLanguage";
import { codeText, t, useLanguage } from "../i18n";
import type { RoutineRunSnapshot } from "../routineRun";

interface RoutineOutcomeProps {
  run: RoutineRunSnapshot;
  onStop?: () => void;
}

/**
 * What a self test came to (ADR-0036, step 2): running, with the seconds
 * it has run of the seconds the data gives it and the one key that stops
 * it; then the module's answer as it wrote it - bytes, and the plain
 * statement that this library does not describe them - or its refusal by
 * name, or the stop, or the time that ran out. Every exchange of the run
 * is behind a spoiler, as a read's is. Once the run has ended the shell
 * reads the module's codes again; the codes that were not there before
 * are what the test found, listed under the answer.
 */
export function RoutineOutcome({ run, onStop }: RoutineOutcomeProps) {
  const language = useLanguage();
  const [helpLanguage] = useHelpLanguage(language);
  const seconds = (ms: number) => Math.round(ms / 1000);
  return (
    <div className={`clear-outcome routine-outcome routine-outcome--${run.state.toLowerCase()}`} role="status">
      {run.state === "RUNNING" ? (
        <div className="mode-operation">
          <strong>
            {t("Running: {elapsed} s of {time} s", {
              elapsed: seconds(run.elapsedMs),
              time: seconds(run.timeMs),
            })}
          </strong>
          {onStop !== undefined ? (
            <button className="button button--quiet" type="button" onClick={onStop}>
              {t("Stop the test")}
            </button>
          ) : null}
        </div>
      ) : null}
      {run.state === "COMPLETED" ? (
        <p>
          <strong>{t("The module answered: {result}", { result: run.resultHex ?? "—" })}</strong>
          {run.session !== null ? ` ${t("in session {session}", { session: run.session })}` : ""}
          {run.routeValidation === "SYNTHETIC" ? ` · ${t("bench, synthetic")}` : ""}
          {` · ${t("{seconds} s", { seconds: seconds(run.elapsedMs) })}`}
        </p>
      ) : null}
      {run.state === "COMPLETED" ? (
        <p className="module-validation">
          {t("The result is shown as the module answers it; this library does not describe it.")}
        </p>
      ) : null}
      {run.state === "REFUSED" ? (
        <p>
          <strong>{t("The module refused the test: {refusal}", { refusal: run.refusal ?? "—" })}</strong>
        </p>
      ) : null}
      {run.state === "STOPPED" ? (
        <p>
          <strong>{t("Stopped by you.")}</strong>
          {` · ${t("{seconds} s", { seconds: seconds(run.elapsedMs) })}`}
        </p>
      ) : null}
      {run.state === "TIMED_OUT" ? (
        <p>
          <strong>{t("The module gave no result in {timeout} s.", { timeout: seconds(run.timeoutMs) })}</strong>
        </p>
      ) : null}
      {run.state === "FAILED" ? (
        <p>
          <strong>{t("The test was not run.")}</strong>
          {run.error !== null ? ` ${t(run.error.message)}` : ""}
          {run.error?.technicalDetails ? (
            <span className="module-validation"> {run.error.technicalDetails}</span>
          ) : null}
        </p>
      ) : null}
      {run.codesAfter !== null && run.codesFound.length > 0 ? (
        <>
          <p>
            <strong>{t("What the test found: {count} code(s)", { count: run.codesFound.length })}</strong>
          </p>
          <ul className="clear-outcome-codes">
            {run.codesFound.map((dtc) => {
              const wording = codeText(
                dtc.descriptionTexts,
                dtc.description,
                language,
                dtc.descriptionDataTexts,
                helpLanguage,
              );
              return (
                <li key={`${dtc.code}-${dtc.failureType}`}>
                  <strong>{dtc.code}</strong> {wording.shown ?? t("No wording in the loaded data")}
                </li>
              );
            })}
          </ul>
        </>
      ) : null}
      {run.codesAfter !== null && run.codesFound.length === 0 ? (
        <p>
          {t("The test logged no new code; the module holds {count} code(s).", {
            count: run.codesAfter.length,
          })}
        </p>
      ) : null}
      {run.exchanges.length > 0 ? (
        <details className="technical-details">
          <summary>{t("Exchange as it happened")}</summary>
          <ul className="clear-outcome-codes">
            {run.exchanges.map(([request, response], index) => (
              <li key={`${index}-${request}`}>
                <code>{request}</code> → <code>{response}</code>
              </li>
            ))}
          </ul>
        </details>
      ) : null}
    </div>
  );
}
