import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createBenchSnapshot, type AdapterClient, type AdapterSnapshot } from "./adapter";
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
  type VehicleDescription,
  type VehicleSurveySnapshot,
  type VinDecodeSnapshot,
} from "./library";
import {
  createModuleReadSnapshot,
  type ModuleReadClient,
  type ModuleReadRequest,
  type ModuleReadSnapshot,
} from "./moduleRead";
import {
  createDtcClearSnapshot,
  type DtcClearRequest,
  type DtcClearSnapshot,
  type ServiceClient,
} from "./serviceMode";
import {
  createSessionReportSnapshot,
  type SessionReportClient,
  type SessionReportSnapshot,
} from "./sessionReport";

/**
 * The service mode and the first operation behind it (ADR-0036): off until
 * the person consents, on for the session with a band saying so, the clear
 * offered only under a module's fault codes once they were read, one
 * question before it, and what the module answered shown afterwards.
 */

class BenchAdapterClient implements AdapterClient {
  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(createBenchSnapshot(1));
  }
  discover() {
    return this.getState();
  }
  connect() {
    return this.getState();
  }
  connectBench() {
    return this.getState();
  }
  disconnect() {
    return this.getState();
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
      hypothesis: 0,
      unreachable: 0,
      message: "1 module known: 1 reachable over the adapter.",
    });
  }
}

/** A module that reports two codes, as the bench's PCM does. */
class TwoCodesClient implements ModuleReadClient {
  requests: ModuleReadRequest[] = [];

  getState(): Promise<ModuleReadSnapshot> {
    return Promise.resolve(createModuleReadSnapshot());
  }

  read(request: ModuleReadRequest): Promise<ModuleReadSnapshot> {
    this.requests.push(request);
    const dtc = (code: string, description: string) => ({
      code,
      failureType: "00",
      status: "09",
      description,
      descriptionScope: "module",
      failureTypeText: null,
      failureTypeTexts: {},
      descriptionDataTexts: {},
      descriptionTexts: {},
      help: [],
      helpTexts: {},
      helpNote: null,
    });
    return Promise.resolve({
      ...createModuleReadSnapshot(),
      state: "SUCCEEDED",
      ecuFamily: request.ecuFamily,
      operation: "Read confirmed fault codes",
      routeId: "hs-can",
      routeValidation: "SYNTHETIC",
      requestHex: "19 02 FF",
      responder: "0x7E8",
      rawResponseHex: "59 02 FF 03 00 00 09 03 01 00 09",
      dataHex: null,
      parameters: [],
      dtcs: [dtc("P0300", "Random misfire detected"), dtc("P0301", "Cylinder 1 misfire detected")],
      negativeResponse: null,
      pendingResponses: 0,
      error: null,
      reportAvailable: true,
    });
  }

  getReportJson() {
    return Promise.resolve("{}");
  }
}

class RecordingSessionClient implements SessionReportClient {
  snapshot: SessionReportSnapshot = createSessionReportSnapshot();
  getState() {
    return Promise.resolve(this.snapshot);
  }
  getReportJson() {
    return Promise.resolve("{}");
  }
}

/** The shell's side of the mode and the clear, remembered as the shell remembers it. */
class RecordingServiceClient implements ServiceClient {
  on = false;
  clears: DtcClearRequest[] = [];
  constructor(private readonly session: RecordingSessionClient) {}
  setServiceMode(on: boolean) {
    this.on = on;
    this.session.snapshot = { ...this.session.snapshot, serviceMode: on };
    return Promise.resolve(this.session.snapshot);
  }
  getClearState() {
    return Promise.resolve(createDtcClearSnapshot());
  }
  clearDtcs(request: DtcClearRequest): Promise<DtcClearSnapshot> {
    this.clears.push(request);
    return Promise.resolve({
      ...createDtcClearSnapshot(),
      state: "CLEARED",
      ecuFamily: request.ecuFamily,
      routeId: "hs-can",
      routeValidation: "SYNTHETIC",
      protocol: "ISO14229",
      session: "0x03",
      requestHex: "14 FF FF FF",
      rawResponseHex: "54",
      codesBefore: [],
      codesAfter: [],
      reportAvailable: true,
    });
  }
}

