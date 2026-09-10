import { useCallback, useEffect, useState } from "react";
import { pickDirectory } from "./files";
import {
  createLibrarySnapshot,
  createVehicleDescription,
  type LibraryClient,
  type LibrarySnapshot,
  type VehicleCatalogueSnapshot,
  type VehicleDescription,
  type VehicleSurveySnapshot,
  type VinDecodeSnapshot,
} from "./library";

/**
 * Where the library folder is remembered between launches. The path only:
 * the stamp is verified on every start, so an expired copy stops working on
 * its own date, and no second copy of the data is written anywhere.
 */
const LIBRARY_DIRECTORY_KEY = "prowlone.libraryDirectory";

function readStoredDirectory(): string {
  try {
    return window.localStorage.getItem(LIBRARY_DIRECTORY_KEY) ?? "";
  } catch {
    // A browser that refuses storage is not a reason to fail to start.
    return "";
  }
}

function storeDirectory(directory: string) {
  try {
    if (directory.trim() === "") window.localStorage.removeItem(LIBRARY_DIRECTORY_KEY);
    else window.localStorage.setItem(LIBRARY_DIRECTORY_KEY, directory);
  } catch {
    // Nothing to do: the folder is simply asked for again next time.
  }
}

function failedLibrary(error: unknown): LibrarySnapshot {
  return {
    ...createLibrarySnapshot(),
    state: "FAILED",
    message: `Data library request failed: ${
      error instanceof Error ? error.message : String(error)
    }`,
  };
}

function failedSurvey(vehicle: VehicleDescription, error: unknown): VehicleSurveySnapshot {
  return {
    context: vehicle,
    modules: [],
    reachable: 0,
    hypothesis: 0,
    unreachable: 0,
    message: `Survey failed: ${error instanceof Error ? error.message : String(error)}`,
  };
}

export function useLibraryController(client: LibraryClient) {
  const [library, setLibrary] = useState<LibrarySnapshot>(createLibrarySnapshot);
  const [directory, setDirectory] = useState(readStoredDirectory);
  const [vehicle, setVehicle] = useState<VehicleDescription>(createVehicleDescription);
  const [survey, setSurvey] = useState<VehicleSurveySnapshot | null>(null);
  const [catalogue, setCatalogue] = useState<VehicleCatalogueSnapshot>({ programmes: [] });
  const [busy, setBusy] = useState(false);
  const [vin, setVin] = useState("");
  const [vinDecode, setVinDecode] = useState<VinDecodeSnapshot | null>(null);
  const [decodingVin, setDecodingVin] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getLibrary()
      .then((next) => {
        if (active) setLibrary(next);
      })
      .catch((error: unknown) => {
        if (active) setLibrary(failedLibrary(error));
      });
    void client
      .getCatalogue()
      .then((next) => {
        if (active) setCatalogue(next);
      })
      .catch(() => {
        if (active) setCatalogue({ programmes: [] });
      });
    return () => {
      active = false;
    };
  }, [client]);

  const load = useCallback(async () => {
    setBusy(true);
    try {
      const loaded = await client.loadDirectory(directory);
      setLibrary(loaded);
      // Remember a folder that worked, and forget one that did not: the next
      // launch reads it by itself rather than asking again.
      storeDirectory(loaded.state === "LOADED" ? directory : "");
      // A different library can answer differently; do not keep an old answer.
      setSurvey(null);
      setCatalogue(await client.getCatalogue());
    } catch (error) {
      setLibrary(failedLibrary(error));
    } finally {
      setBusy(false);
    }
  }, [client, directory]);

  // The folder that worked last time is read on start, without being asked
  // for again. The window stays usable while it reads, and the panel counts
  // the seconds; a folder that has gone, or a copy whose date has passed,
  // says so in the same words it would say them at any other time.
  const [restored, setRestored] = useState(false);
  useEffect(() => {
    if (restored) return;
    setRestored(true);
    if (readStoredDirectory().trim() !== "") void load();
  }, [load, restored]);

  const chooseDirectory = useCallback(async () => {
    const chosen = await pickDirectory();
    if (chosen) setDirectory(chosen);
  }, []);

  const runSurvey = useCallback(async () => {
    setBusy(true);
    try {
      setSurvey(await client.surveyVehicle(vehicle));
    } catch (error) {
      setSurvey(failedSurvey(vehicle, error));
    } finally {
      setBusy(false);
    }
  }, [client, vehicle]);

  const decodeVin = useCallback(async () => {
    setDecodingVin(true);
    try {
      const decoded = await client.decodeVin(vin);
      setVinDecode(decoded);
      // Pre-select what the tables established; the tester confirms or corrects.
      const programme = catalogue.programmes.find((entry) => entry.program === decoded.program);
      if (programme !== undefined) {
        const marker =
          decoded.modelYear === null
            ? undefined
            : programme.markers.find(
                (entry) =>
                  entry.modelYearFrom !== null &&
                  entry.modelYearFrom <= decoded.modelYear! &&
                  (entry.modelYearTo ?? entry.modelYearFrom) >= decoded.modelYear!,
              );
        setVehicle((current) => ({
          ...current,
          vehicleProgram: programme.program,
          yearBreakpoint: marker?.marker ?? null,
          modelYear: marker?.modelYearFrom ?? decoded.modelYear,
          powertrain: null,
        }));
        setSurvey(null);
      }
    } catch (error) {
      setVinDecode({
        vin,
        valid: false,
        message: `VIN decoding failed: ${error instanceof Error ? error.message : String(error)}`,
        decodeModel: null,
        candidates: [],
        program: null,
        modelYear: null,
        attributes: [],
        tableVersion: null,
      });
    } finally {
      setDecodingVin(false);
    }
  }, [catalogue, client, vin]);

  return {
    library,
    catalogue,
    vin,
    setVin,
    vinDecode,
    decodingVin,
    decodeVin,
    directory,
    setDirectory,
    chooseDirectory,
    vehicle,
    setVehicle,
    survey,
    busy,
    load,
    runSurvey,
  };
}
