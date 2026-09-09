import { invoke } from "@tauri-apps/api/core";
import { demoCatalogue, demoSurvey } from "./demoVehicle";
import { browserDemoEnabled } from "./adapter";

export type LibraryState = "NOT_LOADED" | "LOADED" | "PARTIALLY_LOADED" | "FAILED";

export interface ManifestFailure {
  file: string;
  message: string;
}

export interface LibrarySnapshot {
  state: LibraryState;
  directory: string | null;
  manifestsLoaded: number;
  manifestsFailed: number;
  sources: number;
  records: number;
  failures: ManifestFailure[];
  message: string;
  /** Whose copy this is, when the owner stamped it; null for an unstamped library. */
  issue: LibraryIssue | null;
}

export type LibraryIssueIntegrity =
  | "MATCHES"
  | "MISMATCH"
  | "STAMP_REMOVED"
  | "NO_STAMP"
  | "UNSIGNED"
  | "BAD_SIGNATURE"
  | "EXPIRED";

export interface LibraryIssue {
  issuedTo: string;
  issuedOn: string;
  issueCode: string;
  /** Inclusive last day of validity, YYYY-MM-DD; empty on old stamps. */
  validUntil: string;
  /** Days from today to validUntil; negative once expired or unknown. */
  daysLeft: number;
  issuer: string;
  /** MATCHES is the only value under which the library was loaded. */
  integrity: LibraryIssueIntegrity;
}

/** What the user states about the car, in the terms SDD qualifies data by. */
export interface VehicleDescription {
  vehicleProgram: string;
  modelYear: number | null;
  powertrain: string | null;
  variant: string | null;
  market: string | null;
  yearBreakpoint: string | null;
}

export type RouteStatus =
  | "REACHABLE"
  | "HYPOTHESIS"
  | "INDETERMINATE"
  | "CONFLICT"
  | "NOT_APPLICABLE";

export type ModuleApplicability =
  | "APPLICABLE"
  | "INSUFFICIENT_CONTEXT"
  | "INSUFFICIENT_EVIDENCE";

export interface RouteSummary {
  status: RouteStatus;
  reasons: string[];
}

export interface ReadableIdentifierSummary {
  identifier: string;
  parameters: string[];
}

export interface ModuleSurveyEntry {
  ecuFamily: string;
  /** SDD's own name for the module family in English, when the loaded data has one. */
  name: string | null;
  /** The same name in every language the loaded data has, by SDD's code (`eng`, `rus`, …). */
  names: Record<string, string>;
  applicability: ModuleApplicability;
  logicalNetwork: string | null;
  requestId: string | null;
  responseId: string | null;
  backendRoute: string | null;
  pins: string | null;
  bitrateBps: number | null;
  protocol: string | null;
  routeValidation: string;
  identifierRead: RouteSummary;
  dtcRead: RouteSummary;
  readableIdentifiers: ReadableIdentifierSummary[];
}

export interface VehicleSurveySnapshot {
  context: VehicleDescription;
  modules: ModuleSurveyEntry[];
  reachable: number;
  hypothesis: number;
  unreachable: number;
  message: string;
}

export interface MarkerEntry {
  marker: string;
  modelYearFrom: number | null;
  modelYearTo: number | null;
}

export interface ProgrammeEntry {
  program: string;
  markers: MarkerEntry[];
  powertrains: string[];
  /** What SDD splits an engine into, where it does; empty for most programmes. */
  variants: string[];
}

export interface VehicleCatalogueSnapshot {
  programmes: ProgrammeEntry[];
}

export interface DecodedAttribute {
  name: string;
  value: string;
}

/** What SDD's VIN tables say about a VIN, and what they do not. */
export interface VinDecodeSnapshot {
  vin: string;
  valid: boolean;
  message: string;
  decodeModel: number | null;
  candidates: number[];
  program: string | null;
  modelYear: number | null;
  attributes: DecodedAttribute[];
  tableVersion: string | null;
}

export interface LibraryClient {
  getLibrary(): Promise<LibrarySnapshot>;
  loadDirectory(directory: string): Promise<LibrarySnapshot>;
  getCatalogue(): Promise<VehicleCatalogueSnapshot>;
  surveyVehicle(vehicle: VehicleDescription): Promise<VehicleSurveySnapshot>;
  decodeVin(vin: string): Promise<VinDecodeSnapshot>;
}

