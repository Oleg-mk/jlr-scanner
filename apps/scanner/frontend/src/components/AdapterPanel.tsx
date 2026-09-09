import { t, useLanguage } from "../i18n";
import { BENCH_TRANSPORT, type AdapterSnapshot, type UserFacingError } from "../adapter";
import { StatusBadge } from "./StatusBadge";

interface AdapterPanelProps {
  snapshot: AdapterSnapshot;
  selectedPort: string | null;
  onSelectPort: (port: string) => void;
  onDetect: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
  /** The bench (ADR-0020): a virtual vehicle from the library, no adapter and no car. */
  onConnectBench: () => void;
}

function hexId(value: number) {
  return value.toString(16).toUpperCase().padStart(4, "0");
}

function ErrorBanner({ error, hint }: { error: UserFacingError; hint: string | null }) {
  return (
    <div className="error-banner" role="alert">
      <h3>{t(error.message)}</h3>
      {hint !== null ? <p>{hint}</p> : null}
      {error.technicalDetails !== null ? (
        <details>
          <summary>{t("Technical details")}</summary>
          <code>{error.technicalDetails}</code>
        </details>
      ) : null}
    </div>
  );
}

function BenchOffer({ onConnectBench }: { onConnectBench: () => void }) {
  return (
    <div className="bench-offer">
      <button className="button button--quiet" type="button" onClick={onConnectBench}>
        {t("Connect the bench (virtual vehicle)")}
      </button>
      <p className="button-hint">
        {t(
          "No adapter and no car needed: a virtual vehicle built from the library answers instead. Every value is synthetic.",
        )}
      </p>
    </div>
  );
}

export function AdapterPanel({
  snapshot,
  selectedPort,
  onSelectPort,
  onDetect,
  onConnect,
  onDisconnect,
  onConnectBench,
}: AdapterPanelProps) {
  useLanguage();
  const detected =
    snapshot.adapters.find((adapter) => adapter.port === selectedPort) ??
    snapshot.adapters[0] ??
    null;

  if (snapshot.state === "NO_ADAPTER") {
    return (
      <section className="adapter-panel" aria-labelledby="adapter-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("Connection")}</p>
            <h2 id="adapter-title">{t("Adapter")}</h2>
          </div>
        </div>
        {snapshot.error !== null ? <ErrorBanner error={snapshot.error} hint={null} /> : null}
        <div className="adapter-empty">
          <h3>{t("Adapter not detected")}</h3>
          <p>{t("Connect MongoosePro JLR by USB.")}</p>
          <button className="button button--primary" type="button" onClick={onDetect}>
            {t("Detect adapter")}
          </button>
        </div>
        <BenchOffer onConnectBench={onConnectBench} />
      </section>
    );
  }

  if (snapshot.state === "CONNECTED" && snapshot.adapter !== null) {
    const adapter = snapshot.adapter;
    const bench = adapter.transport === BENCH_TRANSPORT;
    return (
      <section className="adapter-panel" aria-labelledby="adapter-title">
        <div className="section-heading section-heading--action">
          <div>
            <p className="eyebrow">{t("Connection")}</p>
            <h2 id="adapter-title">{t("Adapter")}</h2>
          </div>
          <button className="button button--secondary" type="button" onClick={onDisconnect}>
            {t("Disconnect")}
          </button>
        </div>
        {snapshot.error !== null ? <ErrorBanner error={snapshot.error} hint={null} /> : null}
        <div className="adapter-title-row">
          <h3>{bench ? t("Virtual vehicle (bench)") : adapter.name}</h3>
          {bench ? (
            <StatusBadge tone="pending">{t("Bench")}</StatusBadge>
          ) : (
            <StatusBadge tone="positive">{t("Connected")}</StatusBadge>
          )}
        </div>
        {bench ? (
          <>
            <p className="operation-copy">
              {t(
                "Every answer comes from the library, not from a car. Nothing from this session is written to disk.",
              )}
            </p>
            <dl className="detail-list">
              <div><dt>{t("Transport")}</dt><dd>{adapter.transport}</dd></div>
              <div><dt>{t("Driver / backend")}</dt><dd>{adapter.backend}</dd></div>
              <div><dt>{t("Board-info response")}</dt><dd>{adapter.boardInfo.responseCommand}</dd></div>
            </dl>
          </>
        ) : (
          <dl className="detail-list">
            <div><dt>USB VID</dt><dd>{hexId(adapter.usbVid)}</dd></div>
            <div><dt>USB PID</dt><dd>{hexId(adapter.usbPid)}</dd></div>
            <div><dt>{t("Serial")}</dt><dd>{adapter.serialNumber ?? t("Not provided by OS")}</dd></div>
            <div><dt>{t("Port")}</dt><dd>{adapter.port}</dd></div>
            <div><dt>{t("Transport")}</dt><dd>{adapter.transport}</dd></div>
            <div>
              <dt>{t("Driver / backend")}</dt>
              <dd>{adapter.driver ?? t("Driver not reported")} / {adapter.backend}</dd>
            </div>
            <div>
              <dt>{t("Board communication")}</dt>
              <dd><StatusBadge tone="positive">{t("Verified")}</StatusBadge></dd>
            </div>
            <div><dt>{t("Board-info response")}</dt><dd>{adapter.boardInfo.responseCommand}</dd></div>
          </dl>
        )}
      </section>
    );
  }

  const connecting = snapshot.state === "CONNECTING";
  const error = snapshot.error;

  return (
    <section className="adapter-panel" aria-labelledby="adapter-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("Connection")}</p>
          <h2 id="adapter-title">{t("Adapter")}</h2>
        </div>
      </div>
      {error !== null ? (
        <ErrorBanner
          error={error}
          hint={
            error.code === "SESSION_MODE_MISMATCH"
              ? null
              : t("Check the USB connection and try again.")
          }
        />
      ) : null}
      {detected !== null ? (
        <>
          <div className="adapter-title-row">
            <h3>{t("{name} detected", { name: detected.name })}</h3>
            <StatusBadge tone={connecting ? "pending" : "neutral"}>
              {connecting ? t("Connecting") : detected.port}
            </StatusBadge>
          </div>
          {snapshot.selectionRequired ? (
            <label className="adapter-select">
              <span>{t("Choose adapter")}</span>
              <select
                value={selectedPort ?? ""}
                onChange={(event) => onSelectPort(event.target.value)}
              >
                <option value="" disabled>{t("Select a COM port")}</option>
                {snapshot.adapters.map((adapter) => (
                  <option value={adapter.port} key={adapter.port}>
                    {adapter.name} — {adapter.port}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
          <dl className="detail-list detail-list--compact">
            <div><dt>USB VID / PID</dt><dd>{hexId(detected.usbVid)}:{hexId(detected.usbPid)}</dd></div>
            <div><dt>{t("Serial")}</dt><dd>{detected.serialNumber ?? t("Not provided by OS")}</dd></div>
            <div><dt>{t("Port")}</dt><dd>{detected.port}</dd></div>
          </dl>
          <div className="adapter-actions">
            <button
              className="button button--primary"
              type="button"
              onClick={onConnect}
              disabled={connecting || (snapshot.selectionRequired && selectedPort === null)}
            >
              {connecting ? t("Connecting…") : t("Connect")}
            </button>
            <button className="button button--quiet" type="button" onClick={onDetect}>
              {t("Detect again")}
            </button>
          </div>
        </>
      ) : (
        <button className="button button--primary" type="button" onClick={onDetect}>
          {t("Detect adapter")}
        </button>
      )}
      {connecting ? null : <BenchOffer onConnectBench={onConnectBench} />}
    </section>
  );
}
