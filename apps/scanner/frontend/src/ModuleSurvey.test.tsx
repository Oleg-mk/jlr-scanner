import { fireEvent, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
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

const loadedLibrary = (directory: string): LibrarySnapshot => ({
  ...createLibrarySnapshot(),
  state: "LOADED",
  directory,
  manifestsLoaded: 8,
  sources: 3,
  records: 120,
  message: "Loaded 6 manifests: 3 sources, 120 records.",
});

const surveyOf = (vehicle: VehicleDescription): VehicleSurveySnapshot => ({
  context: vehicle,
  modules: [
    {
      ecuFamily: "OTHERMOD",
      name: null,
      names: {},
      applicability: "APPLICABLE",
      logicalNetwork: "CAN_MS",
      requestId: "0x760",
      responseId: "0x768",
      backendRoute: "ms-can",
      pins: "3/11",
      bitrateBps: 125_000,
      protocol: null,
      routeValidation: "",
      identifierRead: {
        status: "INDETERMINATE",
        reasons: ["diagnostic protocol: no applicable evidence-backed value"],
      },
      dtcRead: {
        status: "INDETERMINATE",
        reasons: ["diagnostic protocol: no applicable evidence-backed value"],
      },
      readableIdentifiers: [],
    },
    {
      ecuFamily: "SYNTHMOD",
      name: "Synthetic control module",
      names: { eng: "Synthetic control module", rus: "Синтетический блок управления" },
      applicability: "APPLICABLE",
      logicalNetwork: "CAN_HS",
      requestId: "0x7E0",
      responseId: "0x7E8",
      backendRoute: "hs-can",
      pins: "6/14",
      bitrateBps: 500_000,
      protocol: "ISO14229",
      routeValidation: "UNVERIFIED",
      identifierRead: { status: "REACHABLE", reasons: [] },
      dtcRead: { status: "REACHABLE", reasons: [] },
      readableIdentifiers: [
        { identifier: "0x0347", parameters: ["Synthetic fully qualified value"] },
        { identifier: "0x1945", parameters: ["Synthetic module-scoped value"] },
      ],
    },
  ],
  reachable: 1,
  hypothesis: 0,
  unreachable: 1,
  message: "2 modules known: 1 reachable over the adapter, 1 not — each with the reason.",
});

class ControlledLibraryClient implements LibraryClient {
  loadedDirectory: string | null = null;
  surveyedVehicle: VehicleDescription | null = null;

  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve(createLibrarySnapshot());
  }

  loadDirectory(directory: string): Promise<LibrarySnapshot> {
    this.loadedDirectory = directory;
    return Promise.resolve(loadedLibrary(directory));
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
    this.surveyedVehicle = vehicle;
    return Promise.resolve(surveyOf(vehicle));
  }
}

function renderApp(libraryClient: LibraryClient) {
  return render(
    <App
      client={new IdleAdapterClient()}
      diagnosticClient={new IdleDiagnosticClient()}
      libraryClient={libraryClient}
      pollIntervalMs={60_000}
    />,
  );
}

