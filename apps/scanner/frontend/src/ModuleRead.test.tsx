import { fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { resetHelpLanguage } from "./helpLanguage";
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
          rus: [
            "Синтетический текст экрана, первая строка.",
            "Синтетический текст экрана, вторая строка.",
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
    acceptedOperations: [
      {
        kind: "ROUTINE",
        identifier: "0x0406",
        name: "Synthetic clear adaption values",
        service: "0x31",
        sessions: ["03"],
        security: "level_1",
        maxRunTime: "30",
        restartWhileRunning: "no",
        safetyClass: "SERVICE_ROUTINE",
      },
      {
        kind: "WRITE",
        identifier: "0x1259",
        name: "Synthetic engine input",
        service: "0x2E",
        sessions: ["03"],
        security: null,
        maxRunTime: null,
        restartWhileRunning: null,
        safetyClass: "PERSISTENT_CHANGE",
      },
    ],
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
              descriptionDataTexts: {},
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
    // The code is on screen twice now, on purpose: in the list of what the
    // check found and in the module's own panel (2026-09-18).
    expect(screen.getAllByText("P0301").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Cylinder 1 misfire detected").length).toBeGreaterThan(0);
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

    const selfTests = screen.getByText("Self tests this module declares");
    expect(selfTests).toBeVisible();
    fireEvent.click(selfTests);
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
      screen.queryByText("Self tests this module declares"),
    ).toBeNull();
  });

  /**
   * The other half of the module index (`ADR-0035`). The rule is the one
   * `ADR-0032` set for the self tests: everything SDD declares is shown, with
   * what it would cost, and nothing on the screen can set it going.
   */
  it("lists what a module will accept and offers no way to send any of it", async () => {
    renderApp(new ControlledModuleReadClient());
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "PCM: Reachable" }));
    await screen.findByRole("heading", { name: "PCM" });

    const accepts = screen.getByText("What this module will accept");
    expect(accepts).toBeVisible();
    fireEvent.click(accepts);
    expect(screen.getByText(/sends none of them/)).toBeVisible();
    // The service, the session, the security level, the run time and the
    // class it would cost, on one line each.
    expect(screen.getByText("Synthetic clear adaption values")).toBeVisible();
    expect(
      screen.getByText(
        /0x0406 · service 0x31 · session 03 · security level_1 · up to 30 s · SERVICE_ROUTINE/,
      ),
    ).toBeVisible();
    expect(screen.getByText("Synthetic engine input")).toBeVisible();
    expect(
      screen.getByText(/0x1259 · service 0x2E · session 03 · PERSISTENT_CHANGE/),
    ).toBeVisible();
    // Nothing in the section is a control.
    const buttons = screen.getAllByRole("button").map((button) => button.textContent);
    expect(buttons.some((label) => label?.includes("Synthetic clear adaption values"))).toBe(false);
    expect(buttons.some((label) => label?.includes("Synthetic engine input"))).toBe(false);

    // A module that declares none shows no section at all.
    fireEvent.click(screen.getByRole("button", { name: "BCM: Unverified route" }));
    await screen.findByRole("heading", { name: "BCM" });
    expect(screen.queryByText("What this module will accept")).toBeNull();
  });

  it("says a self test in SDD's own words, Russian to a Ukrainian reader unless told otherwise", async () => {
    resetHelpLanguage();
    renderApp(new ControlledModuleReadClient());
    await screen.findByRole("button", { name: "Read" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "BCM: Unverified route" }));
    await screen.findByRole("heading", { name: "BCM" });
    // The self tests are a spoiler now (2026-09-18): opened once here, and
    // it stays open while the language changes underneath.
    fireEvent.click(screen.getByText("Self tests this module declares"));
    expect(screen.getByText("Synthetic screen text, second line.")).toBeVisible();

    // The interface's own language picker, the way a reader changes it.
    fireEvent.click(screen.getByRole("button", { name: "Українська" }));
    // SDD's own Russian for a Ukrainian interface (ADR-0034): the pack's
    // text, not ours; the heading is the interface's.
    expect(
      await screen.findByText("Синтетический текст экрана, вторая строка."),
    ).toBeVisible();
    expect(screen.getByText("Самотести, які оголошує цей модуль")).toBeVisible();
    // And English one switch away; Ukrainian is not on offer for SDD's own
    // text — the only Ukrainian mark on screen is the interface's own, which
    // lives on the plate in the header.
    fireEvent.click(screen.getAllByRole("button", { name: "Англійська" })[0]);
    expect(await screen.findByText("Synthetic screen text, second line.")).toBeVisible();
    const textLanguage = screen.getAllByRole("group", { name: "Мова тексту" })[0];
    expect(
      within(textLanguage).queryByRole("button", { name: "Українська" }),
    ).toBeNull();
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
    fireEvent.change(screen.getByLabelText("Which value"), { target: { value: "0x1945" } });
    expect(read).toBeEnabled();
    fireEvent.click(read);
    await screen.findByRole("button", { name: "PCM: Answered" });
    expect(client.requests[0].kind).toBe("IDENTIFIER");
    expect(client.requests[0].identifier).toBe("0x1945");
    // The value is shown decoded, with its unit.
    expect(screen.getByText("1726 rpm")).toBeVisible();
  });
});
