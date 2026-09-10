import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";
import { createEmptySnapshot, type AdapterClient, type AdapterSnapshot } from "./adapter";
import {
  createLibrarySnapshot,
  createVehicleDescription,
  type LibraryClient,
  type LibrarySnapshot,
  type VehicleCatalogueSnapshot,
  type VinDecodeSnapshot,
} from "./library";

/**
 * Choosing the folder again on every launch is work the machine can do, and
 * the owner said so on 2026-09-10. The path is what is remembered, never the
 * data: the stamp is verified on every start, so a copy stops working on the
 * date it should.
 */
const KEY = "prowlone.libraryDirectory";
const FOLDER = "C:\\Users\\owner\\prowlone-library-Oleg";

class NoAdapter implements AdapterClient {
  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
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

class RecordingLibrary implements LibraryClient {
  asked: string[] = [];

  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve(createLibrarySnapshot());
  }
  loadDirectory(directory: string): Promise<LibrarySnapshot> {
    this.asked.push(directory);
    return Promise.resolve({
      ...createLibrarySnapshot(),
      state: "LOADED",
      directory,
      message: "Loaded 6510 manifests.",
    });
  }
  getCatalogue(): Promise<VehicleCatalogueSnapshot> {
    return Promise.resolve({ programmes: [] });
  }
  surveyVehicle() {
    return Promise.resolve({
      context: createVehicleDescription(),
      modules: [],
      reachable: 0,
      hypothesis: 0,
      unreachable: 0,
      message: "",
    });
  }
  decodeVin(): Promise<VinDecodeSnapshot> {
    return Promise.resolve({
      vin: "",
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
}

describe("the library folder between launches", () => {
  beforeEach(() => {
    window.localStorage.clear();
  });
  afterEach(() => {
    window.localStorage.clear();
  });

  it("is read on start when one worked last time", async () => {
    window.localStorage.setItem(KEY, FOLDER);
    const library = new RecordingLibrary();
    render(<App client={new NoAdapter()} libraryClient={library} pollIntervalMs={100_000} />);
    await waitFor(() => expect(library.asked).toEqual([FOLDER]));
    expect(await screen.findByDisplayValue(FOLDER)).toBeInTheDocument();
  });

  it("asks for nothing on a first launch", async () => {
    const library = new RecordingLibrary();
    render(<App client={new NoAdapter()} libraryClient={library} pollIntervalMs={100_000} />);
    await waitFor(() => expect(screen.getByText("Adapter not detected")).toBeInTheDocument());
    expect(library.asked).toEqual([]);
  });
});
