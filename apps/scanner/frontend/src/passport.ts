import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";
import type { ModuleReadState } from "./moduleRead";

/**
 * The module passport (ADR-0027): what a module says it is — the part numbers
 * fitted, its serial, the software it runs — read over the identification
 * identifiers SDD declares for it. The text is shown as the module holds it;
 * nothing is parsed out of a part number and nothing is compared.
 */
export type ModulePassportState = "IDLE" | "RUNNING" | "FINISHED";

export interface PassportReading {
  ecuFamily: string;
  identifier: string;
  /** SDD's own name for the identifier, in its own English: the row's identity. */
  parameter: string;
  state: ModuleReadState;
  /** The text the module holds, padding trimmed; bytes when not text. */
  value: string | null;
  routeId: string;
  routeValidation: string;
  rawResponseHex: string | null;
  negativeResponse: string | null;
  note: string | null;
  reason: string | null;
}

export interface ModulePassportSnapshot {
  state: ModulePassportState;
  readings: PassportReading[];
  planned: number;
  asked: number;
  answered: number;
  modules: number;
  routeValidation: string;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface PassportClient {
  getState(): Promise<ModulePassportSnapshot>;
  /** Every module the survey reaches, or only the ones named. */
  start(context: VehicleDescription, ecuFamilies?: string[]): Promise<ModulePassportSnapshot>;
  step(): Promise<ModulePassportSnapshot>;
  finish(): Promise<ModulePassportSnapshot>;
}

export const createPassportSnapshot = (): ModulePassportSnapshot => ({
  state: "IDLE",
  readings: [],
  planned: 0,
  asked: 0,
  answered: 0,
  modules: 0,
  routeValidation: "",
  error: null,
  reportAvailable: false,
});

class TauriPassportClient implements PassportClient {
  getState() {
    return invoke<ModulePassportSnapshot>("get_passport_state");
  }

  start(context: VehicleDescription, ecuFamilies?: string[]) {
    return invoke<ModulePassportSnapshot>("start_module_passport", {
      request: { context, ecuFamilies: ecuFamilies ?? null },
    });
  }

  step() {
    return invoke<ModulePassportSnapshot>("module_passport_step");
  }

  finish() {
    return invoke<ModulePassportSnapshot>("finish_module_passport");
  }
}

/** Two modules' passports for the browser preview; nothing behind them. */
const DEMO: Array<[string, string, string, string | null, string | null]> = [
  ["PCM", "0xF111", "ECU Core Assembly Number", "8X23-14C088-AB", null],
  ["PCM", "0xF188", "ECU Software Number", "8X23-14C204-CD", null],
  ["PCM", "0xF18C", "ECU Serial Number", "0000471D3E92A1", null],
  ["ABS", "0xF111", "ECU Core Assembly Number", "9X23-2C405-AE", null],
  ["ABS", "0xF188", "ECU Software Number", null, "no answer within the timeout"],
];

class BrowserPassportClient implements PassportClient {
  private snapshot = createPassportSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    if (!browserDemoEnabled()) {
      this.snapshot = {
        ...createPassportSnapshot(),
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails: "connect and verify the adapter before reading module passports",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.snapshot);
    }
    this.snapshot = {
      ...createPassportSnapshot(),
      state: "RUNNING",
      planned: DEMO.length,
      modules: 2,
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
    const [ecuFamily, identifier, parameter, value, reason] = next;
    this.snapshot = {
      ...this.snapshot,
      readings: [
        ...this.snapshot.readings,
        {
          ecuFamily,
          identifier,
          parameter,
          state: (reason === null ? "SUCCEEDED" : "FAILED") as ModuleReadState,
          value,
          routeId: "hs-can",
          routeValidation: "DEMO",
          rawResponseHex: value === null ? null : "62 F1 11 38 58 32 33",
          negativeResponse: null,
          note: null,
          reason,
        },
      ],
      asked: this.snapshot.asked + 1,
      answered: this.snapshot.answered + (reason === null ? 1 : 0),
    };
    return Promise.resolve(this.snapshot);
  }

  finish() {
    this.snapshot = { ...this.snapshot, state: "FINISHED", reportAvailable: true };
    return Promise.resolve(this.snapshot);
  }
}

export const defaultPassportClient: PassportClient = hasTauriRuntime()
  ? new TauriPassportClient()
  : new BrowserPassportClient();
