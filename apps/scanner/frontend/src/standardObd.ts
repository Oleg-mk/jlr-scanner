import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";
import type { DtcSummary, ModuleReadState } from "./moduleRead";

/**
 * The legislated OBD-II services (ADR-0022, decision 7): what every OBD-II car
 * answers at the same addresses, JLR or not. Two of the standard's services
 * are absent by design — clearing codes changes the vehicle, and control of
 * on-board systems is not a read.
 */
export type StandardObdReadKind =
  | "CURRENT_DATA"
  | "FREEZE_FRAME"
  | "STORED_DTCS"
  | "PENDING_DTCS"
  | "PERMANENT_DTCS"
  | "MONITOR_RESULTS"
  | "VEHICLE_INFORMATION";

export interface StandardObdRequest {
  kind: StandardObdReadKind;
  /** Which of the standard's eight responders, 0–7; 0 is the engine controller. */
  responder: number;
  /** Hexadecimal bytes such as `0x0C`; empty asks for the support map. */
  items: string[];
  context: VehicleDescription;
}

export interface StandardObdValue {
  pid: string;
  name: string;
  unit: string;
  value: string | null;
  number: number | null;
  rawHex: string;
  /** `number`, `text`, `flag`, `supported` or `raw`. */
  kind: string;
}

export interface StandardObdMonitor {
  mid: string;
  monitor: string;
  tid: string;
  uas: string;
  unit: string;
  value: number | null;
  minimum: number | null;
  maximum: number | null;
  rawValue: number;
  rawMinimum: number;
  rawMaximum: number;
  passed: boolean;
}

export interface LabelledValue {
  label: string;
  value: string;
}

export interface StandardObdSnapshot {
  state: ModuleReadState;
  kind: StandardObdReadKind | null;
  responder: string;
  operation: string;
  routeValidation: string;
  requestHex: string;
  rawResponseHex: string | null;
  supported: string[];
  values: StandardObdValue[];
  dtcKind: string | null;
  dtcs: DtcSummary[];
  freezeFrame: number | null;
  monitors: StandardObdMonitor[];
  information: LabelledValue[];
  negativeResponse: string | null;
  pendingResponses: number;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface StandardObdClient {
  getState(): Promise<StandardObdSnapshot>;
  read(request: StandardObdRequest): Promise<StandardObdSnapshot>;
}

export const createStandardObdSnapshot = (): StandardObdSnapshot => ({
  state: "IDLE",
  kind: null,
  responder: "",
  operation: "",
  routeValidation: "",
  requestHex: "",
  rawResponseHex: null,
  supported: [],
  values: [],
  dtcKind: null,
  dtcs: [],
  freezeFrame: null,
  monitors: [],
  information: [],
  negativeResponse: null,
  pendingResponses: 0,
  error: null,
  reportAvailable: false,
});

/** `0x0C` for 12, as the shell writes them. */
export function hexByte(value: number): string {
  return `0x${value.toString(16).toUpperCase().padStart(2, "0")}`;
}

/** The support items of a mode: 0x00, 0x20, 0x40 … */
export function isSupportItem(item: string): boolean {
  const value = Number.parseInt(item, 16);
  return Number.isInteger(value) && value % 0x20 === 0;
}

class TauriStandardObdClient implements StandardObdClient {
  getState() {
    return invoke<StandardObdSnapshot>("get_standard_obd_state");
  }

  read(request: StandardObdRequest) {
    return invoke<StandardObdSnapshot>("read_standard_obd", { request });
  }
}

/** A standing, warm engine for the browser preview; nothing behind it. */
const DEMO_VALUES: Record<string, StandardObdValue> = {
  "0x04": { pid: "0x04", name: "calculated engine load", unit: "%", value: "25.1", number: 25.1, rawHex: "40", kind: "number" },
  "0x05": { pid: "0x05", name: "engine coolant temperature", unit: "°C", value: "83", number: 83, rawHex: "7B", kind: "number" },
  "0x0B": { pid: "0x0B", name: "intake manifold absolute pressure", unit: "kPa", value: "33", number: 33, rawHex: "21", kind: "number" },
  "0x0C": { pid: "0x0C", name: "engine speed", unit: "rpm", value: "750", number: 750, rawHex: "0B B8", kind: "number" },
  "0x0D": { pid: "0x0D", name: "vehicle speed", unit: "km/h", value: "0", number: 0, rawHex: "00", kind: "number" },
  "0x0F": { pid: "0x0F", name: "intake air temperature", unit: "°C", value: "20", number: 20, rawHex: "3C", kind: "number" },
  "0x11": { pid: "0x11", name: "throttle position", unit: "%", value: "16.5", number: 16.5, rawHex: "2A", kind: "number" },
  "0x42": { pid: "0x42", name: "control module voltage", unit: "V", value: "14", number: 14, rawHex: "36 B0", kind: "number" },
};

class BrowserStandardObdClient implements StandardObdClient {
  private last = createStandardObdSnapshot();

