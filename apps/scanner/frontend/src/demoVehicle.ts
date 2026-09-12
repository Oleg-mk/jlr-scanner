import type {
  ModuleSurveyEntry,
  SelfTestSummary,
  VehicleCatalogueSnapshot,
  VehicleDescription,
  VehicleSurveySnapshot,
} from "./library";
import type { DtcSummary, ModuleReadRequest, ModuleReadSnapshot } from "./moduleRead";

/**
 * The browser demo's vehicle: synthetic, but shaped like a real survey so
 * the interface can be judged — a two-bus car with a gatewayed sub-network
 * for MY10, and the 2014-and-later layout with hypothesised buses for MY14.
 * Nothing here is vehicle evidence; the programme is called SYNTHA on
 * purpose.
 */

interface Bus {
  logicalNetwork: string;
  backendRoute: string | null;
  pins: string | null;
  bitrateBps: number | null;
  routeValidation: string;
  status: "REACHABLE" | "HYPOTHESIS" | "INDETERMINATE";
  reasons: string[];
}

const HYPOTHESIS_REASON =
  "the adapter route for this bus is an unverified hypothesis (ADR-0015); a first read-only request confirms or refutes it";
const SUB_NETWORK_REASONS = [
  "adapter route: no applicable evidence-backed value",
  "connector pins: no applicable evidence-backed value",
];

const HS: Bus = {
  logicalNetwork: "CAN_HS",
  backendRoute: "hs-can",
  pins: "6/14",
  bitrateBps: 500_000,
  routeValidation: "SOURCE_BACKED",
  status: "REACHABLE",
  reasons: [],
};
const MS: Bus = { ...HS, logicalNetwork: "CAN_MS", backendRoute: "ms-can", pins: "3/11", bitrateBps: 125_000 };
const MOST: Bus = {
  logicalNetwork: "SUB_MOST",
  backendRoute: null,
  pins: null,
  bitrateBps: null,
  routeValidation: "",
  status: "INDETERMINATE",
  reasons: SUB_NETWORK_REASONS,
};
const hypothesised = (name: string, base: Bus): Bus => ({
  ...base,
  logicalNetwork: name,
  routeValidation: "UNVERIFIED",
  status: "HYPOTHESIS",
  reasons: [HYPOTHESIS_REASON],
});
const PT_HSCAN = hypothesised("PT_HSCAN", HS);
const CH_HSCAN = hypothesised("CH_HSCAN", HS);
const CO_HSCAN = hypothesised("CO_HSCAN", HS);
const BO_MSCAN = hypothesised("BO_MSCAN", MS);
const NGI: Bus = { ...MOST, logicalNetwork: "NGI" };

type Row = [string, string | null, string | null, Bus, string | null, string | null];

const NAMES: Record<string, [string, string | null]> = {
  PCM: ["Powertrain control module", "Модуль управления силовым агрегатом"],
  TCM: ["Transmission control module", "Модуль управления коробкой передач"],
  ABS: ["Anti-lock brake system module", "Модуль антиблокировочной системы"],
  RCM: ["Restraints control module", "Модуль управления системами безопасности"],
  PSCM: ["Power steering control module", "Модуль управления усилителем руля"],
  IPC: ["Instrument cluster", "Панель приборов"],
  SASM: ["Steering angle sensor module", null],
  PBM: ["Parking brake module", "Модуль стояночного тормоза"],
  BCM: ["Body control module", "Модуль управления кузовом"],
  HVAC: ["Climate control module", "Модуль климат-контроля"],
  DDM: ["Driver door module", "Модуль двери водителя"],
  PDM: ["Passenger door module", "Модуль двери пассажира"],
  TPM: ["Tyre pressure monitor", "Модуль контроля давления в шинах"],
  PAM: ["Parking aid module", "Модуль парковочного ассистента"],
  AAM: ["Audio amplifier module", "Аудиоусилитель"],
  TEL: ["Telematics module", "Телематический модуль"],
  IMC: ["Infotainment master controller", "Главный контроллер информационно-развлекательной системы"],
  GWM: ["Gateway module", "Модуль шлюза"],
  ATCM: ["All terrain control module", null],
  ADCM: ["Adaptive damping control module", null],
  AHU: ["Audio head unit", null],
  DRDM: ["Driver rear door module", null],
  DSM: ["Driver seat module", "Модуль сиденья водителя"],
};

