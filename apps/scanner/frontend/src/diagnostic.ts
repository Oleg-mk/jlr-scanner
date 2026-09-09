import { invoke } from "@tauri-apps/api/core";
import { saveTextFile } from "./files";
import { browserDemoEnabled } from "./adapter";

export interface SupportedVehicleProfile {
  make: string;
  model: string;
  vehicleProgram: string;
  modelYear: number;
  powertrain: string;
  module: string;
  ecuTarget: string;
}

export type DiagnosticState =
  | "UNAVAILABLE"
  | "READY"
  | "RUNNING"
  | "SUCCEEDED"
  | "FAILED";

export type DiagnosticErrorCategory =
  | "ADAPTER_NOT_FOUND"
  | "ADAPTER_COMMUNICATION_FAILED"
  | "UNSUPPORTED_VEHICLE_PROFILE"
  | "CAN_CONNECTION_FAILED"
  | "NO_RESPONSE_FROM_ECU"
  | "UNEXPECTED_RESPONDER"
  | "MALFORMED_DIAGNOSTIC_RESPONSE"
  | "INTERNAL_FAILURE";

export interface DiagnosticError {
  category: DiagnosticErrorCategory;
  message: string;
  technicalDetails: string | null;
  stage: string;
}

export interface DiagnosticSnapshot {
  profile: SupportedVehicleProfile;
  operationName: string;
  state: DiagnosticState;
  calibrationId: string | null;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface DiagnosticClient {
  getState(): Promise<DiagnosticSnapshot>;
  readCalibrationIdentification(): Promise<DiagnosticSnapshot>;
  getReportJson(): Promise<string>;
}

export const supportedProfile: SupportedVehicleProfile = {
  make: "Jaguar",
  model: "XF",
  vehicleProgram: "X250",
  modelYear: 2010,
  powertrain: "5.0L Supercharged",
  module: "ECM / PCM",
  ecuTarget: "jlr.x250.ecm",
};

export const createDiagnosticSnapshot = (): DiagnosticSnapshot => ({
  profile: supportedProfile,
  operationName: "Read Calibration Identification",
  state: "UNAVAILABLE",
  calibrationId: null,
  error: null,
  reportAvailable: false,
});

class TauriDiagnosticClient implements DiagnosticClient {
  getState() {
    return invoke<DiagnosticSnapshot>("get_diagnostic_state");
  }

  readCalibrationIdentification() {
    return invoke<DiagnosticSnapshot>("read_calibration_identification");
  }

  getReportJson() {
    return invoke<string>("get_diagnostic_report_json");
  }
}

class BrowserDiagnosticClient implements DiagnosticClient {
  private completed = false;

  getState(): Promise<DiagnosticSnapshot> {
    const state: DiagnosticState = browserDemoEnabled()
      ? this.completed
        ? "SUCCEEDED"
        : "READY"
      : "UNAVAILABLE";
    return Promise.resolve({
      ...createDiagnosticSnapshot(),
      state,
      calibrationId: this.completed ? "CX23-14C204-ZAD" : null,
      reportAvailable: this.completed,
    });
  }

  async readCalibrationIdentification() {
    this.completed = true;
    return this.getState();
  }

  getReportJson() {
    return Promise.resolve(
      JSON.stringify(
        {
          schemaVersion: 1,
          sessionId: "f8-browser-demo",
          executionSource: "SIMULATOR",
          requestPayload: "09 04",
          actualResponder: "0x7E8",
          decodedResult: "Calibration ID: CX23-14C204-ZAD",
          note: "Development browser demo — no hardware or vehicle interaction",
        },
        null,
        2,
      ),
    );
  }
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultDiagnosticClient: DiagnosticClient = hasTauriRuntime()
  ? new TauriDiagnosticClient()
  : new BrowserDiagnosticClient();

/** Save the diagnostic report where the user chooses; resolves to the path, or null when cancelled. */
export function saveDiagnosticReportFile(json: string) {
  const parsed = JSON.parse(json) as { sessionId?: string };
  const session = parsed.sessionId ?? "diagnostic-report";
  return saveTextFile(`prowlone-${session}.json`, json);
}
