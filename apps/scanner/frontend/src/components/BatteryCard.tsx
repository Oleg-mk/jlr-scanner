import { stateOfCharge, type BatteryReadSnapshot } from "../battery";
import { voltageSourceText } from "../batteryFormat";
import { t, useLanguage } from "../i18n";
import type { VoltageReading } from "../voltage";

interface BatteryCardProps {
  snapshot: BatteryReadSnapshot;
  busy: boolean;
  running: boolean;
  /** Read the battery now; refused with a reason when nothing can be read. */
  onRead: () => void;
  onStop: () => void;
  /** Why the button is not offered, when it is not. */
  disabledReason: string | null;
  /**
   * The freshest voltage any read has brought (ADR-0030, 2026-09-19): the
   * battery read's own, the legislated OBD one, or a live read's. Carried
   * on the cell once there is one; where it came from is on the tooltip.
   */
  volts?: VoltageReading | null;
}

/**
 * The battery (ADR-0030) as one key on the plate: the cell itself is the
 * button that starts the read, it fills to the state of charge, and once a
 * read has brought a voltage the cell carries it in bold, giving way to the
 * word while the pointer is over it, so the key still says what it does
 * (the owner, 2026-09-19). Everything else the battery
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
  volts = null,
}: BatteryCardProps) {
  useLanguage();
  const charge = stateOfCharge(snapshot);
  const blocked = !running && (busy || disabledReason !== null);
  const word = running ? t("Stop") : t("Read");
  return (
    <div className="battery-key">
      <button
        type="button"
        className="battery-key__cell"
        onClick={running ? onStop : onRead}
        disabled={blocked}
        title={
          disabledReason ??
          snapshot.error?.message ??
          (volts === null ? undefined : voltageSourceText(volts))
        }
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
        <span className="battery-key__word">
          {volts === null || running ? (
            <span className="battery-key__say">{word}</span>
          ) : (
            <>
              <span className="battery-key__volts" aria-live="polite">
                {volts.text} V
              </span>
              <span className="battery-key__say battery-key__say--under">{word}</span>
            </>
          )}
        </span>
        <span className="battery-key__pole battery-key__pole--plus" aria-hidden="true">
          +
        </span>
      </button>
    </div>
  );
}
