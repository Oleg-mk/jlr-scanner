import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
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
  type VehicleCatalogueSnapshot,
  type VinDecodeSnapshot,
  type VehicleDescription,
  type VehicleSurveySnapshot,
} from "./library";
import {
  createSessionReportSnapshot,
  sessionSteps,
  type SessionReportClient,
  type SessionReportSnapshot,
} from "./sessionReport";
// The application remembers the library folder that worked, so a test that
// loads one would leave it for the next test in this file (2026-09-10).
beforeEach(() => {
  window.localStorage.clear();
});

class IdleAdapterClient implements AdapterClient {
  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
  }
  discover(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
  }
  connect(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
  }
  connectBench() {
    return this.getState();
  }

  disconnect(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
  }
}

class IdleDiagnosticClient implements DiagnosticClient {
  getState(): Promise<DiagnosticSnapshot> {
    return Promise.resolve(createDiagnosticSnapshot());
  }
  readCalibrationIdentification(): Promise<DiagnosticSnapshot> {
    return Promise.resolve(createDiagnosticSnapshot());
  }
  getReportJson(): Promise<string> {
    return Promise.resolve("{}");
  }
}

class CataloguedLibraryClient implements LibraryClient {
  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve(createLibrarySnapshot());
  }
  loadDirectory(directory: string): Promise<LibrarySnapshot> {
    return Promise.resolve({
      ...createLibrarySnapshot(),
      state: "LOADED",
      directory,
      manifestsLoaded: 8,
      sources: 3,
      records: 120,
      message: "Loaded 8 manifests: 3 sources, 120 records.",
    });
  }
  getCatalogue(): Promise<VehicleCatalogueSnapshot> {
    return Promise.resolve({
      programmes: [
        {
          program: "SYNTHA",
          markers: [{ marker: "MY10", modelYearFrom: 2010, modelYearTo: 2011 }],
          powertrains: [],
      variants: [],
        },
      ],
    });
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
      message: "No modules are known for this vehicle in the loaded data.",
    });
  }
}

class StubSessionReportClient implements SessionReportClient {
  reportRequests = 0;
  constructor(private readonly state: SessionReportSnapshot) {}
  getState(): Promise<SessionReportSnapshot> {
    return Promise.resolve(this.state);
  }
  getReportJson(): Promise<string> {
    this.reportRequests += 1;
    return Promise.resolve('{"schema":"jlr-scanner.session-report","saved_unix_ms":1}');
  }
}

function renderApp(sessionReportClient: SessionReportClient) {
  return render(
    <App
      client={new IdleAdapterClient()}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={new CataloguedLibraryClient()}
      sessionReportClient={sessionReportClient}
      pollIntervalMs={60_000}
    />,
  );
}

const stepState = (title: string) =>
  screen.getByLabelText(new RegExp(`^${title}:`)).textContent;

describe("session flow", () => {
  it("orders the steps and marks them as the session progresses", async () => {
    renderApp(new StubSessionReportClient(createSessionReportSnapshot()));
    await screen.findByText("Built-in data only.");

    expect(stepState("Connect the adapter")).toBe("Next");
    expect(stepState("Load the data library")).toBe("To do");
    expect(screen.getByRole("button", { name: "Save session report" })).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Directory of exported manifests"), {
      target: { value: "D:\\library" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Load library" }));
    await screen.findByText("Loaded 8 manifests: 3 sources, 120 records.");
    expect(stepState("Load the data library")).toBe("Done");

    fireEvent.change(await screen.findByRole("combobox", { name: "Programme" }), {
      target: { value: "SYNTHA" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "Model years" }), {
      target: { value: "MY10" },
    });
    expect(stepState("Choose the vehicle")).toBe("Done");
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    await screen.findByText(/No modules are known for this vehicle in the loaded data/);
    expect(stepState("Survey the modules")).toBe("Done");
    // The adapter is still the first open required step.
    expect(stepState("Connect the adapter")).toBe("Next");
    expect(stepState("Listen to a bus")).toBe("To do");
  });

  it("saves one report once something has been recorded", async () => {
    const client = new StubSessionReportClient({
      captures: 1,
      moduleReads: 2,
      calibrationReads: 0,
      standardObdReads: 0,
      liveReadRuns: 0,
      mileageSurveys: 0,
      modulePassports: 0,
      ccfReads: 0,
      batteryReads: 0,
      reportAvailable: true,
      mode: "real",
    });
    Object.assign(URL, { createObjectURL: vi.fn(() => "blob:session"), revokeObjectURL: vi.fn() });
    // jsdom cannot navigate; the download anchor's click is observed, not followed.
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    renderApp(client);

    const save = await screen.findByRole("button", { name: "Save session report" });
    expect(save).toBeEnabled();
    expect(
      screen.getByText(/Recorded in this session: 1 capture\(s\), 2 module read\(s\)/),
    ).toBeVisible();
    fireEvent.click(save);
    await screen.findByRole("button", { name: "Save session report" });
    expect(client.reportRequests).toBe(1);
    expect(click).toHaveBeenCalledTimes(1);
    click.mockRestore();
  });

  it("derives the next step from the evidence, never skipping a required one", () => {
    const steps = sessionSteps({
      adapterReady: true,
      libraryLoaded: true,
      vehicleDescribed: false,
      surveyed: false,
      captures: 0,
      moduleReads: 0,
      reportAvailable: false,
    });
    expect(steps.map((step) => `${step.id}:${step.state}`)).toEqual([
      "adapter:done",
      "library:done",
      "vehicle:next",
      "survey:todo",
      "capture:todo",
      "read:todo",
      "save:todo",
    ]);
    const complete = sessionSteps({
      adapterReady: true,
      libraryLoaded: true,
      vehicleDescribed: true,
      surveyed: true,
      captures: 1,
      moduleReads: 1,
      reportAvailable: true,
    });
    expect(complete.at(-1)).toMatchObject({ id: "save", state: "next" });
  });
});
