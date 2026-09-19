import { batteryPreconditionText } from "../batteryFormat";
import { useLanguage } from "../i18n";
import type { VoltageReading } from "../voltage";

/**
 * A low battery is a precondition of the session (ADR-0030, amendments of
 * 2026-09-19): one amber line under the header while the freshest voltage
 * any read has brought stands in SDD's own low band. Nothing here calls a
 * battery good or bad, and nothing is blocked by it.
 */
export function BatteryPrecondition({
  reading,
  sddLowVoltageMaxMv,
}: {
  reading: VoltageReading | null;
  sddLowVoltageMaxMv: number;
}) {
  useLanguage();
  const text = batteryPreconditionText(reading, sddLowVoltageMaxMv, Date.now());
  if (text === null) return null;
  return (
    <p className="battery-precondition" role="status">
      {text}
    </p>
  );
}
