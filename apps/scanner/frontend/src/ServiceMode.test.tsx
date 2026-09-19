import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { createBatterySnapshot, type BatteryClient, type BatteryReadSnapshot } from "./battery";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import {
  createRoutineRunSnapshot,
  type RoutineRunClient,
  type RoutineRunRequest,
  type RoutineRunSnapshot,
} from "./routineRun";
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
  type DtcSummary,
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
    selfTests: [
      {
        testId: "202",
        name: "ODST_0202_PCM_HLP",
        timeMs: 3000,
        timeoutMs: 5000,
        description: ["Make sure the ignition is switched on."],
        descriptionTexts: {},
        modelYears: [],
        safetyClass: "SERVICE_ROUTINE",
      },
    ],
    acceptedOperations: [
      {
        kind: "ROUTINE",
        identifier: "0x0202",
        name: "Self Test",
        service: "0x31",
        sessions: ["03"],
        security: null,
        maxRunTime: null,
        restartWhileRunning: null,
        safetyClass: "SERVICE_ROUTINE",
      },
    ],
  },
];

/** A second module with codes of its own, for the collective clear. */
const twoSurveyed: ModuleSurveyEntry[] = [
  surveyed[0],
  {
    ...surveyed[0],
    ecuFamily: "TCM",
    requestId: "0x7E1",
    responseId: "0x7E9",
  },
];

