import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { createEmptySnapshot, type AdapterClient, type AdapterSnapshot } from "./adapter";
import {
  createDiagnosticSnapshot,
  type DiagnosticClient,
  type DiagnosticSnapshot,
} from "./diagnostic";
import {
  createLibrarySnapshot,
  type LibraryClient,
  type LibrarySnapshot,
  type ModuleSurveyEntry,
  type VehicleCatalogueSnapshot,
  type VinDecodeSnapshot,
  type VehicleDescription,
  type VehicleSurveySnapshot,
} from "./library";
import {
  createModuleReadSnapshot,
  type ModuleReadClient,
  type ModuleReadRequest,
  type ModuleReadSnapshot,
} from "./moduleRead";
import { checkableModules, lanes, nodeStatus } from "./networkMap";

const connectedSnapshot = (): AdapterSnapshot => ({
  ...createEmptySnapshot(),
  state: "CONNECTED",
  adapters: [
    {
      name: "MongoosePro JLR",
      port: "COM7",
      usbVid: 0x18e1,
      usbPid: 0x0104,
      serialNumber: "SERIAL-COM7",
      driver: "usbser",
    },
  ],
  selectedAdapterPort: "COM7",
  adapter: {
    name: "MongoosePro JLR",
    port: "COM7",
    usbVid: 0x18e1,
    usbPid: 0x0104,
    serialNumber: "SERIAL-COM7",
    driver: "usbser",
    connectionStatus: "Connected",
    transport: "USB CDC / Serial",
    backend: "mongoose-jlr",
    boardInfo: { responseCommand: "0x8109", rawResponseHex: "AA BB CC" },
  },
  boardCommunication: "VERIFIED",
});

class FixedAdapterClient implements AdapterClient {
  constructor(private readonly fixed: AdapterSnapshot) {}
  getState() {
    return Promise.resolve(this.fixed);
  }
  discover() {
    return Promise.resolve(this.fixed);
  }
  connect() {
    return Promise.resolve(this.fixed);
  }
  connectBench() {
    return this.getState();
  }

  disconnect() {
    return Promise.resolve(this.fixed);
  }
}

class IdleDiagnosticClient implements DiagnosticClient {
  getState(): Promise<DiagnosticSnapshot> {
    return Promise.resolve({ ...createDiagnosticSnapshot(), state: "READY" });
  }
  readCalibrationIdentification(): Promise<DiagnosticSnapshot> {
    return Promise.resolve(createDiagnosticSnapshot());
  }
  getReportJson(): Promise<string> {
    return Promise.resolve("{}");
  }
}

const documented = (ecuFamily: string, bus: string): ModuleSurveyEntry => ({
  ecuFamily,
  name: null,
  names: {},
  applicability: "APPLICABLE",
  logicalNetwork: bus,
  requestId: "0x7E0",
  responseId: "0x7E8",
  backendRoute: "hs-can",
  pins: "6/14",
  bitrateBps: 500_000,
  protocol: "ISO14229",
  routeValidation: "SOURCE_BACKED",
  identifierRead: { status: "REACHABLE", reasons: [] },
  dtcRead: { status: "REACHABLE", reasons: [] },
  readableIdentifiers: [],
  selfTests: [],
});

const surveyed: ModuleSurveyEntry[] = [
  {
    ...documented("BCM", "PT_HSCAN"),
    routeValidation: "UNVERIFIED",
    identifierRead: { status: "HYPOTHESIS", reasons: ["route is an unverified hypothesis"] },
    dtcRead: { status: "HYPOTHESIS", reasons: ["route is an unverified hypothesis"] },
  },
  documented("PCM", "CAN_HS"),
  documented("ABS", "CAN_HS"),
  {
    ...documented("TCM", "SUB_MOST"),
    backendRoute: null,
    pins: null,
    bitrateBps: null,
    identifierRead: { status: "INDETERMINATE", reasons: ["behind a gateway; no route"] },
    dtcRead: { status: "INDETERMINATE", reasons: ["behind a gateway; no route"] },
  },
];

class SurveyingLibraryClient implements LibraryClient {
  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve({ ...createLibrarySnapshot(), state: "LOADED" });
  }
  loadDirectory(): Promise<LibrarySnapshot> {
    return Promise.resolve(createLibrarySnapshot());
  }
  getCatalogue(): Promise<VehicleCatalogueSnapshot> {
    return Promise.resolve({ programmes: [] });
  }
  decodeVin(vin: string): Promise<VinDecodeSnapshot> {
    return Promise.resolve({
      vin,
      valid: vin.length === 17,
      message: "No VIN tables in this test.",
      decodeModel: null,
      candidates: [],
      program: null,
      modelYear: null,
      attributes: [],
      tableVersion: null,
    });
  }
  surveyVehicle(vehicle: VehicleDescription): Promise<VehicleSurveySnapshot> {
    return Promise.resolve({
      context: vehicle,
      modules: surveyed,
      reachable: 2,
      hypothesis: 1,
      unreachable: 1,
      message: "4 modules known: 2 reachable, 1 on a hypothesised route, 1 not.",
    });
  }
}

