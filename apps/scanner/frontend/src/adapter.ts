import { invoke } from "@tauri-apps/api/core";

export type AdapterState =
  | "NO_ADAPTER"
  | "ADAPTER_DETECTED"
  | "CONNECTING"
  | "CONNECTED"
  | "ERROR";

export type BoardCommunicationState =
  | "UNAVAILABLE"
  | "PENDING"
  | "VERIFIED"
  | "FAILED";

export type AdapterErrorCode =
  | "ADAPTER_NOT_FOUND"
  | "ADAPTER_SELECTION_REQUIRED"
  | "ADAPTER_ALREADY_IN_USE"
  | "UNABLE_TO_OPEN_ADAPTER"
  | "ADAPTER_DISCONNECTED"
  | "BOARD_COMMUNICATION_FAILED"
  | "DISCOVERY_FAILED"
  | "DISCONNECT_FAILED";

export interface AdapterSummary {
  name: string;
  port: string;
  usbVid: number;
  usbPid: number;
  serialNumber: string | null;
  driver: string | null;
}

export interface BoardInfoEvidence {
  responseCommand: string;
  rawResponseHex: string;
}

export interface AdapterInfo extends AdapterSummary {
  connectionStatus: string;
  transport: string;
  backend: string;
  boardInfo: BoardInfoEvidence;
}

export interface VehicleInterfaceCapability {
  id: string;
  name: string;
  pins: string;
  nominalBitrate: number | null;
  hardwareConfirmed: boolean;
  fixtureTested: boolean;
  implementation: "AVAILABLE" | "UNSUPPORTED_BY_ADAPTER";
  vehicleValidation: "NOT_YET_VALIDATED" | "NOT_APPLICABLE";
}

export interface UserFacingError {
  code: AdapterErrorCode;
  message: string;
  technicalDetails: string | null;
}

export interface AdapterSnapshot {
  state: AdapterState;
  adapters: AdapterSummary[];
  selectedAdapterPort: string | null;
  adapter: AdapterInfo | null;
  boardCommunication: BoardCommunicationState;
  capabilities: VehicleInterfaceCapability[];
  selectionRequired: boolean;
  error: UserFacingError | null;
  vehicleMessage: string;
}

export interface AdapterClient {
  getState(): Promise<AdapterSnapshot>;
  discover(): Promise<AdapterSnapshot>;
  connect(port: string | null): Promise<AdapterSnapshot>;
  disconnect(): Promise<AdapterSnapshot>;
}

export const createEmptySnapshot = (): AdapterSnapshot => ({
  state: "NO_ADAPTER",
  adapters: [],
  selectedAdapterPort: null,
  adapter: null,
  boardCommunication: "UNAVAILABLE",
  capabilities: [],
  selectionRequired: false,
  error: null,
  vehicleMessage: "No vehicle connected",
});

class TauriAdapterClient implements AdapterClient {
  getState() {
    return invoke<AdapterSnapshot>("get_adapter_state");
  }

  discover() {
    return invoke<AdapterSnapshot>("discover_adapters");
  }

  connect(port: string | null) {
    return invoke<AdapterSnapshot>("connect_adapter", { port });
  }

  disconnect() {
    return invoke<AdapterSnapshot>("disconnect_adapter");
  }
}

class BrowserAdapterClient implements AdapterClient {
  getState() {
    return Promise.resolve(browserDemoEnabled() ? createDemoSnapshot() : createEmptySnapshot());
  }

  discover() {
    return this.getState();
  }

  connect() {
    return this.getState();
  }

  disconnect() {
    return Promise.resolve(createEmptySnapshot());
  }
}

export function browserDemoEnabled() {
  const localDevelopmentHost =
    typeof window !== "undefined" &&
    ["127.0.0.1", "localhost"].includes(window.location.hostname);
  return (
    localDevelopmentHost &&
    new URLSearchParams(window.location.search).get("demo") === "alpha"
  );
}

function createDemoSnapshot(): AdapterSnapshot {
  return {
    state: "CONNECTED",
    adapters: [
      {
        name: "MongoosePro JLR",
        port: "DEMO",
        usbVid: 0x18e1,
        usbPid: 0x0104,
        serialNumber: null,
        driver: "usbser",
      },
    ],
    selectedAdapterPort: "DEMO",
    adapter: {
      name: "MongoosePro JLR",
      port: "DEMO",
      usbVid: 0x18e1,
      usbPid: 0x0104,
      serialNumber: null,
      driver: "usbser",
      connectionStatus: "Connected",
      transport: "USB CDC / Serial",
      backend: "mongoose-jlr",
      boardInfo: {
        responseCommand: "0x8109",
        rawResponseHex: "DEVELOPMENT DEMO — NO HARDWARE I/O",
      },
    },
    boardCommunication: "VERIFIED",
    capabilities: [],
    selectionRequired: false,
    error: null,
    vehicleMessage: "No vehicle interaction",
  };
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultAdapterClient: AdapterClient = hasTauriRuntime()
  ? new TauriAdapterClient()
  : new BrowserAdapterClient();
