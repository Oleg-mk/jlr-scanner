import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "./App";
import {
  createEmptySnapshot,
  type AdapterClient,
  type AdapterSnapshot,
  type AdapterSummary,
  type VehicleInterfaceCapability,
} from "./adapter";
import {
  createDiagnosticSnapshot,
  type DiagnosticClient,
  type DiagnosticSnapshot,
} from "./diagnostic";

const candidate = (port = "COM7"): AdapterSummary => ({
  name: "MongoosePro JLR",
  port,
  usbVid: 0x18e1,
  usbPid: 0x0104,
  serialNumber: `SERIAL-${port}`,
  driver: "usbser",
});

const capabilities: VehicleInterfaceCapability[] = [
  {
    id: "hs-can",
    name: "HS-CAN",
    pins: "6/14",
    nominalBitrate: 500_000,
    hardwareConfirmed: true,
    fixtureTested: true,
    implementation: "AVAILABLE",
    vehicleValidation: "NOT_YET_VALIDATED",
  },
  {
    id: "ms-can",
    name: "MS-CAN",
    pins: "3/11",
    nominalBitrate: 125_000,
    hardwareConfirmed: true,
    fixtureTested: true,
    implementation: "AVAILABLE",
    vehicleValidation: "NOT_YET_VALIDATED",
  },
  {
    id: "ccp-hs-can",
    name: "CCP HS-CAN",
    pins: "12/13 on X250",
    nominalBitrate: null,
    hardwareConfirmed: false,
    fixtureTested: false,
    implementation: "UNSUPPORTED_BY_ADAPTER",
    vehicleValidation: "NOT_APPLICABLE",
  },
];

const detectedSnapshot = (
  adapters = [candidate()],
): AdapterSnapshot => ({
  ...createEmptySnapshot(),
  state: "ADAPTER_DETECTED",
  adapters,
  selectedAdapterPort: adapters.length === 1 ? adapters[0].port : null,
  selectionRequired: adapters.length > 1,
  capabilities,
});

const connectedSnapshot = (port = "COM7"): AdapterSnapshot => ({
  ...detectedSnapshot([candidate(port)]),
  state: "CONNECTED",
  selectionRequired: false,
  adapter: {
    ...candidate(port),
    connectionStatus: "Connected",
    transport: "USB CDC / Serial",
    backend: "mongoose-jlr",
    boardInfo: {
      responseCommand: "0x8109",
      rawResponseHex: "AA BB CC",
    },
  },
  boardCommunication: "VERIFIED",
});

const disconnectedErrorSnapshot = (): AdapterSnapshot => ({
  ...createEmptySnapshot(),
  state: "ERROR",
  error: {
    code: "ADAPTER_DISCONNECTED",
    message: "Adapter disconnected",
    technicalDetails: "connected adapter is no longer present",
  },
});

class ControlledClient implements AdapterClient {
  discoveries: AdapterSnapshot[];
  connectResult: AdapterSnapshot;
  disconnectResult: AdapterSnapshot;
  connectedPort: string | null = null;
  lastDiscovery: AdapterSnapshot = createEmptySnapshot();

  constructor({
    discoveries,
    connectResult = connectedSnapshot(),
    disconnectResult = detectedSnapshot(),
  }: {
    discoveries: AdapterSnapshot[];
    connectResult?: AdapterSnapshot;
    disconnectResult?: AdapterSnapshot;
  }) {
    this.discoveries = [...discoveries];
    this.connectResult = connectResult;
    this.disconnectResult = disconnectResult;
  }

  getState() {
    return Promise.resolve(this.discoveries[0] ?? createEmptySnapshot());
  }

  discover() {
    const next = this.discoveries.shift();
    if (next) {
      this.lastDiscovery = next;
    }
    return Promise.resolve(this.lastDiscovery);
  }

  connect(port: string | null) {
    this.connectedPort = port;
    return Promise.resolve(this.connectResult);
  }

  disconnect() {
    return Promise.resolve(this.disconnectResult);
  }
}

class ControlledDiagnosticClient implements DiagnosticClient {
  snapshot: DiagnosticSnapshot = {
    ...createDiagnosticSnapshot(),
    state: "READY",
  };

  getState() {
    return Promise.resolve(this.snapshot);
  }

  readCalibrationIdentification() {
    this.snapshot = {
      ...this.snapshot,
      state: "SUCCEEDED",
      calibrationId: "CX23-14C204-ZAD",
      reportAvailable: true,
    };
    return Promise.resolve(this.snapshot);
  }

  getReportJson() {
    return Promise.resolve(JSON.stringify({ sessionId: "f8-test" }));
  }
}