  getState() {
    return Promise.resolve(this.last);
  }

  read(request: StandardObdRequest) {
    const base = {
      ...createStandardObdSnapshot(),
      kind: request.kind,
      responder: "0x7E0 → 0x7E8",
      requestHex: "01 00",
      reportAvailable: true,
    };
    if (!browserDemoEnabled()) {
      this.last = {
        ...base,
        state: "FAILED",
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails: "connect and verify the adapter before a standard OBD-II read",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.last);
    }
    const snapshot: StandardObdSnapshot = { ...base, state: "SUCCEEDED", routeValidation: "DEMO" };
    switch (request.kind) {
      case "CURRENT_DATA": {
        const items = request.items.length > 0 ? request.items : ["0x00"];
        for (const item of items) {
          if (item === "0x00") {
            snapshot.supported = Object.keys(DEMO_VALUES);
            snapshot.values.push({ pid: "0x00", name: "PIDs supported, 01–20", unit: "", value: snapshot.supported.join(", "), number: null, rawHex: "18 3B 80 00", kind: "supported" });
          } else if (DEMO_VALUES[item]) {
            snapshot.values.push(DEMO_VALUES[item]);
          }
        }
        break;
      }
      case "STORED_DTCS":
        snapshot.dtcKind = "stored";
        snapshot.dtcs = [
          {
            code: "P0300",
            failureType: "",
            status: "",
            description: "Random/multiple cylinder misfire detected",
            descriptionScope: "generic",
            failureTypeText: null,
            failureTypeTexts: {},
            descriptionTexts: {},
            help: ["Possible causes:", "Browser preview: the loaded data would say them here."],
            helpTexts: {},
            helpNote: null,
          },
        ];
        break;
      case "PENDING_DTCS":
        snapshot.dtcKind = "pending";
        break;
      case "PERMANENT_DTCS":
        snapshot.dtcKind = "permanent";
        break;
      case "VEHICLE_INFORMATION": {
        const [type] = request.items;
        if (type === undefined || type === "0x00") snapshot.supported = ["0x02", "0x04", "0x0A"];
        else if (type === "0x02") snapshot.information = [{ label: "VIN", value: "SAJDEMO0000000001" }];
        else if (type === "0x04") snapshot.information = [{ label: "Calibration ID", value: "DEMO-CALID-01" }];
        else if (type === "0x0A") snapshot.information = [{ label: "ECU name", value: "ECM -EngineControl" }];
        break;
      }
      case "MONITOR_RESULTS": {
        const [mid] = request.items;
        if (mid === undefined || mid === "0x00") snapshot.supported = ["0x01", "0x21"];
        else
          snapshot.monitors = request.items.map((item) => ({
            mid: item,
            monitor: item === "0x01" ? "oxygen sensor monitor, bank 1 sensor 1" : "catalyst monitor, bank 1",
            tid: "0x80",
            uas: "0x0B",
            unit: "V",
            value: item === "0x01" ? 0.5 : 0.7,
            minimum: 0,
            maximum: 1,
            rawValue: item === "0x01" ? 500 : 700,
            rawMinimum: 0,
            rawMaximum: 1000,
            passed: true,
          }));
        break;
      }
      case "FREEZE_FRAME": {
        snapshot.freezeFrame = 0;
        const [pid] = request.items;
        if (pid === undefined || pid === "0x00") snapshot.supported = ["0x02", "0x05", "0x0C"];
        else if (pid === "0x02")
          snapshot.values = [
            { pid: "0x02", name: "fault code that froze the frame", unit: "", value: "P0300", number: null, rawHex: "03 00", kind: "text" },
          ];
        else if (DEMO_VALUES[pid]) snapshot.values = [DEMO_VALUES[pid]];
        else snapshot.negativeResponse = "0x31 request out of range";
        break;
      }
    }
    this.last = snapshot;
    return Promise.resolve(snapshot);
  }
}

export const defaultStandardObdClient: StandardObdClient = hasTauriRuntime()
  ? new TauriStandardObdClient()
  : new BrowserStandardObdClient();
