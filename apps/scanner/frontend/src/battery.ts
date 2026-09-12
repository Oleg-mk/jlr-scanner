import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";
import type { ModuleReadState } from "./moduleRead";

/**
 * The battery (ADR-0030): what the battery monitor holds, read from the
 * modules the loaded data names — how full it is, what it is doing now, what
 * it leaks while parked, how it has aged, what the car remembers about it,
 * and, on a hybrid, the traction battery from its own module.
 *
 * The card judges nothing. A value the data describes is the number it
 * describes; a value it does not is the bytes it is. Where a band appears
 * behind a voltage it is SDD's own band, named as SDD's.
 */
export type BatteryReadState = "IDLE" | "RUNNING" | "FINISHED";

/** What a parameter is for; the card groups by this. */
export type BatteryRole =
  | "CHARGE"
  | "VOLTAGE"
  | "CURRENT"
  | "TEMPERATURE"
  | "DRAIN"
  | "HEALTH"
  | "HISTORY"
  | "CONFIGURATION"
  | "HYBRID";

/** The order the groups are shown in: what a person asks first, first. */
export const BATTERY_ROLES: BatteryRole[] = [
  "CHARGE",
  "VOLTAGE",
  "CURRENT",
  "TEMPERATURE",
  "DRAIN",
  "HEALTH",
  "HISTORY",
  "CONFIGURATION",
  "HYBRID",
];

export interface BatteryReading {
  ecuFamily: string;
  identifier: string;
  /** SDD's own name for the parameter: the row's identity. */
  parameter: string;
  role: BatteryRole;
  /** Whether the reading belongs on the card's face. */
  headline: boolean;
  state: ModuleReadState;
  value: string | null;
  /** The unit the data states, never one inferred from a name. */
  unit: string | null;
  routeId: string;
  routeValidation: string;
  rawResponseHex: string | null;
  negativeResponse: string | null;
  note: string | null;
  reason: string | null;
}

export interface BatteryRefusal {
  ecuFamily: string;
  reason: string;
}

export interface BatteryReadSnapshot {
  state: BatteryReadState;
  readings: BatteryReading[];
  planned: number;
  asked: number;
  answered: number;
  modules: number;
  routeValidation: string;
  /** When the run began, so an old reading is not mistaken for a fresh one. */
  readUnixMs: number | null;
  refused: BatteryRefusal[];
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface BatteryClient {
  getState(): Promise<BatteryReadSnapshot>;
  start(context: VehicleDescription): Promise<BatteryReadSnapshot>;
  step(): Promise<BatteryReadSnapshot>;
  finish(): Promise<BatteryReadSnapshot>;
}

export const createBatterySnapshot = (): BatteryReadSnapshot => ({
  state: "IDLE",
  readings: [],
  planned: 0,
  asked: 0,
  answered: 0,
  modules: 0,
  routeValidation: "",
  readUnixMs: null,
  refused: [],
  error: null,
  reportAvailable: false,
});

/**
 * The state of charge as a number, when the run read one. The card draws the
 * bar from this and shows nothing where there is none: a battery with no
 * stated state of charge is not a battery at zero.
 */
export function stateOfCharge(snapshot: BatteryReadSnapshot): number | null {
  const row = snapshot.readings.find(
    (reading) =>
      reading.role === "CHARGE" && reading.headline && reading.value !== null,
  );
  if (row === undefined || row.value === null) return null;
  const value = Number.parseFloat(row.value.replace(",", "."));
  return Number.isFinite(value) ? value : null;
}

/** The headline readings of one role, in the order they were read. */
export function headline(
  snapshot: BatteryReadSnapshot,
  role: BatteryRole,
): BatteryReading | null {
  return (
    snapshot.readings.find(
      (reading) =>
        reading.role === role && reading.headline && reading.value !== null,
    ) ?? null
  );
}

class TauriBatteryClient implements BatteryClient {
  getState() {
    return invoke<BatteryReadSnapshot>("get_battery_state");
  }

  start(context: VehicleDescription) {
    return invoke<BatteryReadSnapshot>("start_battery_read", {
      request: { context },
    });
  }

  step() {
    return invoke<BatteryReadSnapshot>("battery_read_step");
  }

  finish() {
    return invoke<BatteryReadSnapshot>("finish_battery_read");
  }
}

/** A few rows for the browser preview; nothing behind them. */
const DEMO: Array<[string, BatteryRole, boolean, string, string | null]> = [
  ["Vehicle Battery Estimated State of Charge", "CHARGE", true, "78", "pct"],
  ["Vehicle Battery Voltage", "VOLTAGE", true, "12.6", "V"],
  ["Battery current", "CURRENT", true, "-1.5", "A"],
  ["Vehicle Battery Temperature - Estimated", "TEMPERATURE", true, "21", "degC"],
  ["Average Vehicle Quiesent Current - Previous 24 Hours (mA)", "DRAIN", true, "23", null],
  ["Estimated Cold Cranking Voltage At Present State Of Charge", "HEALTH", true, "10.9", "V"],
  ["Battery Time in Service (Days)", "HISTORY", true, "1287", null],
  ["Number of Vehicle Battery Monitor Resets", "HISTORY", true, "2", null],
  ["Battery Type", "CONFIGURATION", false, "41 47 4D", null],
];

class BrowserBatteryClient implements BatteryClient {
  private snapshot = createBatterySnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    if (!browserDemoEnabled()) {
      this.snapshot = {
        ...createBatterySnapshot(),
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails:
            "connect and verify the adapter before reading the battery",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.snapshot);
    }
    this.snapshot = {
      ...createBatterySnapshot(),
      state: "RUNNING",
      planned: DEMO.length,
      modules: 1,
      routeValidation: "DEMO",
      readUnixMs: Date.now(),
    };
    return Promise.resolve(this.snapshot);
  }

  step() {
    if (this.snapshot.state !== "RUNNING") return Promise.resolve(this.snapshot);
    const readings: BatteryReading[] = DEMO.map(
      ([parameter, role, isHeadline, value, unit]) => ({
        ecuFamily: "BCM",
        identifier: "0x4028",
        parameter,
        role,
        headline: isHeadline,
        state: "SUCCEEDED" as ModuleReadState,
        value,
        unit,
        routeId: "hs-can",
        routeValidation: "DEMO",
        rawResponseHex: "62 40 28 4E",
        negativeResponse: null,
        note: null,
        reason: null,
      }),
    );
    this.snapshot = {
      ...this.snapshot,
      state: "FINISHED",
      asked: DEMO.length,
      answered: DEMO.length,
      readings,
      reportAvailable: true,
    };
    return Promise.resolve(this.snapshot);
  }

  finish() {
    this.snapshot = {
      ...this.snapshot,
      state: "FINISHED",
      reportAvailable: true,
    };
    return Promise.resolve(this.snapshot);
  }
}

export const defaultBatteryClient: BatteryClient = hasTauriRuntime()
  ? new TauriBatteryClient()
  : new BrowserBatteryClient();
