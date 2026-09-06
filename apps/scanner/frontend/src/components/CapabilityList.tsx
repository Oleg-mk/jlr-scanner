import type {
  AdapterInfo,
  VehicleInterfaceCapability,
} from "../adapter";
import { StatusBadge } from "./StatusBadge";

interface CapabilityListProps {
  adapter: AdapterInfo;
  capabilities: VehicleInterfaceCapability[];
}

function bitrateLabel(value: number | null) {
  return value === null ? "N/A" : `${value / 1_000} kbit/s`;
}

export function CapabilityList({
  adapter,
  capabilities,
}: CapabilityListProps) {
  return (
    <section className="capabilities" aria-labelledby="capabilities-title">
      <div className="section-heading">
        <h2 id="capabilities-title">Supported vehicle interfaces</h2>
      </div>
      <div className="capability-table" role="table">
        <div className="capability-row capability-row--header" role="row">
          <span role="columnheader">Interface</span>
          <span role="columnheader">Pins / connector</span>
          <span role="columnheader">Nominal bitrate</span>
          <span role="columnheader">Implementation</span>
          <span role="columnheader">Vehicle validation</span>
        </div>
        {capabilities.map((capability) => {
          const unsupported =
            capability.implementation === "UNSUPPORTED_BY_ADAPTER";
          return (
            <div className="capability-row" role="row" key={capability.id}>
              <strong role="cell">{capability.name}</strong>
              <span role="cell" data-label="Pins / connector">{capability.pins}</span>
              <span role="cell" data-label="Nominal bitrate">
                {bitrateLabel(capability.nominalBitrate)}
              </span>
              <div role="cell" data-label="Implementation" className="capability-status">
                <StatusBadge tone={unsupported ? "negative" : "positive"}>
                  {unsupported ? "Unsupported by adapter" : "Implemented"}
                </StatusBadge>
                {!unsupported ? (
                  <span className="capability-evidence">
                    {capability.hardwareConfirmed ? "Hardware confirmed" : null}
                    {capability.hardwareConfirmed && capability.fixtureTested ? ", " : null}
                    {capability.fixtureTested ? "Fixture tested" : null}
                  </span>
                ) : (
                  <span className="capability-evidence">Current adapter: Unsupported</span>
                )}
              </div>
              <div role="cell" data-label="Vehicle validation" className="capability-status">
                {capability.vehicleValidation === "NOT_YET_VALIDATED" ? (
                  <>
                    <StatusBadge tone="pending">Vehicle validation pending</StatusBadge>
                    <span className="capability-evidence">Not yet validated</span>
                  </>
                ) : (
                  <span aria-label="Not applicable">—</span>
                )}
              </div>
            </div>
          );
        })}
      </div>
      <details className="technical-details">
        <summary>Technical details</summary>
        <dl className="technical-details__grid">
          <div><dt>Response command</dt><dd>{adapter.boardInfo.responseCommand}</dd></div>
          <div><dt>Backend</dt><dd>{adapter.backend}</dd></div>
          <div className="technical-details__raw">
            <dt>Raw board-info</dt>
            <dd><code>{adapter.boardInfo.rawResponseHex}</code></dd>
          </div>
        </dl>
      </details>
    </section>
  );
}
