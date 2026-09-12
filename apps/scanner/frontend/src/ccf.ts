import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";
import type { ModuleReadState } from "./moduleRead";

/**
 * The car configuration file (ADR-0028): what the car is fitted with and how
 * it is set, read block by block from the module the data names as its
 * keeper and from the modules holding copies, decoded with the data's own
 * layout. Every value is what its type says; nothing here judges it.
 */
export type CcfReadState = "IDLE" | "RUNNING" | "FINISHED";

export interface CcfReading {
  ecuFamily: string;
  role: string;
  block: string;
  /** SDD's name for the parameter: the row's identity. */
  parameter: string;
  group: string;
  groupTitleEn: string;
  groupTitleRu: string;
  titleEn: string;
  titleRu: string;
  kind: string;
  /** Whether SDD's editor shows this row. */
  display: boolean;
  scope: string;
  valueEn: string | null;
  valueRu: string | null;
  optionName: string | null;
  optionCode: string | null;
  raw: number | null;
  hex: string | null;
  note: string | null;
}

export interface CcfDifference {
  block: string;
  parameter: string;
  masterModule: string;
  masterValue: string | null;
  copyModule: string;
  copyValue: string | null;
}

export interface CcfBlockRead {
  ecuFamily: string;
  role: string;
  identifier: string;
  blocks: string[];
  state: ModuleReadState;
  bytes: number | null;
  routeValidation: string;
  rawResponseHex: string | null;
  negativeResponse: string | null;
  reason: string | null;
}

export interface CcfReadSnapshot {
  state: CcfReadState;
  scheme: string | null;
  masterModule: string | null;
  reads: CcfBlockRead[];
  readings: CcfReading[];
  differences: CcfDifference[];
  planned: number;
  asked: number;
  answered: number;
  hidden: number;
  routeValidation: string;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface CcfClient {
  getState(): Promise<CcfReadSnapshot>;
  start(context: VehicleDescription): Promise<CcfReadSnapshot>;
  step(): Promise<CcfReadSnapshot>;
  finish(): Promise<CcfReadSnapshot>;
}

export const createCcfSnapshot = (): CcfReadSnapshot => ({
  state: "IDLE",
  scheme: null,
  masterModule: null,
  reads: [],
  readings: [],
  differences: [],
  planned: 0,
  asked: 0,
  answered: 0,
  hidden: 0,
  routeValidation: "",
  error: null,
  reportAvailable: false,
});

class TauriCcfClient implements CcfClient {
  getState() {
    return invoke<CcfReadSnapshot>("get_ccf_state");
  }

  start(context: VehicleDescription) {
    return invoke<CcfReadSnapshot>("start_ccf_read", { request: { context } });
  }

  step() {
    return invoke<CcfReadSnapshot>("ccf_read_step");
  }

  finish() {
    return invoke<CcfReadSnapshot>("finish_ccf_read");
  }
}

/** A handful of rows for the browser preview; nothing behind them. */
const DEMO: Array<[string, string, string, boolean, string, string | null]> = [
  ["PARAM_CCF_BRAND", "GROUP_CCF_BRAND", "Brand", true, "ENUM", "Jaguar"],
  ["PARAM_CCF_MARKET", "GROUP_CCF_MARKET", "Market", true, "ENUM", "Europe"],
  ["PARAM_CCF_ENGINE_TYPE", "GROUP_CCF_ENGINE", "Engine", true, "ENUM", "5.0L V8 supercharged"],
  ["PARAM_CCF_HEATED_SEATS", "GROUP_CCF_SEATS", "Heated seats", true, "BOOL", "Fitted"],
  ["PARAM_CCF_TYRE_DYNAMIC_ROLLING_RADIUS", "GROUP_CCF_TYRES", "Tyre rolling radius", false, "BIN", "1013"],
];

class BrowserCcfClient implements CcfClient {
  private snapshot = createCcfSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    if (!browserDemoEnabled()) {
      this.snapshot = {
        ...createCcfSnapshot(),
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails: "connect and verify the adapter before reading the configuration",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.snapshot);
    }
    this.snapshot = {
      ...createCcfSnapshot(),
      state: "RUNNING",
      scheme: "did",
      masterModule: "RSJB",
      planned: 1,
      routeValidation: "DEMO",
    };
    return Promise.resolve(this.snapshot);
  }

  step() {
    if (this.snapshot.state !== "RUNNING") return Promise.resolve(this.snapshot);
    const readings: CcfReading[] = DEMO.map(([parameter, group, title, display, kind, value]) => ({
      ecuFamily: "RSJB",
      role: "sync",
      block: "CCF",
      parameter,
      group,
      groupTitleEn: title,
      groupTitleRu: "",
      titleEn: "",
      titleRu: "",
      kind,
      display,
      scope: "base",
      valueEn: value,
      valueRu: value,
      optionName: kind === "ENUM" || kind === "BOOL" ? value : null,
      optionCode: null,
      raw: kind === "BIN" ? Number(value) : null,
      hex: null,
      note: null,
    }));
    this.snapshot = {
      ...this.snapshot,
      state: "FINISHED",
      asked: 1,
      answered: 1,
      readings,
      hidden: readings.filter((row) => !row.display).length,
      reads: [
        {
          ecuFamily: "RSJB",
          role: "sync",
          identifier: "0xF106",
          blocks: ["CCF"],
          state: "SUCCEEDED",
          bytes: 196,
          routeValidation: "DEMO",
          rawResponseHex: "62 F1 06 FD 01 …",
          negativeResponse: null,
          reason: null,
        },
      ],
      reportAvailable: true,
    };
    return Promise.resolve(this.snapshot);
  }

  finish() {
    this.snapshot = { ...this.snapshot, state: "FINISHED", reportAvailable: true };
    return Promise.resolve(this.snapshot);
  }
}

export const defaultCcfClient: CcfClient = hasTauriRuntime()
  ? new TauriCcfClient()
  : new BrowserCcfClient();