const MY10_ROWS: Row[] = [
  ["PCM", "0x7E0", "0x7E8", HS, "0xF190", "VIN"],
  ["TCM", "0x7E1", "0x7E9", HS, "0xF187", "Part number"],
  ["ABS", "0x760", "0x768", HS, "0xF190", "VIN"],
  ["RCM", "0x737", "0x73F", HS, null, null],
  ["PSCM", "0x730", "0x738", HS, null, null],
  ["IPC", "0x720", "0x728", HS, "0xF190", "VIN"],
  ["SASM", "0x797", "0x79F", HS, null, null],
  ["PBM", "0x7B1", "0x7B9", HS, null, null],
  ["BCM", "0x726", "0x72E", MS, "0xF190", "VIN"],
  ["HVAC", "0x733", "0x73B", MS, null, null],
  ["DDM", "0x740", "0x748", MS, null, null],
  ["PDM", "0x741", "0x749", MS, null, null],
  ["TPM", "0x751", "0x759", MS, null, null],
  ["PAM", "0x736", "0x73E", MS, null, null],
  ["AAM", null, null, MOST, null, null],
  ["TEL", null, null, MOST, null, null],
  ["IMC", null, null, MOST, null, null],
];

const MY14_ROWS: Row[] = [
  ["PCM", "0x7E0", "0x7E8", PT_HSCAN, "0xF190", "VIN"],
  ["TCM", "0x7E1", "0x7E9", PT_HSCAN, null, null],
  ["ABS", "0x760", "0x768", PT_HSCAN, null, null],
  ["GWM", "0x716", "0x71E", PT_HSCAN, "0xF190", "VIN"],
  ["IPC", "0x720", "0x728", PT_HSCAN, null, null],
  ["BCM", "0x726", "0x72E", PT_HSCAN, null, null],
  ["RCM", "0x737", "0x73F", PT_HSCAN, null, null],
  ["ATCM", "0x792", "0x79A", CH_HSCAN, null, null],
  ["ADCM", "0x786", "0x78E", CH_HSCAN, null, null],
  ["PSCM", "0x730", "0x738", CH_HSCAN, null, null],
  ["HVAC", "0x733", "0x73B", CO_HSCAN, null, null],
  ["IMC", "0x7B3", "0x7BB", CO_HSCAN, null, null],
  ["AHU", "0x727", "0x72F", CO_HSCAN, null, null],
  ["DDM", "0x740", "0x748", BO_MSCAN, null, null],
  ["PDM", "0x741", "0x749", BO_MSCAN, null, null],
  ["DRDM", "0x742", "0x74A", BO_MSCAN, null, null],
  ["DSM", "0x744", "0x74C", BO_MSCAN, null, null],
  ["AAM", null, null, NGI, null, null],
];

/**
 * Synthetic self tests for the demo (ADR-0032). They are listed and never
 * run — in the demo there is nothing to run them on — and they exist here
 * only so the section has something to show.
 */
const DEMO_SELF_TESTS: Record<string, SelfTestSummary[]> = {
  ABS: [
    {
      testId: "1",
      name: "Synthetic pump motor self test",
      timeMs: 12_000,
      timeoutMs: 30_000,
      description: [
        "A demonstration entry, not a procedure: nothing here was taken from a vehicle.",
        "The real list comes from the loaded library, module by module.",
      ],
      descriptionTexts: {},
      modelYears: ["MY10"],
      safetyClass: "SERVICE_ROUTINE",
    },
  ],
  RCM: [
    {
      testId: "2",
      name: "Synthetic lamp check",
      timeMs: 4_000,
      timeoutMs: 10_000,
      description: [],
      descriptionTexts: {},
      modelYears: [],
      safetyClass: "SERVICE_ROUTINE",
    },
  ],
};

function entry([family, requestId, responseId, bus, identifier, parameter]: Row): ModuleSurveyEntry {
  const [eng, rus] = NAMES[family] ?? [family, null];
  const names: Record<string, string> = { eng };
  if (rus) names.rus = rus;
  const summary = { status: bus.status, reasons: bus.reasons };
  return {
    ecuFamily: family,
    name: eng,
    names,
    applicability: "APPLICABLE",
    logicalNetwork: bus.logicalNetwork,
    requestId,
    responseId,
    backendRoute: bus.backendRoute,
    pins: bus.pins,
    bitrateBps: bus.bitrateBps,
    protocol: bus.status === "INDETERMINATE" ? null : "ISO14229",
    routeValidation: bus.routeValidation,
    identifierRead: summary,
    dtcRead: summary,
    readableIdentifiers:
      identifier && parameter ? [{ identifier, parameters: [parameter] }] : [],
    selfTests: DEMO_SELF_TESTS[family] ?? [],
  };
}

export const demoCatalogue: VehicleCatalogueSnapshot = {
  programmes: [
    {
      program: "SYNTHA",
      markers: [
        { marker: "MY10", modelYearFrom: 2010, modelYearTo: 2013 },
        { marker: "MY14", modelYearFrom: 2014, modelYearTo: 2016 },
      ],
      powertrains: ["SYNTHENGINE", "5.0L Supercharged"],
      variants: [],
    },
  ],
};

