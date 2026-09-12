import { useEffect, useState } from "react";
import {
  headline,
  stateOfCharge,
  type BatteryReadSnapshot,
  type BatteryRole,
} from "../battery";
import { freshness, valueText } from "../batteryFormat";
import { t, useLanguage } from "../i18n";

interface BatteryCardProps {
  snapshot: BatteryReadSnapshot;
  busy: boolean;
  running: boolean;
  /** Read the battery now; refused with a reason when nothing can be read. */
  onRead: () => void;
  onStop: () => void;
  /** Why the button is not offered, when it is not. */
  disabledReason: string | null;
}

/** Our own short words for the three readings a session turns on. */
const TILE_TITLE: Record<string, string> = {
  VOLTAGE: "Voltage",
  CURRENT: "Current",
  TEMPERATURE: "Temperature",
};

/** Re-render every half minute so "read 3 minutes ago" stays true. */
function useNow(intervalMs = 30_000): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), intervalMs);
    return () => clearInterval(timer);
  }, [intervalMs]);
  return now;
}

/**
 * The battery (ADR-0030) in the session column: how full it is, and the
 * three readings a session turns on. Everything else the battery monitor
 * holds is in the battery panel — thirty-odd rows are a panel's worth of
 * reading, not a rail's. Nothing here is a verdict: no colour says good or
 * bad, and a reading always carries the time it was taken.
 */
export function BatteryCard({
  snapshot,
  busy,
  running,
  onRead,
  onStop,
  disabledReason,
}: BatteryCardProps) {
  useLanguage();
  const now = useNow();
  const charge = stateOfCharge(snapshot);
  const synthetic = snapshot.routeValidation === "SYNTHETIC";
  const taken = freshness(snapshot.readUnixMs, now);
  const tiles: BatteryRole[] = ["VOLTAGE", "CURRENT", "TEMPERATURE"];

  return (
    <section className="battery-card" aria-labelledby="battery-title">
      <div className="battery-card__head">
        <h3 id="battery-title">{t("Battery")}</h3>
        {running ? (
          <button type="button" className="battery-card__action" onClick={onStop}>
            {t("Stop")}
          </button>
        ) : (
          <button
            type="button"
            className="battery-card__action"
            onClick={onRead}
            disabled={busy || disabledReason !== null}
            title={disabledReason ?? undefined}
          >
            {snapshot.readings.length > 0 ? t("Read again") : t("Read the battery")}
          </button>
        )}
      </div>

      <div className="battery-card__level" aria-live="polite">
        <div
          className="battery-card__gauge"
          role="img"
          aria-label={
            charge === null
              ? t("State of charge: not read")
              : t("State of charge: {percent}%", { percent: charge })
          }
        >
          <div
            className="battery-card__fill"
            style={{ width: `${Math.max(0, Math.min(100, charge ?? 0))}%` }}
          />
        </div>
        <div className="battery-card__reading">
          <strong>{charge === null ? "—" : `${charge}%`}</strong>
          <span>{taken ?? t("not read yet")}</span>
        </div>
      </div>

      {snapshot.readings.length > 0 ? (
        <dl className="battery-card__tiles">
          {tiles.map((role) => {
            const row = headline(snapshot, role);
            return (
              <div className="battery-card__tile" key={role}>
                <dt>{t(TILE_TITLE[role])}</dt>
                <dd>{row === null ? "—" : valueText(row)}</dd>
              </div>
            );
          })}
        </dl>
      ) : null}

      {snapshot.error !== null ? (
        <p className="battery-card__note battery-card__note--reason">
          {snapshot.error.technicalDetails ?? snapshot.error.message}
        </p>
      ) : null}

      <p className="battery-card__note">
        {synthetic
          ? t("Bench values: nothing here was measured on a car.")
          : t("The rest of the battery is in its own panel; nothing here is a verdict.")}
      </p>
    </section>
  );
}