class ScriptedModuleReadClient implements ModuleReadClient {
  requests: ModuleReadRequest[] = [];

  getState(): Promise<ModuleReadSnapshot> {
    return Promise.resolve(createModuleReadSnapshot());
  }

  read(request: ModuleReadRequest): Promise<ModuleReadSnapshot> {
    this.requests.push(request);
    const base = {
      ...createModuleReadSnapshot(),
      ecuFamily: request.ecuFamily,
      operation: "Read confirmed fault codes",
      routeId: "hs-can",
      requestHex: "19 02 08",
    };
    if (request.ecuFamily === "BCM") {
      return Promise.resolve({
        ...base,
        state: "FAILED",
        error: {
          category: "NO_RESPONSE_FROM_ECU",
          message: "No answer from BCM",
          technicalDetails: null,
          stage: "TRANSPORT",
        },
      });
    }
    return Promise.resolve({
      ...base,
      state: "SUCCEEDED",
      responder: "0x7E8",
      rawResponseHex: "59 02 FF",
      reportAvailable: true,
    });
  }

  getReportJson() {
    return Promise.resolve("{}");
  }
}

function renderApp(client: ScriptedModuleReadClient) {
  return render(
    <App
      client={new FixedAdapterClient(connectedSnapshot())}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={new SurveyingLibraryClient()}
      moduleReadClient={client}
      pollIntervalMs={60_000}
    />,
  );
}

describe("network check", () => {
  it("reads every checkable module in lane order and marks what answered", async () => {
    const client = new ScriptedModuleReadClient();
    renderApp(client);
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    await screen.findByRole("button", { name: "TCM: Not reachable" });

    const checkAll = screen.getByRole("button", { name: "Check all modules (3)" });
    expect(checkAll).toBeEnabled();
    fireEvent.click(checkAll);

    expect(await screen.findByRole("button", { name: "BCM: No answer" })).toBeVisible();
    await waitFor(() => expect(client.requests).toHaveLength(3));
    // Documented lanes first, then the hypothesis; the unreachable module is never asked.
    expect(client.requests.map((request) => request.ecuFamily)).toEqual(["ABS", "PCM", "BCM"]);
    expect(client.requests.every((request) => request.kind === "FAULT_CODES")).toBe(true);
    expect(screen.getByRole("button", { name: "PCM: Answered" })).toBeVisible();
    expect(screen.getByRole("button", { name: "ABS: Answered" })).toBeVisible();
    expect(screen.getByRole("button", { name: "TCM: Not reachable" })).toBeVisible();

    // The map explains the silence, in words, when the node is chosen.
    fireEvent.click(screen.getByRole("button", { name: "BCM: No answer" }));
    expect(screen.getByText(/Silence does not confirm it/)).toBeVisible();

    fireEvent.click(screen.getByLabelText("Also try unverified routes"));
    expect(screen.getByRole("button", { name: "Check all modules (2)" })).toBeEnabled();
  });

  it("groups modules into lanes by bus, most useful first, and states each node", () => {
    const grouped = lanes(surveyed);
    expect(grouped.map((lane) => `${lane.name}:${lane.kind}`)).toEqual([
      "CAN_HS:documented",
      "PT_HSCAN:hypothesis",
      "SUB_MOST:unbound",
    ]);
    expect(grouped[0].route).toBe("hs-can · pins 6/14 · 500 kbit/s");
    expect(grouped[1].route).toBe("unverified: hs-can · pins 6/14 · 500 kbit/s");
    expect(grouped[2].route).toBe("not bound: behind a gateway; no route");
    expect(checkableModules(surveyed, false).map((module) => module.ecuFamily)).toEqual([
      "PCM",
      "ABS",
    ]);

    expect(nodeStatus(surveyed[3], undefined, false)).toMatchObject({
      state: "unreachable",
      label: "Not reachable",
      detail: "behind a gateway; no route",
    });
    expect(
      nodeStatus(
        surveyed[1],
        {
          ...createModuleReadSnapshot(),
          state: "SUCCEEDED",
          ecuFamily: "PCM",
          negativeResponse: "serviceNotSupported (0x11)",
        },
        false,
      ),
    ).toMatchObject({ state: "declined", label: "Declined" });
  });
});