class SurveyingLibraryClient implements LibraryClient {
  constructor(private readonly modules: ModuleSurveyEntry[] = surveyed) {}
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
      modules: this.modules,
      reachable: this.modules.length,
      hypothesis: 0,
      unreachable: 0,
      message: `${this.modules.length} module(s) known: ${this.modules.length} reachable over the adapter.`,
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

/** A code the self test logs, as the module's read after the run answers it. */
const loggedByTheTest: DtcSummary = {
  code: "B1D01",
  failureType: "11",
  status: "09",
  description: "Steering wheel switch circuit open",
  descriptionScope: "module",
  failureTypeText: null,
  failureTypeTexts: {},
  descriptionTexts: {},
  descriptionDataTexts: {},
  help: [],
  helpTexts: {},
  helpNote: null,
};

/** The shell's side of a self test, remembered as the shell remembers it:
 *  the start, then a step or two, then the module's fixed status record. */
class RecordingRoutineClient implements RoutineRunClient {
  starts: RoutineRunRequest[] = [];
  steps = 0;
  stops = 0;
  private last: RoutineRunSnapshot = createRoutineRunSnapshot();
  getState() {
    return Promise.resolve(this.last);
  }
  start(request: RoutineRunRequest) {
    this.starts.push(request);
    this.last = {
      ...createRoutineRunSnapshot(),
      state: "RUNNING",
      ecuFamily: request.ecuFamily,
      testId: request.testId,
      testName: "ODST_0202_PCM_HLP",
      routeId: "hs-can",
      routeValidation: "SYNTHETIC",
      session: "0x03",
      timeMs: 3000,
      timeoutMs: 5000,
      exchanges: [
        ["10 03", "50 03 00 32 01 F4"],
        ["31 01 02 02", "71 01 02 02"],
      ],
    };
    return Promise.resolve(this.last);
  }
  step() {
    this.steps += 1;
    if (this.last.state === "RUNNING" && this.steps >= 2) {
      this.last = {
        ...this.last,
        state: "COMPLETED",
        resultHex: "00 A5 5A",
        elapsedMs: 3100,
        // Read again once the run ended: one code that was not there before.
        codesAfter: [loggedByTheTest],
        codesFound: [loggedByTheTest],
        exchanges: [
          ...this.last.exchanges,
          ["3E 00", "7E 00"],
          ["31 03 02 02", "71 03 02 02 00 A5 5A"],
          ["10 01", "50 01 00 32 01 F4"],
        ],
        reportAvailable: true,
      };
    }
    return Promise.resolve(this.last);
  }
  stop() {
    this.stops += 1;
    this.last = { ...this.last, state: "STOPPED", reportAvailable: true };
    return Promise.resolve(this.last);
  }
}

/** A battery that always answers the same reading, for the precondition. */
class FixedBatteryClient implements BatteryClient {
  constructor(private readonly fixed: BatteryReadSnapshot) {}
  getState() {
    return Promise.resolve(this.fixed);
  }
  start() {
    return Promise.resolve(this.fixed);
  }
  step() {
    return Promise.resolve(this.fixed);
  }
  finish() {
    return Promise.resolve(this.fixed);
  }
}

/** A last battery read in SDD's low band: 11.2 V under the 11.6 V edge. */
function lowBatteryRead(): BatteryReadSnapshot {
  return {
    ...createBatterySnapshot(),
    state: "FINISHED",
    planned: 1,
    asked: 1,
    answered: 1,
    modules: 1,
    routeValidation: "SYNTHETIC",
    readUnixMs: Date.now(),
    sddLowVoltageMaxMv: 11_600,
    readings: [
      {
        ecuFamily: "BCM",
        identifier: "0x402A",
        parameter: "Vehicle Battery Voltage",
        role: "VOLTAGE",
        headline: true,
        state: "SUCCEEDED",
        value: "11.2",
        unit: "V",
        routeId: "hs-can",
        routeValidation: "SYNTHETIC",
        rawResponseHex: "62 40 2A 68",
        negativeResponse: null,
        note: null,
        reason: null,
      },
    ],
  };
}

function renderApp(
  service: RecordingServiceClient,
  session: RecordingSessionClient,
  reads: TwoCodesClient,
  battery?: BatteryClient,
  modules: ModuleSurveyEntry[] = surveyed,
  routine: RoutineRunClient = new RecordingRoutineClient(),
) {
  return render(
    <App
      client={new BenchAdapterClient()}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={new SurveyingLibraryClient(modules)}
      routineRunClient={routine}
      routineStepMs={10}
      moduleReadClient={reads}
      serviceClient={service}
      sessionReportClient={session}
      batteryClient={battery}
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

    // The consent stands before the switch; declining leaves everything off.
    fireEvent.click(toggle);
    const consent = screen.getByRole("dialog", { name: "Service mode" });
    expect(within(consent).getByText(/undoes nothing by itself/)).toBeVisible();
    expect(within(consent).getByRole("button", { name: "Cancel" })).toHaveFocus();
    fireEvent.click(within(consent).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(service.on).toBe(false);

    // Accepting turns it on: the shell is told, the switch itself goes red
    // (pressed), the lamp is red; no band anywhere (the owner, 2026-09-18).
    fireEvent.click(toggle);
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    await waitFor(() => expect(service.on).toBe(true));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Service mode" })).toHaveAttribute(
        "aria-pressed",
        "true",
      ),
    );
    expect(screen.getByRole("button", { name: "Service mode" }).className).toContain("button--service");
    expect(screen.queryByText(/SERVICE MODE/)).toBeNull();
    expect(screen.getByRole("heading", { name: "ProwlOne" })).toHaveAttribute(
      "data-lamp",
      "sending",
    );

    // The same button turns it off again, without a question.
    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    await waitFor(() => expect(service.on).toBe(false));
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Service mode" })).toHaveAttribute(
        "aria-pressed",
        "false",
      ),
    );
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
    // The code is on screen twice now, on purpose: in the list of what the
    // check found and in the module's own panel (2026-09-18).
    expect((await screen.findAllByText("P0300")).length).toBeGreaterThan(0);
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

  /**
   * The one collective clear (ADR-0036, decision 3; built 2026-09-19): over
   * every module the check found codes in, after one question that lists
   * them, each module asked in turn and each answer recorded on its own;
   * the list of what the car answered follows the clears.
   */
  it("clears every module that answered with codes after one question, in turn", async () => {
    const session = new RecordingSessionClient();
    const service = new RecordingServiceClient(session);
    const reads = new TwoCodesClient();
    renderApp(service, session, reads, undefined, twoSurveyed);
    await screen.findByRole("button", { name: "Service mode" });

    // Survey both, check them all: four codes in two modules on the list.
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: /^Check all modules/ }));
    await screen.findByRole("heading", { name: "4 fault code(s) in 2 module(s)" });
    // Nothing collective while the mode is off.
    expect(screen.queryByRole("button", { name: /^Clear the codes of all/ })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    fireEvent.click(await screen.findByRole("button", { name: "Clear the codes of all 2 modules" }));

    // One question, listing the modules it will ask and what will be erased.
    const question = screen.getByRole("dialog", { name: "Clear the fault codes of 2 modules" });
    expect(within(question).getByText("PCM")).toBeVisible();
    expect(within(question).getByText("TCM")).toBeVisible();
    expect(
      within(question).getByText(/4 code\(s\) will be erased from 2 module\(s\), one module after another/),
    ).toBeVisible();
    fireEvent.click(within(question).getByRole("button", { name: "Clear the codes of 2 modules" }));

    // Each module in turn, each answer its own clear.
    await waitFor(() => expect(service.clears).toHaveLength(2));
    expect(service.clears.map((clear) => clear.ecuFamily)).toEqual(["PCM", "TCM"]);
    expect(
      await screen.findByText("Cleared in turn: 2 accepted, 0 refused, 0 not made."),
    ).toBeVisible();
    // Read again, both empty: the list says so, and offers no second clear.
    expect(await screen.findByRole("heading", { name: "No fault codes" })).toBeVisible();
    expect(screen.queryByRole("button", { name: /^Clear the codes of all/ })).toBeNull();
  });

  /**
   * The on-demand self test (ADR-0036, step 2; built 2026-09-19): listed as
   * before while the mode is off; in the mode, one key under the test the
   * index and the pack agree on, one question with SDD's own instructions
   * and the data's times, then the run stepped until the shell says it
   * ended, and the module's answer shown as the bytes it is.
   */
  it("runs the self test after one question, steps it, and shows the result as the module answered it", async () => {
    const session = new RecordingSessionClient();
    const service = new RecordingServiceClient(session);
    const routine = new RecordingRoutineClient();
    renderApp(service, session, new TwoCodesClient(), undefined, surveyed, routine);
    await screen.findByRole("button", { name: "Service mode" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "PCM: Reachable" }));
    await screen.findByRole("heading", { name: "PCM" });
    // Listed, and not offered, while the mode is off.
    fireEvent.click(screen.getByText("Self tests this module declares"));
    expect(screen.getByText("ODST_0202_PCM_HLP")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Run the test" })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    fireEvent.click(await screen.findByRole("button", { name: "Run the test" }));
    // One question: the module and the test, SDD's instructions, the times,
    // the class; the confirming control names the operation.
    const question = screen.getByRole("dialog", { name: "Run the self test on PCM" });
    expect(within(question).getByText("Make sure the ignition is switched on.")).toBeVisible();
    expect(within(question).getByText("It runs 3 s; the tool waits 5 s at most.")).toBeVisible();
    expect(within(question).getByText(/SERVICE_ROUTINE/)).toBeVisible();
    fireEvent.click(within(question).getByRole("button", { name: "Run the test ODST_0202_PCM_HLP on PCM" }));

    await waitFor(() => expect(routine.starts).toHaveLength(1));
    expect(routine.starts[0]).toMatchObject({ ecuFamily: "PCM", testId: "202" });
    // Stepped until the shell says it ended; the answer is bytes, said so.
    expect(await screen.findByText("The module answered: 00 A5 5A")).toBeVisible();
    expect(routine.steps).toBeGreaterThanOrEqual(2);
    // And what it found: the code read after the run that was not there before.
    expect(screen.getByText("What the test found: 1 code(s)")).toBeVisible();
    expect(screen.getByText("Steering wheel switch circuit open")).toBeVisible();
    expect(
      screen.getByText("The result is shown as the module answers it; this library does not describe it."),
    ).toBeVisible();
  });

  it("repeats the low-battery line in the clear's confirmation (ADR-0030, 2026-09-19)", async () => {
    const session = new RecordingSessionClient();
    const service = new RecordingServiceClient(session);
    renderApp(service, session, new TwoCodesClient(), new FixedBatteryClient(lowBatteryRead()));
    await screen.findByRole("button", { name: "Service mode" });
    // The line stands under the header as soon as the reading is known.
    expect(await screen.findByText(/Connect an external power supply/)).toBeVisible();

    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "L405" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    fireEvent.click(await screen.findByRole("button", { name: "PCM: Reachable" }));
    await screen.findByRole("heading", { name: "PCM" });
    fireEvent.click(screen.getByRole("button", { name: "Read" }));
    expect((await screen.findAllByText("P0300")).length).toBeGreaterThan(0);
    fireEvent.click(screen.getByRole("button", { name: "Service mode" }));
    fireEvent.click(screen.getByRole("button", { name: "Turn the service mode on" }));
    fireEvent.click(await screen.findByRole("button", { name: "Clear the fault codes" }));

    // And again inside the question, before anything is sent, beside the
    // other preconditions; the person still decides.
    const question = screen.getByRole("dialog", { name: "Clear the fault codes of PCM" });
    expect(within(question).getByText(/11\.2 V/)).toBeVisible();
    expect(within(question).getByText(/Connect an external power supply/)).toBeVisible();
    expect(within(question).getByRole("button", { name: "Clear the codes of PCM" })).toBeEnabled();
  });
});
