import { useState } from "react";
import { useHelpLanguage } from "../helpLanguage";
import { codeText, dataText, helpLines, parameterNote, sddText, t, useLanguage } from "../i18n";
import { ConfirmDialog } from "./ConfirmDialog";
import { DtcHelp } from "./DtcHelp";
import { HelpLanguageSwitch } from "./HelpLanguageSwitch";
import type { ModuleSurveyEntry } from "../library";
import { isFaultCodesRead, type ModuleReadKind, type ModuleReadSnapshot } from "../moduleRead";
import type { DtcClearSnapshot } from "../serviceMode";
import { moduleReasons, moduleRoute, nodeStatus, routeText } from "../networkMap";
import { parameterName } from "../parameterNames";
import { withUnit } from "../units";
import { StatusBadge } from "./StatusBadge";

interface ModuleDetailsProps {
  module: ModuleSurveyEntry | null;
  outcome: ModuleReadSnapshot | undefined;
  kind: ModuleReadKind;
  identifier: string;
  adapterReady: boolean;
  busy: boolean;
  saving: boolean;
  onKindChange: (kind: ModuleReadKind) => void;
  onIdentifierChange: (identifier: string) => void;
  onRead: () => void;
  onSaveReport: () => void;
  /** On the bench (ADR-0020) nothing is saved. */
  bench?: boolean;
  /** The service mode (ADR-0036): while it is on, the clear is offered
   *  under a module's fault codes once they have been read. */
  serviceMode?: boolean;
  /** The last clear, shown under the module it was made on. */
  clear?: DtcClearSnapshot;
  clearing?: boolean;
  onClearCodes?: (ecuFamily: string) => void;
  /**
   * The session's low-battery line (ADR-0030, 2026-09-19), repeated in the
   * confirmation of every operation that changes what a module holds: a
   * clear on a sagging rail can leave a module halfway.
   */
  batteryNotice?: string | null;
}

/**
 * What a clear came to (ADR-0036): the module's answer, the session it was
 * given in, and what the module said when read again - a code that returned
 * at once is a fault that is present, not one that was missed.
 */
function ClearOutcome({ clear }: { clear: DtcClearSnapshot }) {
  return (
    <div className={`clear-outcome clear-outcome--${clear.state.toLowerCase()}`} role="status">
      {clear.state === "CLEARED" ? (
        <p>
          <strong>{t("The module accepted the clear.")}</strong>
          {clear.session !== null ? ` ${t("in session {session}", { session: clear.session })}` : ""}
          {clear.routeValidation === "SYNTHETIC" ? ` · ${t("bench, synthetic")}` : ""}
        </p>
      ) : null}
      {clear.state === "REFUSED" ? (
        <p>
          <strong>{t("The module refused the clear: {refusal}", { refusal: clear.refusal ?? "—" })}</strong>
        </p>
      ) : null}
      {clear.state === "FAILED" ? (
        <p>
          <strong>{t("The clear was not made.")}</strong>
          {clear.error !== null ? ` ${t(clear.error.message)}` : ""}
          {clear.error?.technicalDetails ? (
            <span className="module-validation"> {clear.error.technicalDetails}</span>
          ) : null}
        </p>
      ) : null}
      {clear.codesAfter !== null ? (
        clear.codesAfter.length === 0 ? (
          <p>{t("Read again: no codes reported.")}</p>
        ) : (
          <>
            <p>{t("Read again: {count} code(s) returned at once.", { count: clear.codesAfter.length })}</p>
            <ul className="clear-outcome-codes">
              {clear.codesAfter.map((dtc) => (
                <li key={`${dtc.code}-${dtc.failureType}`}>
                  <strong>{dtc.code}</strong>
                  {dtc.description !== null ? ` — ${dtc.description}` : ""}
                </li>
              ))}
            </ul>
          </>
        )
      ) : null}
      <p className="module-validation">
        {t("{count} code(s) were held before the clear; they stay in this session's report.", {
          count: clear.codesBefore.length,
        })}
      </p>
    </div>
  );
}


