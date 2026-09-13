import { headline, stateOfCharge, type BatteryReadSnapshot } from "../battery";
import { valueText } from "../batteryFormat";
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

/**
 * The battery (ADR-0030) as one key on the plate: the cell itself is the
 * button that starts the read, it fills to the state of charge, and the
 * voltage stands to its left once there is one. Everything else the battery
 * monitor holds is in the battery panel — thirty-odd rows are a panel's
 * worth of reading, not a key's. Nothing here is a verdict: no colour says
 * good or bad.
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
  const charge = stateOfCharge(snapshot);
  const volts = headline(snapshot, "VOLTAGE");
  const blocked = !running && (busy || disabledReason !== null);
  return (
    <div className="battery-key">
      {volts === null ? null : (
        <span className="battery-key__volts" aria-live="polite">
          {valueText(volts)}
        </span>
      )}
      <button
        type="button"
        className="battery-key__cell"
        onClick={running ? onStop : onRead}
        disabled={blocked}
        title={disabledReason ?? snapshot.error?.message ?? undefined}
        aria-label={
          charge === null
            ? t("State of charge: not read")
            : t("State of charge: {percent}%", { percent: charge })
        }
      >
        <span
          className="battery-key__fill"
          style={{ width: `${Math.max(0, Math.min(100, charge ?? 0))}%` }}
          aria-hidden="true"
        />
        <span className="battery-key__pole battery-key__pole--minus" aria-hidden="true">
          −
        </span>
        <span className="battery-key__word">{running ? t("Stop") : t("Read")}</span>
        <span className="battery-key__pole battery-key__pole--plus" aria-hidden="true">
          +
        </span>
      </button>
    </div>
  );
}
