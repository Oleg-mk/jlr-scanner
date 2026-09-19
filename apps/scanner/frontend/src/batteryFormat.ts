import type { BatteryReading } from "./battery";
import { isLowVoltage, type VoltageReading } from "./voltage";
import { t } from "./i18n";
import { withUnit } from "./units";

export { unitLabel } from "./units";

/**
 * How a battery reading is worded (ADR-0030): the unit the data states in
 * the symbol a person reads, the value beside it, and how old the reading
 * is. Shared by the card on the session rail and the panel that holds
 * every row.
 */
/** The value with its unit, or an em dash where the module said nothing. */
export function valueText(reading: BatteryReading): string {
  if (reading.value === null) return "—";
  return withUnit(reading.value, reading.unit);
}

/**
 * How old the reading is. A battery reading during a session ages: the
 * session itself draws from that battery, so the time is part of the fact.
 */
export function freshness(readUnixMs: number | null, now: number): string | null {
  if (readUnixMs === null) return null;
  const minutes = Math.max(0, Math.floor((now - readUnixMs) / 60_000));
  if (minutes < 1) return t("read just now");
  return t("read {minutes} min ago", { minutes });
}

/** Where a voltage came from, in the reader's own words. */
export function voltageSourceText(reading: VoltageReading): string {
  switch (reading.source) {
    case "battery":
      return t("from the battery monitor");
    case "obd":
      return t("from the OBD read");
    default:
      return reading.module === null
        ? t("from the live read")
        : t("from the live read of {module}", { module: reading.module });
  }
}

/**
 * The session's precondition as one sentence (ADR-0030, amendments of
 * 2026-09-19): the freshest voltage any read has brought stands in SDD's own
 * low band, so the reading, where it came from, how old it is, SDD's edge,
 * and the request to connect an external supply before reading modules —
 * the way SDD asks before it starts a session. A sentence about the
 * session, not about the battery, and null whenever there is nothing to say.
 */
export function batteryPreconditionText(
  reading: VoltageReading | null,
  sddLowVoltageMaxMv: number,
  now: number,
): string | null {
  if (reading === null || !isLowVoltage(reading, sddLowVoltageMaxMv)) return null;
  const age = freshness(reading.atMs, now);
  const sentence = t(
    "The battery reads {volts} V, in SDD's low band, under {limit} V ({source}, {age}). Connect an external power supply before reading modules.",
    {
      volts: reading.text,
      limit: (sddLowVoltageMaxMv / 1000).toFixed(1),
      source: voltageSourceText(reading),
      age: age ?? t("read at an unknown time"),
    },
  );
  return reading.synthetic ? `${sentence} · ${t("bench, synthetic")}` : sentence;
}