function tone(state: string): "positive" | "pending" | "negative" | "neutral" {
  switch (state) {
    case "reachable":
    case "answered":
      return "positive";
    case "hypothesis":
    case "reading":
      return "pending";
    case "silent":
    case "declined":
    case "failed":
      return "negative";
    default:
      return "neutral";
  }
}

export function ModuleDetails({
  module,
  outcome,
  kind,
  identifier,
  adapterReady,
  busy,
  saving,
  onKindChange,
  onIdentifierChange,
  onRead,
  onSaveReport,
  bench = false,
  serviceMode = false,
  clear,
  clearing = false,
  onClearCodes,
  batteryNotice = null,
}: ModuleDetailsProps) {
  const language = useLanguage();
  // The one question before a clear (ADR-0036, decision 3).
  const [confirmClear, setConfirmClear] = useState(false);
  const [helpLanguage] = useHelpLanguage(language);
  if (module === null) {
    return (
      <section className="module-details" aria-labelledby="module-details-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("Module")}</p>
            <h2 id="module-details-title">{t("Choose a module")}</h2>
          </div>
        </div>
        <p className="operation-copy">
          {t(
            "Pick a module on the network map to see how it is reached, what it can be asked, and to read its fault codes or one identifier. Every request is read-only.",
          )}
        </p>
        <button className="button button--primary" type="button" disabled>
          {t("Read")}
        </button>
      </section>
    );
  }

  const route = moduleRoute(module);
  const status = nodeStatus(module, outcome, busy);
  const readable = route !== "none";
  // A module on a K-line is read over its own protocol (ADR-0029): the
  // operations are that protocol's, and there is no identifier to choose.
  const serial = module.protocol === "DS2" || module.protocol === "KW2000";
  const canRead =
    adapterReady && !busy && readable && (serial || kind === "FAULT_CODES" || identifier !== "");
  const reasons = moduleReasons(module);

  return (
    <section className="module-details" aria-labelledby="module-details-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Module")}</p>
          <h2 id="module-details-title">{module.ecuFamily}</h2>
          {dataText(module.names, module.name, language) !== null ? (
            <p className="module-fullname">{dataText(module.names, module.name, language)}</p>
          ) : null}
        </div>
        <StatusBadge tone={tone(status.state)}>{status.label}</StatusBadge>
      </div>
      <p className="operation-copy">{status.detail}</p>

      <dl className="detail-list detail-list--compact">
        <div>
          <dt>{t("Bus")}</dt>
          <dd>{module.logicalNetwork ?? t("not recorded")}</dd>
        </div>
        <div>
          <dt>{t("Adapter route")}</dt>
          <dd>
            {routeText(module)}
            {module.routeValidation !== "" ? (
              <div className="module-validation">{module.routeValidation}</div>
            ) : null}
          </dd>
        </div>
        {module.requestId !== null && module.responseId !== null ? (
          <div>
            <dt>{t("Addresses")}</dt>
            <dd>
              <code>
                {module.requestId} → {module.responseId}
              </code>
            </dd>
          </div>
        ) : null}
        <div>
          <dt>{t("Protocol")}</dt>
          <dd>{module.protocol ?? t("not recorded")}</dd>
        </div>
        <div>
          <dt>{t("Readable identifiers")}</dt>
          <dd>{module.readableIdentifiers.length}</dd>
        </div>
      </dl>
      {reasons.length > 0 ? (
        <ul className="module-reasons">
          {reasons.map((reason) => (
            <li key={reason}>{reason}</li>
          ))}
        </ul>
      ) : null}

      {readable ? (
        <>
          <div className="field-stack">
            <label className="field">
              <span>{t("Operation")}</span>
              <select
                name="readOperation"
                value={kind}
                onChange={(event) => onKindChange(event.target.value as ModuleReadKind)}
              >
                {serial ? (
                  <>
                    <option value="FAULT_CODES">{t("Fault memory")}</option>
                    {module.protocol === "DS2" ? (
                      <option value="IDENTIFIER">{t("What the module says it is")}</option>
                    ) : null}
                  </>
                ) : (
                  <>
                    <option value="FAULT_CODES">{t("Confirmed fault codes")}</option>
                    {/*
                      "Identifier" named the address and not the thing at it,
                      so the closed list said nothing about what the operation
                      would bring back (the owner, 2026-09-18: "хочеться
                      зрозуміти що ми ідентифікуємо"). The option now says
                      what you get; the number is in the list under it.
                    */}
                    <option value="IDENTIFIER">{t("One value the module holds")}</option>
                  </>
                )}
              </select>
            </label>
            {kind === "IDENTIFIER" && !serial ? (
              <label className="field">
                <span>{t("Which value")}</span>
                <select
                  name="readIdentifier"
                  value={identifier}
                  onChange={(event) => onIdentifierChange(event.target.value)}
                >
                  <option value="">{t("Choose a value")}</option>
                  {module.readableIdentifiers.map((entry) => (
                    <option key={entry.identifier} value={entry.identifier}>
                      {entry.identifier} — {entry.parameters.map(parameterName).join(", ")}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}
          </div>
          <div className="details-actions">
            <button
              className="button button--primary"
              type="button"
              onClick={onRead}
              disabled={!canRead}
            >
              {busy ? t("Reading…") : t("Read")}
            </button>
            <button
              className="button button--secondary"
              type="button"
              onClick={onSaveReport}
              disabled={bench || outcome === undefined || !outcome.reportAvailable || saving || busy}
            >
              {saving ? t("Saving…") : t("Save read report")}
            </button>
          </div>
          {bench ? (
            <p className="button-hint" role="note">
              {t("Bench: nothing is saved.")}
            </p>
          ) : null}
          {route === "hypothesis" ? (
            <p className="button-hint" role="note">
              {t(
                "This module's route is an unverified hypothesis. The request is read-only; an answer confirms the route, silence refutes it. Either way, save the report.",
              )}
            </p>
          ) : null}
          {!adapterReady ? (
            <p className="button-hint">{t("Connect and verify MongoosePro JLR to enable reads.")}</p>
          ) : null}
        </>
      ) : (
        <button className="button button--primary" type="button" disabled>
          {t("Read")}
        </button>
      )}

      {outcome !== undefined && outcome.error !== null ? (
        <div className="error-banner diagnostic-error" role="alert">
          <h3>{outcome.error.message}</h3>
          {outcome.error.technicalDetails !== null ? (
            <details>
              <summary>{t("Technical details")}</summary>
              <code>{outcome.error.technicalDetails}</code>
            </details>
          ) : null}
        </div>
      ) : null}

      {outcome !== undefined && outcome.state === "SUCCEEDED" ? (
        <>
          {outcome.parameters.length > 0 ? (
            <table className="module-table">
              <thead>
                <tr>
                  <th scope="col">{t("Parameter")}</th>
                  <th scope="col">{t("Value")}</th>
                  <th scope="col">{t("Note")}</th>
                </tr>
              </thead>
              <tbody>
                {outcome.parameters.map((parameter) => (
                  <tr key={parameter.name}>
                    <td>{parameterName(parameter.name)}</td>
                    <td>
                      {parameter.state !== null ? (
                        <>
                          <strong>{parameter.state}</strong>
                          <div className="module-validation">
                            {parameter.value === null ? "—" : withUnit(parameter.value, parameter.unit)}
                          </div>
                        </>
                      ) : (
                        <strong>
                          {parameter.value === null ? "—" : withUnit(parameter.value, parameter.unit)}
                        </strong>
                      )}
                    </td>
                    <td>{parameter.note !== null ? parameterNote(parameter.note) : ""}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
          {outcome.dtcs.length > 0 ? (
            <table className="module-table">
              <thead>
                <tr>
                  <th scope="col">{t("Fault code")}</th>
                  <th scope="col">{t("Description")}</th>
                  <th scope="col">{t("Failure type")}</th>
                  <th scope="col">{t("Status")}</th>
                </tr>
              </thead>
              <tbody>
                {outcome.dtcs.map((dtc) => {
                  const wording = codeText(
                    dtc.descriptionTexts,
                    dtc.description,
                    language,
                    dtc.descriptionDataTexts,
                    helpLanguage,
                  );
                  return (
                  <tr key={`${dtc.code}-${dtc.failureType}`}>
                    <td>
                      <strong>{dtc.code}</strong>
                    </td>
                    <td>
                      {wording.shown ?? t("No wording in the loaded data")}
                      {wording.original !== null ? (
                        <div className="module-validation">{wording.original}</div>
                      ) : null}
                      {dtc.descriptionScope === "generic" ? (
                        <div className="module-validation">{t("generic wording")}</div>
                      ) : null}
                      <DtcHelp dtc={dtc} />
                    </td>
                    <td>
                      <code>{dtc.failureType}</code>
                      {sddText(dtc.failureTypeTexts, dtc.failureTypeText, helpLanguage) !== null ? (
                        <div className="module-validation">
                          {sddText(dtc.failureTypeTexts, dtc.failureTypeText, helpLanguage)}
                        </div>
                      ) : null}
                    </td>
                    <td>
                      <code>{dtc.status}</code>
                    </td>
                  </tr>
                  );
                })}
              </tbody>
            </table>
          ) : outcome.negativeResponse === null && outcome.dataHex === null ? (
            <p className="survey-summary">{t("No confirmed fault codes reported.")}</p>
          ) : null}
          {serviceMode && onClearCodes !== undefined && isFaultCodesRead(outcome) ? (
            <div className="mode-operation">
              <button
                className="button button--service"
                type="button"
                disabled={clearing || busy || !adapterReady}
                onClick={() => setConfirmClear(true)}
              >
                {clearing ? t("Clearing…") : t("Clear the fault codes")}
              </button>
              <span className="button-hint">
                {t("SERVICE_ROUTINE · the codes above stay in this session's report; the module is read again afterwards.")}
              </span>
            </div>
          ) : null}
          {clear !== undefined && clear.state !== "IDLE" && clear.ecuFamily === module.ecuFamily ? (
            <ClearOutcome clear={clear} />
          ) : null}
          {confirmClear ? (
            <ConfirmDialog
              title={t("Clear the fault codes of {module}", { module: module.ecuFamily })}
              confirmLabel={t("Clear the codes of {module}", { module: module.ecuFamily })}
              cancelLabel={t("Cancel")}
              onConfirm={() => {
                setConfirmClear(false);
                onClearCodes?.(module.ecuFamily);
              }}
              onCancel={() => setConfirmClear(false)}
            >
              <p>
                <strong>{module.ecuFamily}</strong>
                {dataText(module.names, module.name, language) !== null
                  ? ` — ${dataText(module.names, module.name, language)}`
                  : ""}
              </p>
              <p>{t("Operation: clear the fault codes · class SERVICE_ROUTINE")}</p>
              <p>
                {t("{count} code(s) will be erased from the module. They stay in this session's report.", {
                  count: outcome.dtcs.length,
                })}
              </p>
              {batteryNotice !== null ? (
                <p className="battery-precondition">{batteryNotice}</p>
              ) : null}
              <p>{t("The car stands still, the ignition is on, the engine is off.")}</p>
              <p>{t("After a positive answer the codes are read again at once.")}</p>
            </ConfirmDialog>
          ) : null}
          <details className="technical-details">
            <summary>{t("Exchange as it happened")}</summary>
            <dl className="detail-list detail-list--compact">
              <div>
                <dt>{t("Module")}</dt>
                <dd>
                  {outcome.ecuFamily} on {outcome.routeId}
                  {outcome.routeValidation !== "" ? ` (${outcome.routeValidation})` : ""}
                </dd>
              </div>
              <div>
                <dt>{t("Request")}</dt>
                <dd>
                  <code>{outcome.requestHex}</code> — {outcome.operation}
                </dd>
              </div>
              <div>
                <dt>{t("Answer")}</dt>
                <dd>
                  {outcome.responder ?? "—"}: <code>{outcome.rawResponseHex ?? "—"}</code>
                  {outcome.pendingResponses > 0
                    ? ` ${t("after {count} response-pending", { count: outcome.pendingResponses })}`
                    : ""}
                </dd>
              </div>
              {outcome.negativeResponse !== null ? (
                <div>
                  <dt>{t("Declined")}</dt>
                  <dd>{outcome.negativeResponse}</dd>
                </div>
              ) : null}
              {outcome.dataHex !== null ? (
                <div>
                  <dt>{t("Data")}</dt>
                  <dd>
                    <code>{outcome.dataHex}</code>
                  </dd>
                </div>
              ) : null}
            </dl>
          </details>
        </>
      ) : null}
      {/* Reference, not an action: what the module declares it can be
          asked to do stands after what was actually read, so the read
          button keeps its place beside the identifier that feeds it. */}
          {module.selfTests.length > 0 ? (
            <details className="self-tests">
              <summary>{t("Self tests this module declares")}</summary>
              <p className="module-validation">
                {t(
                  "SDD can command these; this application does not. A self test is a routine, not a read, and routines belong to a later stage with their own safety rules. The list is here because knowing what a module can be asked to do is worth having.",
                )}
              </p>
              <p className="module-validation">
                {t(
                  "The year markers are SDD's own and this session does not check them, so a test listed here can belong to another year of the same programme.",
                )}
              </p>
              <HelpLanguageSwitch />
              <ul className="self-test-list">
                {module.selfTests.map((test) => (
                  <li key={test.testId}>
                    <div className="self-test-head">
                      <strong>{test.name}</strong>
                      <span className="module-validation">
                        {t("test {id}", { id: test.testId })}
                        {test.modelYears.length === 0
                          ? ""
                          : ` · ${test.modelYears.join(", ")}`}
                        {test.timeMs === null
                          ? ""
                          : ` · ${t("{seconds} s", { seconds: Math.round(test.timeMs / 1000) })}`}
                        {` · ${test.safetyClass}`}
                      </span>
                    </div>
                    {helpLines(test.descriptionTexts, test.description, helpLanguage).map(
                      (line, index) => (
                        <p className="self-test-line" key={`${test.testId}-${index}`}>
                          {line}
                        </p>
                      ),
                    )}
                  </li>
                ))}
              </ul>
            </details>
          ) : null}
          {(module.acceptedOperations ?? []).length > 0 ? (
            <details className="accepted-operations">
              <summary>{t("What this module will accept")}</summary>
              <p className="module-validation">
                {t(
                  "SDD declares these; this application sends none of them. A value written into a module, an output driven by hand and a routine the module runs are three different kinds of change, and each belongs to a later stage with its own safety rules. The list is here because the cost of an operation is worth knowing long before anyone decides whether to allow it.",
                )}
              </p>
              {/*
                The names are SDD's, from its module files, and SDD writes
                them in English only. ADR-0034 closed translating SDD's text
                by anyone, so the block says why these stay English rather
                than leaving a Ukrainian or Russian reader to wonder why the
                language switch above did not reach them (the owner,
                2026-09-18: "не перекладає весь текст під рожевим").
              */}
              <p className="module-validation">
                {t("The names are SDD's own, and SDD writes them in English only; the data carries no other language for them.")}
              </p>
              <ul className="accepted-operation-list">
                {(module.acceptedOperations ?? []).map((operation) => (
                  <li key={`${operation.kind}-${operation.identifier}`}>
                    <div className="accepted-operation-head">
                      <strong>{operation.name ?? operation.identifier}</strong>
                      <span className="module-validation">
                        {[
                          operation.identifier,
                          operation.service === null
                            ? null
                            : t("service {service}", { service: operation.service }),
                          operation.sessions.length === 0
                            ? null
                            : t("session {sessions}", {
                                sessions: operation.sessions.join(", "),
                              }),
                          operation.security === null
                            ? null
                            : t("security {level}", { level: operation.security }),
                          operation.maxRunTime === null
                            ? null
                            : t("up to {seconds} s", { seconds: operation.maxRunTime }),
                          operation.safetyClass,
                        ]
                          .filter((part) => part !== null)
                          .join(" · ")}
                      </span>
                    </div>
                  </li>
                ))}
              </ul>
            </details>
          ) : null}
    </section>
  );
}
