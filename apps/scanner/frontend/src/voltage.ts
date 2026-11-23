import { headline, type BatteryReadSnapshot } from "./battery";
import type { LiveReadSnapshot } from "./liveRead";
import type { StandardObdValue } from "./standardObd";
import type { ConnectorVoltageReading } from "./useConnectorVoltage";

/**
 * The rail's voltage, from whichever read last brought one (ADR-0030,
 * amendment of 2026-09-19, later).
 *
 * The owner's X250 declares no battery voltage in its battery monitor, so
 * the battery read alone never gives the card a number. Three reads can:
 * the battery read's headline voltage where a car has one, the legislated
 * OBD control-module voltage (PID 0x42), and a live read of a module's
 * supply voltage. The freshest of them is the voltage, with the time it
 * was taken and where it came from.
 *
 * Since 2026-11-15 (ADR-0030, amended) a fourth: the adapter's own
 * measurement at the connector, as SDD shows it, asked every few seconds
 * while the adapter is connected — so it is the freshest whenever there is
 * one. The earlier "the adapter measures nothing" was written without
 * looking; the vendor library reads it with one value command.
 */
export type VoltageSource = "battery" | "obd" | "live" | "adapter";

export interface VoltageReading {
  volts: number;
  /** The reading as the read gave it, for the sentence and the key. */
  text: string;
  source: VoltageSource;
  /** The module that answered, where one did. */
  module: string | null;
  /** When it was read, by the wall clock; null when that is not known. */
  atMs: number | null;
  synthetic: boolean;
}

/** The words the live limits' starter suggestion recognises (F15): a supply voltage. */
export const SUPPLY_VOLTAGE_NAME = /battery voltage|control module voltage/i;

export interface VoltageInputs {
  battery: BatteryReadSnapshot;
  obdValues: StandardObdValue[];
  obdAtMs: number | null;
  obdRouteValidation: string;
  live: LiveReadSnapshot;
  liveStartedAtMs: number | null;
  /** The adapter's measurement at the connector, when it has given one. */
  adapter?: ConnectorVoltageReading | null;
}

function fromAdapter(reading: ConnectorVoltageReading | null | undefined): VoltageReading | null {
  if (reading === null || reading === undefined) return null;
  // As the vendor library gives it: to the nearest tenth of a volt.
  const volts = Math.round(reading.millivolts / 100) / 10;
  return {
    volts,
    text: volts.toFixed(1),
    source: "adapter",
    module: null,
    atMs: reading.atMs,
    synthetic: reading.synthetic,
  };
}

function fromBattery(snapshot: BatteryReadSnapshot): VoltageReading | null {
  const reading = headline(snapshot, "VOLTAGE");
  if (reading === null || reading.value === null) return null;
  if (reading.unit !== null && reading.unit !== "V") return null;
  const volts = Number(reading.value);
  if (!Number.isFinite(volts)) return null;
  return {
    volts,
    text: reading.value,
    source: "battery",
    module: reading.ecuFamily,
    atMs: snapshot.readUnixMs,
    synthetic: snapshot.routeValidation === "SYNTHETIC",
  };
}

function fromObd(values: StandardObdValue[], atMs: number | null, routeValidation: string): VoltageReading | null {
  const value = values.find(
    (candidate) =>
      candidate.number !== null &&
      candidate.unit === "V" &&
      (candidate.pid.toLowerCase() === "0x42" || SUPPLY_VOLTAGE_NAME.test(candidate.name)),
  );
  if (value === undefined || value.number === null) return null;
  return {
    volts: value.number,
    text: value.value ?? String(value.number),
    source: "obd",
    module: null,
    atMs,
    synthetic: routeValidation === "SYNTHETIC",
  };
}

function fromLive(live: LiveReadSnapshot, startedAtMs: number | null): VoltageReading | null {
  let best: VoltageReading | null = null;
  for (const value of live.values) {
    if (value.value === null || value.unit !== "V" || !SUPPLY_VOLTAGE_NAME.test(value.name)) continue;
    const volts = Number(value.value);
    if (!Number.isFinite(volts)) continue;
    const reading: VoltageReading = {
      volts,
      text: value.value,
      source: "live",
      module: value.ecuFamily,
      atMs: startedAtMs === null ? null : startedAtMs + value.atMs,
      synthetic: live.routeValidation === "SYNTHETIC",
    };
    if (best === null || (reading.atMs ?? -Infinity) > (best.atMs ?? -Infinity)) best = reading;
  }
  return best;
}

/** The freshest voltage any read has brought, or null when none has. */
export function latestVoltage(inputs: VoltageInputs): VoltageReading | null {
  const candidates = [
    fromBattery(inputs.battery),
    fromObd(inputs.obdValues, inputs.obdAtMs, inputs.obdRouteValidation),
    fromLive(inputs.live, inputs.liveStartedAtMs),
    fromAdapter(inputs.adapter),
  ].filter((reading): reading is VoltageReading => reading !== null);
  let best: VoltageReading | null = null;
  for (const reading of candidates) {
    // A reading without a known time is older than any with one; ties keep
    // the order above.
    if (best === null || (reading.atMs ?? -Infinity) > (best.atMs ?? -Infinity)) best = reading;
  }
  return best;
}

/** Whether a reading stands in SDD's low band; nothing is said without a band. */
export function isLowVoltage(reading: VoltageReading | null, sddLowVoltageMaxMv: number): boolean {
  if (reading === null || sddLowVoltageMaxMv <= 0) return false;
  return reading.volts < sddLowVoltageMaxMv / 1000;
}
