import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import { createEmptySnapshot, type AdapterClient, type AdapterSnapshot } from "./adapter";
import { createCaptureSnapshot, type CaptureClient, type CaptureSnapshot } from "./capture";
import {
  createDiagnosticSnapshot,
  type DiagnosticClient,
  type DiagnosticSnapshot,
} from "./diagnostic";
import {
  createLibrarySnapshot,
  type LibraryClient,
  type LibrarySnapshot,
  type VehicleCatalogueSnapshot,
  type VinDecodeSnapshot,
  type VehicleDescription,
  type VehicleSurveySnapshot,
} from "./library";

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

class IdleLibraryClient implements LibraryClient {
  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve(createLibrarySnapshot());
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
      modules: [],
      reachable: 0,
      hypothesis: 0,
      unreachable: 0,
      message: "",
    });
  }
}

class ControlledCaptureClient implements CaptureClient {
  requests: Array<{ routeId: string; seconds: number; vehicle: VehicleDescription }> = [];
  jsonRequests = 0;

  getState(): Promise<CaptureSnapshot> {
    return Promise.resolve(createCaptureSnapshot());
  }

  listen(routeId: string, seconds: number, vehicle: VehicleDescription) {
    this.requests.push({ routeId, seconds, vehicle });
    return Promise.resolve<CaptureSnapshot>({
      ...createCaptureSnapshot(),
      state: "COMPLETED",
      routeId,
      pins: routeId === "hs-can" ? "6/14" : "3/11",
      bitrateBps: routeId === "hs-can" ? 500_000 : 125_000,
      requestedSeconds: seconds,
      listenedMs: seconds * 1000,
      frames: 42,
      framesPerSecond: Math.round(42 / seconds),
      distinctIds: 2,
      standardFrames: 41,
      extendedFrames: 1,
      droppedFrames: 0,
      truncated: false,
      topIds: [
        { id: "0x321", extended: false, count: 41 },
        { id: "0x18DAF110", extended: true, count: 1 },
      ],
      verdict: `Traffic present on ${routeId}. A live bus is on this pair.`,
      error: null,
      captureAvailable: true,
      synthetic: false,
    });
  }

  getCaptureJson() {
    this.jsonRequests += 1;
    return Promise.resolve(JSON.stringify({ name: "capture-test", frames: [] }));
  }
}

function renderApp(adapter: AdapterSnapshot, captureClient: CaptureClient) {
  return render(
    <App
      client={new FixedAdapterClient(adapter)}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={new IdleLibraryClient()}
      captureClient={captureClient}
      pollIntervalMs={60_000}
    />,
  );
}

describe("listen-only bus capture", () => {
  it("cannot listen without a verified adapter", async () => {
    renderApp(createEmptySnapshot(), new ControlledCaptureClient());
    expect(await screen.findByRole("heading", { name: "Bus capture" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Listen" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Save capture" })).toBeDisabled();
  });

  it("listens on the chosen pair for the chosen time and offers the capture to save", async () => {
    const client = new ControlledCaptureClient();
    renderApp(connectedSnapshot(), client);

    const listen = await screen.findByRole("button", { name: "Listen" });
    expect(listen).toBeEnabled();
    fireEvent.change(screen.getByLabelText("Pair"), { target: { value: "ms-can" } });
    fireEvent.change(screen.getByLabelText("Seconds (1–15)"), { target: { value: "3" } });
    fireEvent.click(listen);

    expect(await screen.findByText(/Traffic present on ms-can \(pins 3\/11\) at 125 kbit\/s/)).toBeVisible();
    expect(client.requests).toHaveLength(1);
    expect(client.requests[0].routeId).toBe("ms-can");
    expect(client.requests[0].seconds).toBe(3);
    expect(screen.getByText("Traffic heard")).toBeVisible();
    expect(screen.getByText("0x18DAF110")).toBeVisible();
    expect(screen.getByText("29-bit")).toBeVisible();

    const save = screen.getByRole("button", { name: "Save capture" });
    expect(save).toBeEnabled();
  });
});
