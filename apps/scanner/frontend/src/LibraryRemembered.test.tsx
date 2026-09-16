import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
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
const NEWER = "C:\\Users\\owner\\prowlone-library-Oleg-16";

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

/**
 * The same client, with the reading held open so the test decides when each
 * folder answers and in which order.
 */
class SlowLibrary extends RecordingLibrary {
  private waiting = new Map<string, (snapshot: LibrarySnapshot) => void>();

  override loadDirectory(directory: string): Promise<LibrarySnapshot> {
    this.asked.push(directory);
    return new Promise((resolve) => {
      this.waiting.set(directory, resolve);
    });
  }

  answer(directory: string, message: string) {
    const resolve = this.waiting.get(directory);
    if (resolve === undefined) throw new Error("nothing is reading " + directory);
    this.waiting.delete(directory);
    resolve({ ...createLibrarySnapshot(), state: "LOADED", directory, message });
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

  /**
   * Reading a copy of the real size runs for tens of seconds, and on
   * 2026-09-16 the owner had a newer library to put in place of the older one
   * the application was busy reading. Until then both buttons were disabled
   * for all of that time. The restore must not take the choice away, and its
   * late answer must not land on top of the folder chosen since.
   */
  it("is overtaken by a folder chosen while it is still being read", async () => {
    window.localStorage.setItem(KEY, FOLDER);
    const library = new SlowLibrary();
    render(<App client={new NoAdapter()} libraryClient={library} pollIntervalMs={100_000} />);
    await waitFor(() => expect(library.asked).toEqual([FOLDER]));

    // Nothing is taken away while the remembered folder is being read.
    const load = screen.getByRole("button", { name: "Load library" });
    expect(load).toBeEnabled();
    fireEvent.change(screen.getByDisplayValue(FOLDER), { target: { value: NEWER } });
    fireEvent.click(load);
    await waitFor(() => expect(library.asked).toEqual([FOLDER, NEWER]));

    // The newer folder answers first and the older one afterwards; what
    // stands on screen is the newer one, and it is what is remembered.
    await act(async () => {
      library.answer(NEWER, "Loaded 16 bundles.");
    });
    await act(async () => {
      library.answer(FOLDER, "Loaded 13 bundles.");
    });
    expect(await screen.findByText("Loaded 16 bundles.")).toBeVisible();
    expect(screen.queryByText("Loaded 13 bundles.")).toBeNull();
    expect(window.localStorage.getItem(KEY)).toBe(NEWER);
  });
});
