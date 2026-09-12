import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { createEmptySnapshot, type AdapterClient, type AdapterSnapshot } from "./adapter";
import {
  createDiagnosticSnapshot,
  type DiagnosticClient,
  type DiagnosticSnapshot,
} from "./diagnostic";
import { LANGUAGE_STORAGE_KEY, currentLanguage, dataText, setCurrentLanguage, t } from "./i18n";
import {
  createLibrarySnapshot,
  type LibraryClient,
  type LibrarySnapshot,
  type VehicleCatalogueSnapshot,
  type VehicleDescription,
  type VehicleSurveySnapshot,
  type VinDecodeSnapshot,
} from "./library";

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

class NamedSurveyClient implements LibraryClient {
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
      valid: false,
      message: "",
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
      modules: [
        {
          ecuFamily: "PCM",
          name: "Powertrain control module",
          names: { eng: "Powertrain control module", rus: "Блок управления силовым агрегатом" },
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
          readableIdentifiers: [],
          selfTests: [],
        },
      ],
      reachable: 1,
      hypothesis: 0,
      unreachable: 0,
      message: "1 module known.",
    });
  }
}

describe("interface language", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("translates by English text, fills placeholders, and falls back to English", () => {
    expect(t("Vehicle network")).toBe("Vehicle network");
    expect(t("Check all modules ({count})", { count: 3 })).toBe("Check all modules (3)");
    setCurrentLanguage("uk");
    expect(currentLanguage()).toBe("uk");
    expect(t("Vehicle network")).toBe("Мережа автомобіля");
    expect(t("Check all modules ({count})", { count: 3 })).toBe("Перевірити всі модулі (3)");
    // A string without a translation shows its English, never a key.
    expect(t("Not translated anywhere")).toBe("Not translated anywhere");
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("uk");
  });

  it("switches the whole interface from the header and remembers the choice", async () => {
    render(
      <App
        client={new IdleAdapterClient()}
        diagnosticClient={new IdleDiagnosticClient()}
        pollIntervalMs={60_000}
      />,
    );
    expect(await screen.findByRole("heading", { name: "Vehicle network" })).toBeVisible();

    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), {
      target: { value: "uk" },
    });
    expect(screen.getByRole("heading", { name: "Мережа автомобіля" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Зберегти звіт сесії" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Оглянути модулі" })).toBeInTheDocument();
    expect(screen.getByText("Підключити адаптер")).toBeVisible();
    expect(screen.getByRole("heading", { name: "Захоплення шини" })).toBeVisible();
    expect(window.localStorage.getItem(LANGUAGE_STORAGE_KEY)).toBe("uk");
  });

  it("shows SDD's data text in Russian with the Russian interface and in English otherwise", async () => {
    expect(dataText({ eng: "Engine", rus: "Двигатель" }, "Engine", "ru")).toBe("Двигатель");
    expect(dataText({ eng: "Engine", rus: "Двигатель" }, "Engine", "uk")).toBe("Engine");
    expect(dataText({}, null, "ru")).toBe(null);

    render(
      <App
        client={new IdleAdapterClient()}
        diagnosticClient={new IdleDiagnosticClient()}
        libraryClient={new NamedSurveyClient()}
        pollIntervalMs={60_000}
      />,
    );
    await screen.findByRole("heading", { name: "Vehicle network" });
    fireEvent.change(screen.getByLabelText("Programme"), { target: { value: "X250" } });
    fireEvent.click(screen.getByRole("button", { name: "Survey modules" }));
    expect(await screen.findByText("Powertrain control module")).toBeVisible();

    fireEvent.change(screen.getByRole("combobox", { name: "Language" }), {
      target: { value: "ru" },
    });
    expect(screen.getByRole("heading", { name: "Сеть автомобиля" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Сохранить отчёт сеанса" })).toBeInTheDocument();
    expect(screen.getByText("Блок управления силовым агрегатом")).toBeVisible();
    expect(screen.queryByText("Powertrain control module")).not.toBeInTheDocument();

    // Ukrainian has no SDD text: the module keeps its English name.
    fireEvent.change(screen.getByRole("combobox", { name: "Язык" }), {
      target: { value: "uk" },
    });
    expect(screen.getByText("Powertrain control module")).toBeVisible();
    expect(screen.getByRole("heading", { name: "Мережа автомобіля" })).toBeVisible();
  });
});
