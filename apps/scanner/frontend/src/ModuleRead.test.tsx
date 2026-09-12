import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
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
  type VehicleCatalogueSnapshot,
  type VinDecodeSnapshot,
  type ModuleSurveyEntry,
  type VehicleDescription,
  type VehicleSurveySnapshot,
} from "./library";
import {
  createModuleReadSnapshot,
  type ModuleReadClient,
  type ModuleReadRequest,
  type ModuleReadSnapshot,
} from "./moduleRead";

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

const surveyed: ModuleSurveyEntry[] = [
  {
    ecuFamily: "BCM",
    name: null,
    names: {},
    applicability: "APPLICABLE",
    logicalNetwork: "PT_HSCAN",
    requestId: "0x726",
    responseId: "0x72E",
    backendRoute: "hs-can",
    pins: "6/14",
    bitrateBps: 500_000,
    protocol: "ISO14229",
    routeValidation: "UNVERIFIED",
    identifierRead: { status: "HYPOTHESIS", reasons: ["the adapter route for this bus is an unverified hypothesis"] },
    dtcRead: { status: "HYPOTHESIS", reasons: ["the adapter route for this bus is an unverified hypothesis"] },
    readableIdentifiers: [{ identifier: "0xF190", parameters: ["VIN"] }],
    selfTests: [
      {
        testId: "7",
        name: "Synthetic door lock cycle",
        timeMs: 9_000,
        timeoutMs: 20_000,
        description: [
          "Synthetic screen text, first line.",
          "Synthetic screen text, second line.",
        ],
        descriptionTexts: {
          ukr: [
            "Синтетичний текст екрана, перший рядок.",
            "Синтетичний текст екрана, другий рядок.",
          ],
        },
        modelYears: ["MY10"],
        safetyClass: "SERVICE_ROUTINE",
      },
    ],
  },
  {
    ecuFamily: "PCM",
    name: null,
    names: {},
    applicability: "APPLICABLE",
    logicalNetwork: "CAN_HS",
    requestId: "0x7E0",
    responseId: "0x7E8",
    backendRoute: "hs-can",
    pins: "6/14",
    bitrateBps: 500_000,
    protocol: "ISO14229",
    routeValidation: "SOURCE_BACKED",
    identifierRead: { status: "REACHABLE", reasons: [] },
    dtcRead: { status: "REACHABLE", reasons: [] },
    readableIdentifiers: [{ identifier: "0x1945", parameters: ["Synthetic module-scoped value"] }],
    selfTests: [],
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
      reachable: 1,
      hypothesis: 1,
      unreachable: 0,
      message: "2 modules known: 1 reachable over the adapter, 1 on a hypothesised route, 0 not — each with the reason.",
    });
  }
}

class ControlledModuleReadClient implements ModuleReadClient {
  requests: ModuleReadRequest[] = [];

  getState(): Promise<ModuleReadSnapshot> {
    return Promise.resolve(createModuleReadSnapshot());
  }

  read(request: ModuleReadRequest): Promise<ModuleReadSnapshot> {
    this.requests.push(request);
    const identifierRead = request.kind === "IDENTIFIER";
    return Promise.resolve({
      ...createModuleReadSnapshot(),
      state: "SUCCEEDED",
      ecuFamily: request.ecuFamily,
      operation: identifierRead ? "Read identifier 0x1945" : "Read confirmed fault codes",
      routeId: "hs-can",
      routeValidation: request.ecuFamily === "BCM" ? "UNVERIFIED" : "SOURCE_BACKED",
      requestHex: identifierRead ? "22 19 45" : "19 02 08",
      responder: request.ecuFamily === "BCM" ? "0x72E" : "0x7E8",
      rawResponseHex: identifierRead ? "62 19 45 1A F8" : "59 02 FF 03 01 00 08",
      dataHex: identifierRead ? "1A F8" : null,
      parameters: identifierRead
        ? [{ name: "Engine speed", raw: 6904, value: "1726", unit: "rpm", state: null, note: null }]
        : [],
      dtcs: identifierRead
        ? []
        : [
            {
              code: "P0301",
              failureType: "00",
              status: "08",
              description: "Cylinder 1 misfire detected",
              descriptionScope: "module",
              failureTypeText: null,
              failureTypeTexts: {},
              descriptionTexts: {},
              help: [],
              helpTexts: {},
              helpNote: null,
            },
          ],
      negativeResponse: null,
      pendingResponses: 0,
      error: null,
      reportAvailable: true,
    });
  }

