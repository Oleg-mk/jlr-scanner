import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";
import type { ModuleReadState } from "./moduleRead";

/**
 * The odometer read from every module (ADR-0024). A car keeps its mileage in
 * dozens of places and a rolled-back car is rolled back only where the tool
 * could reach, so every module is asked once and the answers are put side by
 * side. The difference is arithmetic over two readings — never a verdict.
 */
export type MileageSurveyState = "IDLE" | "RUNNING" | "FINISHED";

/** A module's running total, or the odometer as it stood at a recorded event. */
export type MileageKind = "CURRENT" | "EVENT";

export interface MileageReading {
  ecuFamily: string;
  identifier: string;
  /** SDD's own name for the parameter, in its own English. */
  parameter: string;
  kind: MileageKind;
  state: ModuleReadState;
  value: string | null;
  unit: string | null;
  number: number | null;
  raw: number | null;
  /** From the highest running total on the car; null while there is none. */
  difference: number | null;
  routeId: string;
  routeValidation: string;
  rawResponseHex: string | null;
  negativeResponse: string | null;
  note: string | null;
  reason: string | null;
}

export interface MileageSurveySnapshot {
  state: MileageSurveyState;
  readings: MileageReading[];
  planned: number;
  asked: number;
  answered: number;
  highest: number | null;
  highestModule: string | null;
  unit: string | null;
  routeValidation: string;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface MileageClient {
  getState(): Promise<MileageSurveySnapshot>;
  start(context: VehicleDescription): Promise<MileageSurveySnapshot>;
  step(): Promise<MileageSurveySnapshot>;
  finish(): Promise<MileageSurveySnapshot>;
}

export const createMileageSnapshot = (): MileageSurveySnapshot => ({
  state: "IDLE",
  readings: [],
  planned: 0,
  asked: 0,
  answered: 0,
  highest: null,
  highestModule: null,
  unit: null,
  routeValidation: "",
  error: null,
  reportAvailable: false,
});

class TauriMileageClient implements MileageClient {
  getState() {
    return invoke<MileageSurveySnapshot>("get_mileage_state");
  }

  start(context: VehicleDescription) {
    return invoke<MileageSurveySnapshot>("start_mileage_survey", { request: { context } });
  }

  step() {
    return invoke<MileageSurveySnapshot>("mileage_survey_step");
  }

  finish() {
    return invoke<MileageSurveySnapshot>("finish_mileage_survey");
  }
}

/** Three modules that disagree, for the browser preview; nothing behind it. */
const DEMO: Array<[string, string, MileageKind, number | null, string | null]> = [
  ["IPC", "0x61BB", "CURRENT", 142_310, null],
  ["ABS", "0xDD01", "CURRENT", 268_905, null],
  ["RCM", "0xDD01", "CURRENT", 268_412, null],
  ["PCM", "0xDD01", "CURRENT", 142_310, null],
  ["TCM", "0xDD01", "CURRENT", null, "no answer within the timeout"],
  ["TCM", "0x1EC2", "EVENT", 265_770, null],
];

class BrowserMileageClient implements MileageClient {
  private snapshot = createMileageSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    if (!browserDemoEnabled()) {
      this.snapshot = {
        ...createMileageSnapshot(),
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails: "connect and verify the adapter before a mileage survey",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.snapshot);
    }
    this.snapshot = {
      ...createMileageSnapshot(),
      state: "RUNNING",
      planned: DEMO.length,
      routeValidation: "DEMO",
    };
    return Promise.resolve(this.snapshot);
  }

  step() {
    if (this.snapshot.state !== "RUNNING") return Promise.resolve(this.snapshot);
    const next = DEMO[this.snapshot.asked];
    if (next === undefined) {
      this.snapshot = { ...this.snapshot, state: "FINISHED", reportAvailable: true };
      return Promise.resolve(this.snapshot);
    }
    const [ecuFamily, identifier, kind, number, reason] = next;
    const readings = [
      ...this.snapshot.readings,
      {
        ecuFamily,
        identifier,
        parameter: kind === "EVENT" ? "Gearbox stall event history  -  Odometer reading" : "Total distance",
        kind,
        state: (reason === null ? "SUCCEEDED" : "FAILED") as ModuleReadState,
        value: number === null ? null : String(number),
        unit: number === null ? null : "km",
        number,
        raw: number,
        difference: null,
        routeId: "hs-can",
        routeValidation: "DEMO",
        rawResponseHex: number === null ? null : "62 DD 01 00 04 1A 19",
        negativeResponse: null,
        note: null,
        reason,
      },
    ];
    const highest = readings
      .filter((row) => row.kind === "CURRENT")
      .reduce<number | null>(
        (best, row) => (row.number === null ? best : Math.max(best ?? row.number, row.number)),
        null,
      );
    const highestRow = readings.find((row) => row.kind === "CURRENT" && row.number === highest);
    this.snapshot = {
      ...this.snapshot,
      readings: readings.map((row) => ({
        ...row,
        difference: row.number === null || highest === null ? null : row.number - highest,
      })),
      asked: this.snapshot.asked + 1,
      answered: this.snapshot.answered + (reason === null ? 1 : 0),
      highest,
      highestModule: highestRow?.ecuFamily ?? null,
      unit: highest === null ? null : "km",
    };
    return Promise.resolve(this.snapshot);
  }

  finish() {
    this.snapshot = { ...this.snapshot, state: "FINISHED", reportAvailable: true };
    return Promise.resolve(this.snapshot);
  }
}

export const defaultMileageClient: MileageClient = hasTauriRuntime()
  ? new TauriMileageClient()
  : new BrowserMileageClient();
