import { invoke } from "@tauri-apps/api/core";
import { demoReadOutcome } from "./demoVehicle";
import { saveTextFile } from "./files";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import type { VehicleDescription } from "./library";

export type ModuleReadState = "IDLE" | "SUCCEEDED" | "FAILED";
export type ModuleReadKind = "FAULT_CODES" | "IDENTIFIER";

export interface ModuleReadRequest {
  ecuFamily: string;
  kind: ModuleReadKind;
  identifier: string | null;
  context: VehicleDescription;
}

export interface DtcSummary {
  code: string;
  failureType: string;
  status: string;
  description: string | null;
  descriptionScope: string | null;
  failureTypeText: string | null;
  /** The failure type wording by SDD language code, when the data has it. */
  failureTypeTexts: Record<string, string>;
}

export interface DecodedParameterSummary {
  name: string;
  raw: number | null;
  value: string | null;
  unit: string | null;
  state: string | null;
  note: string | null;
}

export interface ModuleReadSnapshot {
  state: ModuleReadState;
  ecuFamily: string;
  operation: string;
  routeId: string;
  routeValidation: string;
  requestHex: string;
  responder: string | null;
  rawResponseHex: string | null;
  dataHex: string | null;
  parameters: DecodedParameterSummary[];
  dtcs: DtcSummary[];
  negativeResponse: string | null;
  pendingResponses: number;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface ModuleReadClient {
  getState(): Promise<ModuleReadSnapshot>;
  read(request: ModuleReadRequest): Promise<ModuleReadSnapshot>;
  getReportJson(): Promise<string>;
}

export const createModuleReadSnapshot = (): ModuleReadSnapshot => ({
  state: "IDLE",
  ecuFamily: "",
  operation: "",
  routeId: "",
  routeValidation: "",
  requestHex: "",
  responder: null,
  rawResponseHex: null,
  dataHex: null,
  parameters: [],
  dtcs: [],
  negativeResponse: null,
  pendingResponses: 0,
  error: null,
  reportAvailable: false,
});

class TauriModuleReadClient implements ModuleReadClient {
  getState() {
    return invoke<ModuleReadSnapshot>("get_module_read_state");
  }

  read(request: ModuleReadRequest) {
    return invoke<ModuleReadSnapshot>("read_module", { request });
  }

  getReportJson() {
    return invoke<string>("get_module_read_report_json");
  }
}

class BrowserModuleReadClient implements ModuleReadClient {
  private last: ModuleReadSnapshot = createModuleReadSnapshot();

  getState() {
    return Promise.resolve(this.last);
  }

  read(request: ModuleReadRequest): Promise<ModuleReadSnapshot> {
    if (!browserDemoEnabled()) {
      this.last = {
        ...createModuleReadSnapshot(),
        state: "FAILED",
        ecuFamily: request.ecuFamily,
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Reading a module needs the desktop application and a connected adapter.",
          technicalDetails: null,
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.last);
    }
    this.last = demoReadOutcome(request, createModuleReadSnapshot());
    return Promise.resolve(this.last);
  }

  getReportJson() {
    return Promise.resolve(
      JSON.stringify({
        sessionId: "f10-browser-demo",
        executionSource: "SIMULATOR",
        note: "Development browser demo — no adapter or vehicle interaction",
      }),
    );
  }
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultModuleReadClient: ModuleReadClient = hasTauriRuntime()
  ? new TauriModuleReadClient()
  : new BrowserModuleReadClient();

/** Save the module read report where the user chooses; resolves to the path, or null when cancelled. */
export function saveModuleReadReportFile(json: string) {
  const parsed = JSON.parse(json) as { sessionId?: string };
  const session = parsed.sessionId ?? "module-read";
  return saveTextFile(`prowlone-${session}.json`, json);
}