function renderApp(
  service: RecordingServiceClient,
  session: RecordingSessionClient,
  reads: TwoCodesClient,
) {
  return render(
    <App
      client={new BenchAdapterClient()}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={new SurveyingLibraryClient()}
      moduleReadClient={reads}
      serviceClient={service}
      sessionReportClient={session}
      pollIntervalMs={60_000}
    />,
  );
}

describe("the service mode (ADR-0036)", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("is off until the person consents, then on for the session with a band saying so", async () => {
    const session = new RecordingSessionClient();
    const service = new RecordingServiceClient(session);
    renderApp(service, session, new TwoCodesClient());
    const toggle = await screen.findByRole("button", { name: "Service mode" });
    expect(toggle).toHaveAttribute("aria-pressed", "false");
    expect(screen.queryByText(/SERVICE MODE/)).toBeNull();

    // The consent stands before the switch; declining leaves everything off.
    fireEvent.click(toggle);
    const consent = screen.getByRole("dialog", { name: "Service mode" });
    expect(within(consent).getByText(/undoes nothing by itself/)).toBeVisible();
    expect(within(consent).getByRole("button", { name: "Cancel" })).toHaveFocus();
    fireEvent.click(within(consent).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(service.on).toBe(false);

    // Accepting turns it on: the shell is told, the bands appear, the lamp is red.
    fireEvent.click(toggle);
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    await waitFor(() => expect(service.on).toBe(true));
    await waitFor(() => expect(screen.getAllByText(/SERVICE MODE/)).toHaveLength(2));
    expect(screen.getByRole("button", { name: "Service mode" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("heading", { name: "ProwlOne" })).toHaveAttribute(
      "data-lamp",
      "sending",
    );

    // The same button turns it off again, without a question.
    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    await waitFor(() => expect(service.on).toBe(false));
    await waitFor(() => expect(screen.queryByText(/SERVICE MODE/)).toBeNull());
  });

  it("offers the clear only in the mode and only after the codes were read, asks once, and shows the answer", async () => {
    const session = new RecordingSessionClient();
    const service = new RecordingServiceClient(session);
    const reads = new TwoCodesClient();
    renderApp(service, session, reads);
    await screen.findByRole("button", { name: "Service mode" });

    // Survey, pick the PCM, read its codes: no clear is offered while the mode is off.
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "PCM: Reachable" }));
    await screen.findByRole("heading", { name: "PCM" });
    fireEvent.click(screen.getByRole("button", { name: "Read" }));
    expect(await screen.findByText("P0300")).toBeVisible();
    expect(screen.queryByRole("button", { name: "Clear the fault codes" })).toBeNull();

    // The mode goes on: the clear appears under the codes.
    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    const clearButton = await screen.findByRole("button", { name: "Clear the fault codes" });

    // One question, naming the module and what will be erased; the
    // confirming control names the operation.
    fireEvent.click(clearButton);
    const question = screen.getByRole("dialog", { name: "Clear the fault codes of PCM" });
    expect(within(question).getByText(/2 code\(s\) will be erased/)).toBeVisible();
    expect(within(question).getByText(/SERVICE_ROUTINE/)).toBeVisible();
    fireEvent.click(within(question).getByRole("button", { name: "Clear the codes of PCM" }));

    await waitFor(() => expect(service.clears).toHaveLength(1));
    expect(service.clears[0].ecuFamily).toBe("PCM");
    // The answer: accepted, in the extended session, on the bench, nothing left.
    expect(await screen.findByText("The module accepted the clear.")).toBeVisible();
    expect(screen.getByText(/in session 0x03/)).toBeVisible();
    expect(screen.getByText("Read again: no codes reported.")).toBeVisible();
  });
});
