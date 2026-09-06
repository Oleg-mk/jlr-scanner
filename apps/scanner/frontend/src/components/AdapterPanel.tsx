import { t, useLanguage } from "../i18n";
import type { AdapterSnapshot } from "../adapter";
import { StatusBadge } from "./StatusBadge";

interface AdapterPanelProps {
  snapshot: AdapterSnapshot;
  selectedPort: string | null;
  onSelectPort: (port: string) => void;
  onDetect: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
}

function hexId(value: number) {
  return value.toString(16).toUpperCase().padStart(4, "0");
}

export function AdapterPanel({
  snapshot,
  selectedPort,
  onSelectPort,
  onDetect,
  onConnect,
  onDisconnect,
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
        <div className="adapter-empty">
          <h3>{t("Adapter not detected")}</h3>
          <p>{t("Connect MongoosePro JLR by USB.")}</p>
          <button className="button button--primary" type="button" onClick={onDetect}>
            {t("Detect adapter")}
          </button>
        </div>
      </section>
    );
  }

  if (snapshot.state === "CONNECTED" && snapshot.adapter !== null) {
    const adapter = snapshot.adapter;
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
        <div className="adapter-title-row">
          <h3>{adapter.name}</h3>
          <StatusBadge tone="positive">{t("Connected")}</StatusBadge>
        </div>
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
      </section>
    );
  }

  const connecting = snapshot.state === "CONNECTING";
  const error = snapshot.state === "ERROR" ? snapshot.error : null;

  return (
    <section className="adapter-panel" aria-labelledby="adapter-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">{t("Connection")}</p>
          <h2 id="adapter-title">{t("Adapter")}</h2>
        </div>
      </div>
      {error !== null ? (
        <div className="error-banner" role="alert">
          <h3>{error.message}</h3>
          <p>{t("Check the USB connection and try again.")}</p>
          {error.technicalDetails !== null ? (
            <details>
              <summary>{t("Technical details")}</summary>
              <code>{error.technicalDetails}</code>
            </details>
          ) : null}
        </div>
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
    </section>
  );
}
