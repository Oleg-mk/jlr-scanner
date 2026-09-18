import { useEffect, useState } from "react";
import { t, useLanguage } from "../i18n";
import {
  BENCH_SCENARIO_DEFAULT,
  BENCH_TRANSPORT,
  type AdapterSnapshot,
  type UserFacingError,
} from "../adapter";
import type { AdapterPending } from "../useAdapterController";
import { StatusBadge } from "./StatusBadge";

interface AdapterPanelProps {
  snapshot: AdapterSnapshot;
  selectedPort: string | null;
  onSelectPort: (port: string) => void;
  onDetect: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
  /** The bench (ADR-0020): a virtual vehicle from the library, no adapter and no car. */
  onConnectBench: (scenario: number) => void;
  /** Which of the two the person is waiting on, while they are waiting. */
  pending?: AdapterPending;
  /**
   * The modules are being surveyed. The bench is built from what that survey
   * finds, so it cannot be connected until the survey is done; the button
   * says so rather than standing there and taking minutes (the owner,
   * 2026-09-18).
   */
  surveying?: boolean;
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

function BenchOffer({
  onConnectBench,
  surveying,
}: {
  onConnectBench: (scenario: number) => void;
  surveying: boolean;
}) {
  const [scenario, setScenario] = useState(String(BENCH_SCENARIO_DEFAULT));
  const chosen = Number.parseInt(scenario, 10);
  const valid = Number.isInteger(chosen) && chosen >= 0 && chosen <= 999;
  return (
    <div className="bench-offer">
      <div className="bench-offer-row">
        <button
          className="button button--quiet"
          type="button"
          disabled={!valid || surveying}
          onClick={() => onConnectBench(chosen)}
        >
          {t("Connect the bench (virtual vehicle)")}
        </button>
        <label className="bench-scenario">
          <span>{t("Scenario")}</span>
          <input
            type="number"
            min={0}
            max={999}
            value={scenario}
            onChange={(event) => setScenario(event.target.value)}
          />
        </label>
      </div>
      {surveying ? (
        <p className="button-hint">
          {t(
            "The modules are being surveyed. The bench answers for what that survey finds, so it waits for it to end.",
          )}
        </p>
      ) : null}
      <p className="button-hint">
        {t(
          "No adapter and no car needed: a virtual vehicle built from the library answers instead. Every value is synthetic.",
        )}
      </p>
      <p className="button-hint">
        {t(
          "The scenario decides the fault codes: 0 is a vehicle in good order with none at all, and any other number draws codes the library describes for each module — the same number always draws the same ones.",
        )}
      </p>
    </div>
  );
}

/** Four hex digits, the way a USB identifier is written. */
function hex4(value: number): string {
  return value.toString(16).toUpperCase().padStart(4, "0");
}

export function AdapterPanel({
  snapshot,
  selectedPort,
  onSelectPort,
  onDetect,
  onConnect,
  onDisconnect,
  onConnectBench,
  pending = null,
  surveying = false,
}: AdapterPanelProps) {
  useLanguage();
  const connecting = snapshot.state === "CONNECTING";
  /*
   * The seconds, counted under the thing the person is waiting on, the way
   * the library load and the module survey already count them. A message
   * that does not move reads as a hang however honest its words are (the
   * owner, 2026-09-18).
   */
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => {
    if (!connecting) {
      setElapsed(0);
      return;
    }
    const started = Date.now();
    setElapsed(0);
    const timer = window.setInterval(
      () => setElapsed(Math.round((Date.now() - started) / 1000)),
      1000,
    );
    return () => window.clearInterval(timer);
  }, [connecting]);
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
        {snapshot.error !== null ? (
          <ErrorBanner
            error={snapshot.error}
            hint={
              snapshot.error.code === "SESSION_BUSY"
                ? t("Wait for the survey to end, then connect the bench again.")
                : null
            }
          />
        ) : null}
        <div className="adapter-empty">
          <h3>{t("Adapter not detected")}</h3>
          <p>{t("Connect MongoosePro JLR by USB.")}</p>
          {(snapshot.otherVendorDevices ?? []).map((device) => (
            <p className="adapter-other-device" key={device.port}>
              {t(
                "A device of the same maker is on {port}: USB {vid}:{pid}. That is not the MongoosePro JLR this application speaks to.",
                {
                  port: device.port,
                  vid: hex4(device.usbVid),
                  pid: hex4(device.usbPid),
                },
              )}
            </p>
          ))}
          <button className="button button--primary" type="button" onClick={onDetect}>
            {t("Detect adapter")}
          </button>
        </div>
        <BenchOffer onConnectBench={onConnectBench} surveying={surveying} />
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
              <div>
                <dt>{t("Scenario")}</dt>
                <dd>
                  {snapshot.benchScenario ?? BENCH_SCENARIO_DEFAULT}
                  {(snapshot.benchScenario ?? BENCH_SCENARIO_DEFAULT) === 0
                    ? ` — ${t("a vehicle in good order, no fault codes")}`
                    : ""}
                </dd>
              </div>
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

  const error = snapshot.error;

  /*
   * While a connection is being made the card says so, and says which one.
   * It used to fall through to the empty state and show a button reading
   * "detect adapter" — so building the bench, which on a surveyed car is not
   * instant, looked like an application waiting to be told to look for
   * hardware. The owner sat in front of that button for minutes
   * (2026-09-18).
   */
  if (connecting) {
    const bench = pending === "bench";
    return (
      <section className="adapter-panel" aria-labelledby="adapter-title">
        <div className="section-heading">
          <div>
            <p className="eyebrow">{t("Connection")}</p>
            <h2 id="adapter-title">{t("Adapter")}</h2>
          </div>
        </div>
        <div className="adapter-title-row">
          <h3>{bench ? t("Virtual vehicle (bench)") : (detected?.name ?? t("Adapter"))}</h3>
          <StatusBadge tone="pending">{t("Connecting")}</StatusBadge>
        </div>
        <p className="library-status" role="status">
          {bench ? t("Connecting the bench…") : t("Connecting the adapter…")} {elapsed}
          &nbsp;{t("sec")}
          <br />
          <span className="button-hint">
            {bench
              ? t(
                  "The virtual vehicle is being built from the library for the vehicle this session describes. Nothing is sent anywhere.",
                )
              : t("Opening the port and asking the board what it is.")}
          </span>
        </p>
      </section>
    );
  }

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
              : error.code === "SESSION_BUSY"
                ? t("Wait for the survey to end, then connect the bench again.")
                : error.code === "BENCH_FAILED"
                  ? t("Try the bench again; it needs no adapter.")
                  : t("Check the USB connection and try again.")
          }
        />
      ) : null}
      {detected !== null ? (
        <>
          <div className="adapter-title-row">
            <h3>{t("{name} detected", { name: detected.name })}</h3>
            <StatusBadge tone="neutral">{detected.port}</StatusBadge>
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
              disabled={snapshot.selectionRequired && selectedPort === null}
            >
              {t("Connect")}
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
      <BenchOffer onConnectBench={onConnectBench} surveying={surveying} />
    </section>
  );
}
