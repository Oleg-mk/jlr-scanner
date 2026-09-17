import type { BatteryReading } from "./battery";
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
