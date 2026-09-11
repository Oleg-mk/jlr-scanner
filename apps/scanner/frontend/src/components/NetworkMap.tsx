import { dataText, t, useLanguage } from "../i18n";
import type { ModuleSurveyEntry, VehicleSurveySnapshot } from "../library";
import type { ModuleReadSnapshot } from "../moduleRead";
import { lanes, nodeStatus, type NodeState } from "../networkMap";

interface NetworkMapProps {
  survey: VehicleSurveySnapshot | null;
  results: Record<string, ModuleReadSnapshot>;
  readingModule: string | null;
  selected: string | null;
  adapterReady: boolean;
  checking: { done: number; total: number } | null;
  checkable: number;
  includeHypothesis: boolean;
  onSelect: (ecuFamily: string) => void;
  onCheckAll: () => void;
  onCancelCheck: () => void;
  onIncludeHypothesisChange: (include: boolean) => void;
  /** The mileage survey (ADR-0024) is a whole-car action like the check. */
  mileageRunning: boolean;
  mileageBusy: boolean;
  onReadMileage: () => void;
  onStopMileage: () => void;
}

function NodeIcon({ state }: { state: NodeState }) {
  switch (state) {
    case "reachable":
    case "answered":
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="9" />
          <path d="M5.5 10.5l3 3 6-6" fill="none" strokeWidth="2.2" strokeLinecap="round" />
        </svg>
      );
    case "hypothesis":
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="9" strokeDasharray="3 2.5" />
          <text x="10" y="14.5" textAnchor="middle" fontSize="11" fontWeight="700">
            ?
          </text>
        </svg>
      );
    case "reading":
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true" className="node-icon--spin">
          <circle cx="10" cy="10" r="9" strokeDasharray="14 42" />
        </svg>
      );
    case "declined":
    case "failed":
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="9" />
          <path d="M10 5.5v6M10 14.5v.5" fill="none" strokeWidth="2.2" strokeLinecap="round" />
        </svg>
      );
    case "silent":
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="9" />
          <path d="M6.5 6.5l7 7M13.5 6.5l-7 7" fill="none" strokeWidth="2.2" strokeLinecap="round" />
        </svg>
      );
    default:
      return (
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <circle cx="10" cy="10" r="9" />
          <path d="M6 10h8" fill="none" strokeWidth="2.2" strokeLinecap="round" />
        </svg>
      );
  }
}

const legend: Array<{ state: NodeState; text: string }> = [
  { state: "reachable", text: "documented route; can be read" },
  { state: "hypothesis", text: "route is a hypothesis; an answer confirms it" },
  { state: "unreachable", text: "not reachable from the adapter; the reason is shown" },
  { state: "answered", text: "answered a read-only request" },
  { state: "silent", text: "no answer within the timeout" },
  { state: "declined", text: "negative response or failed read" },
];

export function NetworkMap({
  survey,
  results,
  readingModule,
  selected,
  adapterReady,
  checking,
  checkable,
  includeHypothesis,
  onSelect,
  onCheckAll,
  onCancelCheck,
  onIncludeHypothesisChange,
  mileageRunning,
  mileageBusy,
  onReadMileage,
  onStopMileage,
}: NetworkMapProps) {
  const language = useLanguage();
  const modules: ModuleSurveyEntry[] = survey?.modules ?? [];
  const busy = checking !== null;
  return (
    <section className="network-map" aria-labelledby="network-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Every module the data knows")}</p>
          <h2 id="network-title">{t("Vehicle network")}</h2>
        </div>
        <div className="network-actions">
          <label className="check-option">
            <input
              type="checkbox"
              checked={includeHypothesis}
              disabled={busy}
              onChange={(event) => onIncludeHypothesisChange(event.target.checked)}
            />
            <span>{t("Also try unverified routes")}</span>
          </label>
          {busy ? (
            <button className="button button--secondary" type="button" onClick={onCancelCheck}>
              {t("Stop after this module ({done}/{total})", { done: checking.done, total: checking.total })}
            </button>
          ) : (
            <button
              className="button button--primary"
              type="button"
              onClick={onCheckAll}
              disabled={!adapterReady || checkable === 0}
            >
              {t("Check all modules ({count})", { count: checkable })}
            </button>
          )}
          {mileageRunning ? (
            <button className="button button--secondary" type="button" onClick={onStopMileage}>
              {t("Stop")}
            </button>
          ) : (
            <button
              className="button button--secondary"
              type="button"
              onClick={onReadMileage}
              disabled={!adapterReady || survey === null || busy || mileageBusy}
            >
              {t("Read the mileage")}
            </button>
          )}
        </div>
      </div>

      {survey === null ? (
        <p className="operation-copy">
          {t(
            "Describe the vehicle and survey it: every module the loaded data associates with it appears here on its bus, with what the adapter can do about it. Nothing is transmitted by the survey.",
          )}
        </p>
      ) : (
        <>
          <p className="operation-copy" role="status">
            {survey.message}
            {!adapterReady
              ? t(" Connect and verify the adapter to read modules; the check sends one read-only fault-code request per module.")
              : t(" The check sends one read-only fault-code request per module and marks what answered.")}
          </p>
          <div className="lanes">
            {lanes(modules).map((lane) => (
              <div key={lane.id} className={`lane lane--${lane.kind}`}>
                <div className="lane-label">
                  <strong>{lane.name}</strong>
                  <span>{lane.route}</span>
                </div>
                <ul className="lane-nodes" aria-label={t("{lane} modules", { lane: lane.name })}>
                  {lane.modules.map((module) => {
                    const status = nodeStatus(
                      module,
                      results[module.ecuFamily],
                      readingModule === module.ecuFamily,
                    );
                    const isSelected = selected === module.ecuFamily;
                    return (
                      <li key={module.ecuFamily}>
                        <button
                          type="button"
                          className={`node node--${status.state}${isSelected ? " node--selected" : ""}`}
                          aria-pressed={isSelected}
                          aria-label={`${module.ecuFamily}: ${status.label}`}
                          title={status.detail}
                          onClick={() => onSelect(module.ecuFamily)}
                        >
                          <span className="node-icon">
                            <NodeIcon state={status.state} />
                          </span>
                          <span className="node-name">{module.ecuFamily}</span>
                          {dataText(module.names, module.name, language) !== null ? (
                            <span className="node-fullname">
                              {dataText(module.names, module.name, language)}
                            </span>
                          ) : null}
                          <span className="node-state">{status.label}</span>
                        </button>
                      </li>
                    );
                  })}
                </ul>
                {lane.note !== null ? <p className="lane-note">{lane.note}</p> : null}
              </div>
            ))}
          </div>
          <ul className="map-legend" aria-label={t("Legend")}>
            {legend.map((entry) => (
              <li key={entry.state} className={`legend-item node--${entry.state}`}>
                <span className="node-icon">
                  <NodeIcon state={entry.state} />
                </span>
                <span>{t(entry.text)}</span>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