describe("JLR Scanner application shell", () => {
  it("renders the real no-adapter state without fake vehicle data", async () => {
    const client = new ControlledClient({
      discoveries: [createEmptySnapshot()],
    });
    render(
      <App
        client={client}
        diagnosticClient={new ControlledDiagnosticClient()}
        pollIntervalMs={60_000}
      />,
    );

    expect(
      await screen.findByRole("heading", { name: "Adapter not detected" }),
    ).toBeVisible();
    expect(
      screen.getByRole("button", { name: "Detect adapter" }),
    ).toBeEnabled();
    expect(screen.getByText(/Jaguar XF \/ X250, 2010, 5\.0L Supercharged/)).toBeVisible();
    expect(screen.getByRole("button", { name: "Read Calibration ID" })).toBeDisabled();
    expect(
      screen.queryByRole("heading", { name: "Supported vehicle interfaces" }),
    ).not.toBeInTheDocument();
  });

  it("connects through the client and renders verified board communication", async () => {
    const client = new ControlledClient({
      discoveries: [detectedSnapshot()],
      connectResult: connectedSnapshot(),
    });
    const diagnosticClient = new ControlledDiagnosticClient();
    render(
      <App
        client={client}
        diagnosticClient={diagnosticClient}
        pollIntervalMs={60_000}
      />,
    );

    const connect = await screen.findByRole("button", { name: "Connect" });
    expect(screen.getAllByText("COM7")).toHaveLength(2);
    fireEvent.click(connect);

    expect(await screen.findByText("Verified")).toBeVisible();
    expect(screen.getByText("0x8109")).toBeVisible();
    const read = screen.getByRole("button", { name: "Read Calibration ID" });
    expect(read).toBeEnabled();
    fireEvent.click(read);
    expect(await screen.findByText("Calibration ID: CX23-14C204-ZAD")).toBeVisible();
    expect(screen.getByRole("button", { name: "Save Diagnostic Report" })).toBeEnabled();
  });

  it("requires an explicit selection when multiple adapters are present", async () => {
    const client = new ControlledClient({
      discoveries: [
        detectedSnapshot([candidate("COM7"), candidate("COM8")]),
      ],
      connectResult: connectedSnapshot("COM8"),
    });
    render(
      <App
        client={client}
        diagnosticClient={new ControlledDiagnosticClient()}
        pollIntervalMs={60_000}
      />,
    );

    const select = await screen.findByRole("combobox", {
      name: "Choose adapter",
    });
    const connect = screen.getByRole("button", { name: "Connect" });
    expect(connect).toBeDisabled();

    fireEvent.change(select, { target: { value: "COM8" } });
    expect(connect).toBeEnabled();
    fireEvent.click(connect);

    await screen.findByText("Verified");
    expect(client.connectedPort).toBe("COM8");
  });

  it("shows a normal board communication error and technical details", async () => {
    const failed: AdapterSnapshot = {
      ...detectedSnapshot(),
      state: "ERROR",
      boardCommunication: "FAILED",
      error: {
        code: "BOARD_COMMUNICATION_FAILED",
        message: "Board communication failed",
        technicalDetails: "unexpected response",
      },
    };
    const client = new ControlledClient({
      discoveries: [detectedSnapshot()],
      connectResult: failed,
    });
    render(
      <App
        client={client}
        diagnosticClient={new ControlledDiagnosticClient()}
        pollIntervalMs={60_000}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Connect" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Board communication failed",
    );
    expect(screen.queryByText("Rust panic")).not.toBeInTheDocument();
  });

  it("disconnects cleanly back to the detected state", async () => {
    const client = new ControlledClient({
      discoveries: [connectedSnapshot()],
      disconnectResult: detectedSnapshot(),
    });
    render(
      <App
        client={client}
        diagnosticClient={new ControlledDiagnosticClient()}
        pollIntervalMs={60_000}
      />,
    );

    fireEvent.click(
      await screen.findByRole("button", { name: "Disconnect" }),
    );

    expect(await screen.findByRole("button", { name: "Connect" })).toBeVisible();
    expect(screen.queryByText("Verified")).not.toBeInTheDocument();
  });

  it("survives hot unplug and detects the replugged COM port", async () => {
    const client = new ControlledClient({
      discoveries: [
        connectedSnapshot(),
        disconnectedErrorSnapshot(),
      ],
    });
    render(
      <App
        client={client}
        diagnosticClient={new ControlledDiagnosticClient()}
        pollIntervalMs={180}
      />,
    );

    expect(await screen.findByText("Verified")).toBeVisible();
    expect(
      await screen.findAllByText("Adapter disconnected", {}, { timeout: 1_500 }),
    ).toHaveLength(2);
    client.discoveries.push(detectedSnapshot([candidate("COM9")]));
    await waitFor(
      () => expect(screen.getAllByText("COM9")).toHaveLength(2),
      { timeout: 1_500 },
    );
  });
});
