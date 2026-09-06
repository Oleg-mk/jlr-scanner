import { t, useLanguage } from "../i18n";
import { captureRoutes, type CaptureSnapshot } from "../capture";
import { StatusBadge } from "./StatusBadge";

interface CapturePanelProps {
  snapshot: CaptureSnapshot;
  routeId: string;
  seconds: number;
  adapterReady: boolean;
  busy: boolean;
  saving: boolean;
  onRouteChange: (routeId: string) => void;
  onSecondsChange: (seconds: number) => void;
  onListen: () => void;
  onSave: () => void;
}

/// The verdict in the interface language, composed from the snapshot's
/// counts; the shell's English `verdict` string goes into the report file.
function verdictText(snapshot: CaptureSnapshot): string {
  const rate =
    snapshot.bitrateBps === null
      ? t("the route's rate")
      : `${Math.round(snapshot.bitrateBps / 1000)} kbit/s`;
  const values = {
    route: snapshot.routeId,
    pins: snapshot.pins,
    rate,
    seconds: (snapshot.listenedMs / 1000).toFixed(1),
    perSecond: snapshot.framesPerSecond,
    distinct: snapshot.distinctIds,
  };
  const parts = [
    snapshot.frames > 0
      ? t(
          "Traffic present on {route} (pins {pins}) at {rate}: about {perSecond} frames/s, {distinct} distinct identifiers. A live bus is on this pair. Which of the vehicle's buses it is cannot be told from listening alone.",
          values,
        )
      : t(
          "No frames heard on {route} (pins {pins}) at {rate} in {seconds} s. Either this pair is silent at that rate — a diagnostic-only CAN behind a gateway carries nothing until a tester speaks — or the rate does not match, or nothing is connected. Listening alone cannot tell these apart.",
          values,
        ),
  ];
  if (snapshot.truncated) {
    parts.push(
      t("The frame cap was reached before the time was up; the counts describe the captured part only."),
    );
  }
  if (snapshot.droppedFrames > 0) {
    parts.push(
      t("{count} non-data packets from the adapter were dropped.", { count: snapshot.droppedFrames }),
    );
  }
  return parts.join(" ");
}

function badge(snapshot: CaptureSnapshot, busy: boolean) {
  if (busy) return <StatusBadge tone="pending">{t("Listening…")}</StatusBadge>;
  switch (snapshot.state) {
    case "COMPLETED":
      return snapshot.frames > 0 ? (
        <StatusBadge tone="positive">{t("Traffic heard")}</StatusBadge>
      ) : (
        <StatusBadge tone="neutral">{t("Silent")}</StatusBadge>
      );
    case "FAILED":
      return <StatusBadge tone="negative">{t("Failed")}</StatusBadge>;
    default:
      return <StatusBadge tone="neutral">{t("Not run")}</StatusBadge>;
  }
}

export function CapturePanel({
  snapshot,
  routeId,
  seconds,
  adapterReady,
  busy,
  saving,
  onRouteChange,
  onSecondsChange,
  onListen,
  onSave,
}: CapturePanelProps) {
  useLanguage();
  return (
    <section className="capture-panel" aria-labelledby="capture-title">
      <div className="section-heading section-heading--action">
        <div>
          <p className="eyebrow">{t("Listen only")}</p>
          <h2 id="capture-title">{t("Bus capture")}</h2>
        </div>
        {badge(snapshot, busy)}
      </div>

      <p className="operation-copy">
        {t(
          "Opens one CAN pair of the diagnostic connector in listen-only mode and records what the vehicle broadcasts. Nothing is transmitted; the adapter stays silent on the bus, so the vehicle cannot notice. The result says whether a live bus is on that pair; it cannot say which of the vehicle's buses it is.",
        )}
      </p>

      <div className="field-row">
        <label className="field">
          <span>{t("Pair")}</span>
          <select
            name="captureRoute"
            value={routeId}
            onChange={(event) => onRouteChange(event.target.value)}
          >
            {captureRoutes.map((route) => (
              <option key={route.id} value={route.id}>
                {route.label}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>{t("Seconds (1–15)")}</span>
          <input
            type="number"
            name="captureSeconds"
            min={1}
            max={15}
            value={seconds}
            onChange={(event) =>
              onSecondsChange(Math.min(15, Math.max(1, Number(event.target.value) || 1)))
            }
          />
        </label>
        <button
          className="button button--primary"
          type="button"
          onClick={onListen}
          disabled={!adapterReady || busy}
        >
          {busy ? t("Listening…") : t("Listen")}
        </button>
        <button
          className="button button--secondary"
          type="button"
          onClick={onSave}
          disabled={!snapshot.captureAvailable || saving || busy}
        >
          {saving ? t("Saving…") : t("Save capture")}
        </button>
      </div>
      {!adapterReady ? (
        <p className="button-hint">{t("Connect and verify MongoosePro JLR to enable listening.")}</p>
      ) : null}

      {snapshot.error !== null ? (
        <div className="error-banner diagnostic-error" role="alert">
          <h3>{t("Capture did not complete")}</h3>
          <p>{snapshot.error}</p>
        </div>
      ) : null}

      {snapshot.state === "COMPLETED" ? (
        <>
          <p className="survey-summary" role="status">
            {verdictText(snapshot)}
          </p>
          <dl className="detail-list detail-list--compact">
            <div>
              <dt>{t("Frames")}</dt>
              <dd>
                {t("{frames} in {seconds} s ({perSecond}/s)", {
                  frames: snapshot.frames,
                  seconds: (snapshot.listenedMs / 1000).toFixed(1),
                  perSecond: snapshot.framesPerSecond,
                })}
              </dd>
            </div>
            <div>
              <dt>{t("Identifiers")}</dt>
              <dd>
                {t("{distinct} distinct — {standard} standard, {extended} extended frames", {
                  distinct: snapshot.distinctIds,
                  standard: snapshot.standardFrames,
                  extended: snapshot.extendedFrames,
                })}
              </dd>
            </div>
            {snapshot.droppedFrames > 0 || snapshot.truncated ? (
              <div>
                <dt>{t("Notes")}</dt>
                <dd>
                  {snapshot.truncated ? `${t("Frame cap reached.")} ` : ""}
                  {snapshot.droppedFrames > 0
                    ? t("{count} non-data packets dropped.", { count: snapshot.droppedFrames })
                    : ""}
                </dd>
              </div>
            ) : null}
          </dl>
          {snapshot.topIds.length > 0 ? (
            <table className="module-table">
              <thead>
                <tr>
                  <th scope="col">{t("Identifier")}</th>
                  <th scope="col">{t("Width")}</th>
                  <th scope="col">{t("Frames")}</th>
                </tr>
              </thead>
              <tbody>
                {snapshot.topIds.map((entry) => (
                  <tr key={entry.id}>
                    <td>
                      <code>{entry.id}</code>
                    </td>
                    <td>{entry.extended ? "29-bit" : "11-bit"}</td>
                    <td>{entry.count}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          ) : null}
        </>
      ) : null}
    </section>
  );
}