describe("data library and module survey", () => {
  it("loads a directory through the client and reports what it holds", async () => {
    const client = new ControlledLibraryClient();
    renderApp(client);

    expect(await screen.findByText("Built-in data only.")).toBeVisible();
    const load = screen.getByRole("button", { name: "Load library" });
    expect(load).toBeDisabled();

    fireEvent.change(screen.getByLabelText("Directory of exported manifests"), {
      target: { value: "D:\\library" },
    });
    expect(load).toBeEnabled();
    fireEvent.click(load);

    expect(
      await screen.findByText("Loaded 6 manifests: 3 sources, 120 records."),
    ).toBeVisible();
    expect(client.loadedDirectory).toBe("D:\\library");
    expect(screen.getByText("Loaded")).toBeVisible();
  });

  it("surveys the described vehicle and shows unreachable modules with reasons", async () => {
    const client = new ControlledLibraryClient();
    renderApp(client);
    await screen.findByText("Built-in data only.");

    const surveyButton = screen.getByRole("button", { name: "Survey modules" });
    expect(surveyButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "SYNTHA" } });
    fireEvent.change(screen.getByLabelText("Model year"), { target: { value: "2010" } });
    fireEvent.change(screen.getByLabelText("SDD breakpoint marker"), {
      target: { value: "MY10" },
    });
    expect(surveyButton).toBeEnabled();
    fireEvent.click(surveyButton);

    expect(
      await screen.findByText(/2 modules known: 1 reachable over the adapter, 1 not/),
    ).toBeVisible();
    expect(client.surveyedVehicle).toEqual({
      vehicleProgram: "SYNTHA",
      modelYear: 2010,
      powertrain: null,
      variant: null,
      market: null,
      yearBreakpoint: "MY10",
    });

    // The map: one lane per bus, the adapter route on the lane, each module a node
    // with SDD's own name for it when the data has one.
    expect(screen.getByRole("button", { name: "SYNTHMOD: Reachable" })).toBeVisible();
    expect(screen.getByText("Synthetic control module")).toBeVisible();
    expect(screen.getByText("hs-can · pins 6/14 · 500 kbit/s")).toBeVisible();
    expect(
      screen.getByText("not bound: diagnostic protocol: no applicable evidence-backed value"),
    ).toBeVisible();
    const other = screen.getByRole("button", { name: "OTHERMOD: Not reachable" });
    expect(other).toBeVisible();

    // Choosing the unreachable node explains why, in the module's own panel.
    fireEvent.click(other);
    expect(screen.getByRole("heading", { name: "OTHERMOD" })).toBeVisible();
    expect(
      screen.getAllByText("diagnostic protocol: no applicable evidence-backed value").length,
    ).toBeGreaterThan(0);
    expect(screen.getByRole("button", { name: "Read" })).toBeDisabled();
  });
  it("offers the vehicle as a list once the library describes programmes", async () => {
    class CataloguedLibraryClient extends ControlledLibraryClient {
      getCatalogue(): Promise<VehicleCatalogueSnapshot> {
        return Promise.resolve({
          programmes: [
            {
              program: "SYNTHA",
              markers: [{ marker: "MY10", modelYearFrom: 2010, modelYearTo: 2011 }],
              powertrains: ["SYNTHENGINE"],
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
    }
    const client = new CataloguedLibraryClient();
    renderApp(client);
    await screen.findByText("Built-in data only.");

    const programme = await screen.findByRole("combobox", { name: "Programme" });
    expect(screen.getByRole("option", { name: "SYNTHA" })).toBeInTheDocument();
    const years = screen.getByRole("combobox", { name: "Model years" });
    expect(years).toBeDisabled();
    fireEvent.change(programme, { target: { value: "SYNTHA" } });
    expect(years).toBeEnabled();
    expect(screen.getByRole("option", { name: "2010–2011 (MY10)" })).toBeInTheDocument();
    fireEvent.change(years, { target: { value: "MY10" } });
    fireEvent.change(screen.getByRole("combobox", { name: "Engine" }), {
      target: { value: "SYNTHENGINE" },
    });

    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    await screen.findByRole("button", { name: "SYNTHMOD: Reachable" });
    expect(client.surveyedVehicle).toEqual({
      vehicleProgram: "SYNTHA",
      modelYear: 2010,
      powertrain: "SYNTHENGINE",
      variant: null,
      market: null,
      yearBreakpoint: "MY10",
    });
  });

  it("decodes a VIN with the library's tables and pre-selects the vehicle", async () => {
    class DecodingLibraryClient extends ControlledLibraryClient {
      decodedVin: string | null = null;
      getCatalogue(): Promise<VehicleCatalogueSnapshot> {
        return Promise.resolve({
          programmes: [
            {
              program: "SYNTHA",
              markers: [
                { marker: "MY08", modelYearFrom: 2008, modelYearTo: 2009 },
                { marker: "MY10", modelYearFrom: 2010, modelYearTo: 2011 },
              ],
              powertrains: ["SYNTHENGINE"],
      variants: [],
            },
          ],
        });
      }
      decodeVin(vin: string): Promise<VinDecodeSnapshot> {
        this.decodedVin = vin;
        return Promise.resolve({
          vin: "SYNA102XB9AB12345",
          valid: true,
          message: "SDD's VIN tables read this as SYNTHA, model year 2011.",
          decodeModel: 1,
          candidates: [1],
          program: "SYNTHA",
          modelYear: 2011,
          attributes: [
            { name: "Brand", value: "Synthetic" },
            { name: "Engine", value: "3.0L V6 synthetic" },
          ],
          tableVersion: "SDD table version: issue 1",
        });
      }
    }
    const client = new DecodingLibraryClient();
    renderApp(client);
    await screen.findByRole("combobox", { name: "Programme" });

    const decode = screen.getByRole("button", { name: "Decode VIN" });
    expect(decode).toBeDisabled();
    fireEvent.change(screen.getByLabelText("VIN"), { target: { value: "syn a102xb9ab12345" } });
    expect(decode).toBeEnabled();
    fireEvent.click(decode);

    expect(
      await screen.findByText("SDD's VIN tables read this as SYNTHA, model year 2011."),
    ).toBeVisible();
    expect(client.decodedVin).toBe("syn a102xb9ab12345");
    expect(screen.getByText("3.0L V6 synthetic")).toBeVisible();
    expect(screen.getByRole("combobox", { name: "Programme" })).toHaveValue("SYNTHA");
    // 2011 falls in the MY10 breakpoint's range.
    expect(screen.getByRole("combobox", { name: "Model years" })).toHaveValue("MY10");
    expect(screen.getByRole("button", { name: "Survey modules" })).toBeEnabled();
  });
});
