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
  const [directory, setDirectory] = useState("");
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
      setLibrary(await client.loadDirectory(directory));
      // A different library can answer differently; do not keep an old answer.
      setSurvey(null);
      setCatalogue(await client.getCatalogue());
    } catch (error) {
      setLibrary(failedLibrary(error));
    } finally {
      setBusy(false);
    }
  }, [client, directory]);

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
