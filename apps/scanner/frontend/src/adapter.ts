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
  | "DISCONNECT_FAILED"
  | "SESSION_MODE_MISMATCH";

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
  /**
   * The bench scenario in force (ADR-0020): 0 is the healthy vehicle, any
   * other number a picture of faults that number always repeats. Null unless
   * the bench is what is connected.
   */
  benchScenario?: number | null;
}

export interface AdapterClient {
  getState(): Promise<AdapterSnapshot>;
  discover(): Promise<AdapterSnapshot>;
  connect(port: string | null): Promise<AdapterSnapshot>;
  disconnect(): Promise<AdapterSnapshot>;
  /** The bench (ADR-0020): a virtual vehicle behind a stand-in adapter, no port. */
  connectBench(scenario: number): Promise<AdapterSnapshot>;
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
  benchScenario: null,
});

/** The transport name the shell gives the bench (ADR-0020). */
export const BENCH_TRANSPORT = "bench";

/** The scenario on which no module reports a fault: the healthy vehicle. */
export const BENCH_SCENARIO_HEALTHY = 0;
/** The scenario the bench starts on when nobody chose another. */
export const BENCH_SCENARIO_DEFAULT = 1;

/** Whether the connection is the bench rather than an adapter. */
export function isBench(snapshot: AdapterSnapshot) {
  return snapshot.state === "CONNECTED" && snapshot.adapter?.transport === BENCH_TRANSPORT;
}

/** The bench as the shell reports it; the browser preview shows the same shape without hardware. */
export const createBenchSnapshot = (
  scenario: number = BENCH_SCENARIO_DEFAULT,
): AdapterSnapshot => ({
  ...createEmptySnapshot(),
  state: "CONNECTED",
  selectedAdapterPort: "bench",
  adapter: {
    name: "Virtual vehicle (bench)",
    port: "bench",
    usbVid: 0,
    usbPid: 0,
    serialNumber: null,
    driver: null,
    connectionStatus: "Connected",
    transport: BENCH_TRANSPORT,
    backend: "bench-vehicle",
    boardInfo: { responseCommand: "0x8109", rawResponseHex: "BENCH — NO HARDWARE I/O" },
  },
  boardCommunication: "VERIFIED",
  vehicleMessage: "Virtual vehicle on the bench",
  benchScenario: scenario,
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

  connectBench(scenario: number) {
    return invoke<AdapterSnapshot>("connect_bench", { scenario });
  }
}

class BrowserAdapterClient implements AdapterClient {
  private bench = false;
  private scenario = BENCH_SCENARIO_DEFAULT;

  getState() {
    if (this.bench) return Promise.resolve(createBenchSnapshot(this.scenario));
    return Promise.resolve(browserDemoEnabled() ? createDemoSnapshot() : createEmptySnapshot());
  }

  discover() {
    return this.getState();
  }

  connect() {
    this.bench = false;
    return this.getState();
  }

  connectBench(scenario: number) {
    this.bench = true;
    this.scenario = scenario;
    return this.getState();
  }

  disconnect() {
    this.bench = false;
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