export const createLibrarySnapshot = (): LibrarySnapshot => ({
  state: "NOT_LOADED",
  directory: null,
  manifestsLoaded: 0,
  manifestsFailed: 0,
  sources: 0,
  records: 0,
  failures: [],
  issue: null,
  message: "Built-in data only.",
});

export const createVehicleDescription = (): VehicleDescription => ({
  vehicleProgram: "",
  modelYear: null,
  powertrain: null,
  variant: null,
  market: null,
  yearBreakpoint: null,
});

class TauriLibraryClient implements LibraryClient {
  getLibrary() {
    return invoke<LibrarySnapshot>("get_data_library");
  }

  loadDirectory(directory: string) {
    return invoke<LibrarySnapshot>("load_data_library", { directory });
  }

  getCatalogue() {
    return invoke<VehicleCatalogueSnapshot>("get_vehicle_catalogue");
  }

  surveyVehicle(vehicle: VehicleDescription) {
    return invoke<VehicleSurveySnapshot>("survey_vehicle", { context: vehicle });
  }

  decodeVin(vin: string) {
    return invoke<VinDecodeSnapshot>("decode_vin", { vin });
  }
}

/** The browser preview has no file system and no data; it says so. */
class BrowserLibraryClient implements LibraryClient {
  getLibrary(): Promise<LibrarySnapshot> {
    return Promise.resolve({
      ...createLibrarySnapshot(),
      message: browserDemoEnabled()
        ? "Simulator UI preview: the survey below shows synthetic modules only."
        : "Loading a data directory needs the desktop application.",
    });
  }

  loadDirectory(): Promise<LibrarySnapshot> {
    return Promise.resolve({
      ...createLibrarySnapshot(),
      state: "FAILED",
      message: "Loading a data directory needs the desktop application.",
    });
  }

  getCatalogue(): Promise<VehicleCatalogueSnapshot> {
    if (!browserDemoEnabled()) return Promise.resolve({ programmes: [] });
    return Promise.resolve(demoCatalogue);
  }

  surveyVehicle(vehicle: VehicleDescription): Promise<VehicleSurveySnapshot> {
    if (!browserDemoEnabled()) {
      return Promise.resolve({
        context: vehicle,
        modules: [],
        reachable: 0,
        hypothesis: 0,
        unreachable: 0,
        message: "Surveying a vehicle needs the desktop application.",
      });
    }
    return Promise.resolve(demoSurvey(vehicle));
  }

  decodeVin(vin: string): Promise<VinDecodeSnapshot> {
    const normalised = vin.replace(/[\s-]/g, "").toUpperCase();
    if (!browserDemoEnabled() || !normalised.startsWith("SYN")) {
      return Promise.resolve({
        vin: normalised,
        valid: normalised.length === 17,
        message: "Decoding a VIN needs the desktop application and its data.",
        decodeModel: null,
        candidates: [],
        program: null,
        modelYear: null,
        attributes: [],
        tableVersion: null,
      });
    }
    return Promise.resolve({
      vin: normalised,
      valid: true,
      message: "Synthetic preview: SDD's VIN tables read this as SYNTHA, model year 2010.",
      decodeModel: 1,
      candidates: [1],
      program: "SYNTHA",
      modelYear: 2010,
      attributes: [
        { name: "Brand", value: "Synthetic" },
        { name: "Model", value: "SYNTHA" },
        { name: "ModelYear", value: "2010" },
      ],
      tableVersion: "Synthetic VIN chart",
    });
  }
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultLibraryClient: LibraryClient = hasTauriRuntime()
  ? new TauriLibraryClient()
  : new BrowserLibraryClient();

/** The model years a catalogue marker stands for, as shown to the user. */
export function markerLabel(marker: { marker: string; modelYearFrom: number | null; modelYearTo: number | null }) {
  if (marker.modelYearFrom === null) return marker.marker;
  const to =
    marker.modelYearTo !== null && marker.modelYearTo !== marker.modelYearFrom
      ? `–${marker.modelYearTo}`
      : "";
  return `${marker.modelYearFrom}${to} (${marker.marker})`;
}
