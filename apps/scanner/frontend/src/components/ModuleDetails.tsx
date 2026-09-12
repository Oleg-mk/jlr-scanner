import { codeText, dataText, parameterNote, t, useLanguage } from "../i18n";
import { DtcHelp } from "./DtcHelp";
import type { ModuleSurveyEntry } from "../library";
import type { ModuleReadKind, ModuleReadSnapshot } from "../moduleRead";
import { moduleReasons, moduleRoute, nodeStatus, routeText } from "../networkMap";
import { parameterName } from "../parameterNames";
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
}: ModuleDetailsProps) {
  const language = useLanguage();
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
  const canRead = adapterReady && !busy && readable && (kind === "FAULT_CODES" || identifier !== "");
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
                <option value="FAULT_CODES">{t("Confirmed fault codes")}</option>
                <option value="IDENTIFIER">{t("Identifier")}</option>
              </select>
            </label>
            {kind === "IDENTIFIER" ? (
              <label className="field">
                <span>{t("Identifier")}</span>
                <select
                  name="readIdentifier"
                  value={identifier}
                  onChange={(event) => onIdentifierChange(event.target.value)}
                >
                  <option value="">{t("Choose an identifier")}</option>
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
                            {parameter.value ?? "—"}
                            {parameter.unit !== null ? ` ${parameter.unit}` : ""}
                          </div>
                        </>
                      ) : (
                        <strong>
                          {parameter.value ?? "—"}
                          {parameter.unit !== null && parameter.value !== null
                            ? ` ${parameter.unit}`
                            : ""}
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
                  const wording = codeText(dtc.descriptionTexts, dtc.description, language);
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
                      {dataText(dtc.failureTypeTexts, dtc.failureTypeText, language) !== null ? (
                        <div className="module-validation">
                          {dataText(dtc.failureTypeTexts, dtc.failureTypeText, language)}
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
    </section>
  );
}