export function demoSurvey(vehicle: VehicleDescription): VehicleSurveySnapshot {
  const rows = vehicle.yearBreakpoint === "MY14" ? MY14_ROWS : MY10_ROWS;
  const modules = rows.map(entry);
  const reachable = modules.filter((module) => module.dtcRead.status === "REACHABLE").length;
  const hypothesis = modules.filter((module) => module.dtcRead.status === "HYPOTHESIS").length;
  const unreachable = modules.length - reachable - hypothesis;
  return {
    context: vehicle,
    modules,
    reachable,
    hypothesis,
    unreachable,
    message: `Synthetic preview: ${modules.length} modules known, ${reachable} reachable over the adapter, ${hypothesis} on a hypothesised route, ${unreachable} not — each with the reason.`,
  };
}

const failureType = {
  eng: "General failure information - no sub type information.",
  rus: "Сведения об общей неисправности - нет сведений о подтипе.",
};

function dtc(
  code: string,
  description: string,
  descriptionTexts: Record<string, string> = {},
): DtcSummary {
  return {
    code,
    failureType: "00",
    status: "08",
    description,
    descriptionScope: "module",
    failureTypeText: failureType.eng,
    failureTypeTexts: failureType,
    descriptionTexts,
    help: [],
    helpTexts: {},
    helpNote: null,
  };
}

// The two standard codes below carry our own wording, copied from the
// application's table so that the browser preview shows what the desktop
// build shows; the third has none, and falls back to English, which is also
// what a manufacturer-specific code does.
const FAULTS: Record<string, DtcSummary[]> = {
  PCM: [
    dtc("P0301", "Cylinder 1 misfire detected", {
      ukr: "Пропуски запалювання в циліндрі 1",
      rus: "Пропуски зажигания в цилиндре 1",
    }),
    dtc("P0171", "System too lean (bank 1)", {
      ukr: "Надто бідна суміш (ряд 1)",
      rus: "Слишком бедная смесь (ряд 1)",
    }),
  ],
  ABS: [dtc("C0035", "Left front wheel speed sensor circuit")],
  BCM: [dtc("B1318", "Battery voltage low")],
  DDM: [dtc("B1D01", "Window motor circuit")],
};
const SILENT = new Set(["RCM", "ADCM", "PBM"]);
const DECLINED = new Set(["HVAC"]);

/** What the demo car answers: faults on a few modules, silence on some, one refusal. */
export function demoReadOutcome(request: ModuleReadRequest, base: ModuleReadSnapshot): ModuleReadSnapshot {
  const family = request.ecuFamily;
  const row = [...MY10_ROWS, ...MY14_ROWS].find(([candidate]) => candidate === family);
  const bus = row?.[3] ?? HS;
  const responder = row?.[2] ?? "0x7E8";
  const faultCodes = request.kind === "FAULT_CODES";
  if (SILENT.has(family)) {
    return {
      ...base,
      state: "FAILED",
      ecuFamily: family,
      operation: faultCodes ? "Read confirmed fault codes" : `Read identifier ${request.identifier ?? ""}`,
      routeId: bus.backendRoute ?? "",
      routeValidation: bus.routeValidation,
      requestHex: faultCodes ? "19 02 08" : "22 F1 90",
      error: {
        category: "NO_RESPONSE_FROM_ECU",
        message: "No response within 1000 ms",
        technicalDetails: null,
        stage: "RESPONSE",
      },
      reportAvailable: true,
    };
  }
  const declined = DECLINED.has(family);
  return {
    ...base,
    state: "SUCCEEDED",
    ecuFamily: family,
    operation: faultCodes ? "Read confirmed fault codes" : `Read identifier ${request.identifier ?? ""}`,
    routeId: bus.backendRoute ?? "",
    routeValidation: bus.routeValidation,
    requestHex: faultCodes ? "19 02 08" : "22 F1 90",
    responder,
    rawResponseHex: declined ? "7F 19 12" : faultCodes ? "59 02 FF 03 01 00 08" : "62 F1 90 53 41 4C",
    dataHex: declined || faultCodes ? null : "53 41 4C",
    parameters:
      declined || faultCodes
        ? []
        : [{ name: "VIN", raw: 0, value: "SAL…", unit: null, state: null, note: null }],
    dtcs: declined || !faultCodes ? [] : (FAULTS[family] ?? []),
    negativeResponse: declined ? "0x12 sub-function not supported" : null,
    pendingResponses: 0,
    error: null,
    reportAvailable: true,
  };
}