  getReportJson() {
    return Promise.resolve(JSON.stringify({ sessionId: "f10-test" }));
  }
}

function renderApp(client: ControlledModuleReadClient) {
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

describe("module read", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("marks a hypothesised route on the map and reads its fault codes with wording", async () => {
    const client = new ControlledModuleReadClient();
    renderApp(client);

    // Nothing to read before a survey and a chosen module.
    expect(await screen.findByRole("button", { name: "Read" })).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    const bcm = await screen.findByRole("button", { name: "BCM: Unverified route" });
    expect(screen.getByText("unverified: hs-can · pins 6/14 · 500 kbit/s")).toBeVisible();

    fireEvent.click(bcm);
    // The selection lands after the survey's own effects have settled; on a
    // slow runner that is not the same tick as the click.
    expect(await screen.findByRole("heading", { name: "BCM" })).toBeVisible();
    expect(screen.getByRole("note")).toHaveTextContent("unverified hypothesis");
    const read = screen.getByRole("button", { name: "Read" });
    expect(read).toBeEnabled();
    fireEvent.click(read);

    // The node now says what came back; the details show the wording.
    expect(await screen.findByRole("button", { name: "BCM: 1 fault code" })).toBeVisible();
    expect(client.requests).toHaveLength(1);
    expect(client.requests[0].ecuFamily).toBe("BCM");
    expect(client.requests[0].kind).toBe("FAULT_CODES");
    expect(client.requests[0].context.vehicleProgram).toBe("L405");
    expect(screen.getByText("P0301")).toBeVisible();
    expect(screen.getByText("Cylinder 1 misfire detected")).toBeVisible();
    expect(screen.getByText("BCM on hs-can (UNVERIFIED)")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Save read report" })).toBeEnabled();
  });

  it("lists the self tests a module declares and offers no way to run one", async () => {
    renderApp(new ControlledModuleReadClient());
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "BCM: Unverified route" }));
    await screen.findByRole("heading", { name: "BCM" });

    expect(screen.getByRole("heading", { name: "Self tests this module declares" })).toBeVisible();
    expect(screen.getByText("Synthetic door lock cycle")).toBeVisible();
    // SDD's identifier, the time it states, and the class that keeps it out
    // of this stage, on one line.
    expect(screen.getByText(/test 7 · MY10 · 9 s · SERVICE_ROUTINE/)).toBeVisible();
    expect(screen.getByText("Synthetic screen text, second line.")).toBeVisible();
    expect(screen.getByText(/this application does not/)).toBeVisible();
    // Nothing in the section is a control: the only buttons are the reads.
    const buttons = screen.getAllByRole("button").map((button) => button.textContent);
    expect(buttons.some((label) => label?.includes("Synthetic door lock cycle"))).toBe(false);

    // A module that declares none shows no section at all.
    fireEvent.click(screen.getByRole("button", { name: "PCM: Reachable" }));
    await screen.findByRole("heading", { name: "PCM" });
    expect(
      screen.queryByRole("heading", { name: "Self tests this module declares" }),
    ).toBeNull();
  });

  it("says a self test's words in the interface's language where we have them", async () => {
    renderApp(new ControlledModuleReadClient());
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "BCM: Unverified route" }));
    await screen.findByRole("heading", { name: "BCM" });
    expect(screen.getByText("Synthetic screen text, second line.")).toBeVisible();

    // The interface's own language picker, the way a reader changes it.
    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), {
      target: { value: "uk" },
    });
    // Our own words for the lines we have wording for; SDD's English stays
    // where we have none (ADR-0026).
    expect(
      await screen.findByText("Синтетичний текст екрана, другий рядок."),
    ).toBeVisible();
    expect(
      screen.getByRole("heading", { name: "Самотести, які оголошує цей модуль" }),
    ).toBeVisible();
  });

  it("requires an identifier for an identifier read", async () => {
    const client = new ControlledModuleReadClient();
    renderApp(client);
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "X250" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "PCM: Reachable" }));

    fireEvent.change(screen.getByLabelText("Operation"), { target: { value: "IDENTIFIER" } });
    const read = screen.getByRole("button", { name: "Read" });
    expect(read).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Identifier"), { target: { value: "0x1945" } });
    expect(read).toBeEnabled();
    fireEvent.click(read);
    await screen.findByRole("button", { name: "PCM: Answered" });
    expect(client.requests[0].kind).toBe("IDENTIFIER");
    expect(client.requests[0].identifier).toBe("0x1945");
    // The value is shown decoded, with its unit.
    expect(screen.getByText("1726 rpm")).toBeVisible();
  });
});
