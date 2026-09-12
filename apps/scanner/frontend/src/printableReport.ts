import { invoke } from "@tauri-apps/api/core";
import { hasTauriRuntime, saveTextFile } from "./files";

/**
 * The readable report (ADR-0031): the session bundle turned into a document
 * a person reads, prints, or saves as a web page any browser turns into a
 * PDF.
 *
 * It is a rendering and not a record. The bundle stays the evidence; this
 * shows the same session in prose and tables, in the language the interface
 * is in, and says on every section what its values are worth.
 */

/** What the bundle holds, as far as the document reads it. */
export interface SessionBundle {
  application_version?: string;
  application_build?: string;
  session_mode?: string;
  bench_scenario?: number | null;
  session_started_unix_ms?: number;
  saved_unix_ms?: number;
  adapter?: {
    adapter?: {
      name?: string;
      port?: string;
      serialNumber?: string | null;
      backend?: string;
    } | null;
  } | null;
  library?: {
    state?: string;
    manifestsLoaded?: number;
    manifestsFailed?: number;
    sources?: number;
    records?: number;
    issue?: { issuedTo?: string; issueCode?: string; validUntil?: string } | null;
  } | null;
  survey?: {
    context?: VehicleContext;
    modules?: SurveyModule[];
    reachable?: number;
    hypothesis?: number;
    unreachable?: number;
    message?: string;
  } | null;
  captures?: unknown[];
  module_reads?: ModuleRead[];
  calibration_reads?: unknown[];
  standard_obd_reads?: unknown[];
  live_read_runs?: LiveRun[];
  mileage_surveys?: MileageSurvey[];
  module_passports?: PassportRun[];
  ccf_reads?: CcfRun[];
  battery_reads?: BatteryRun[];
}

interface VehicleContext {
  vehicleProgram?: string;
  modelYear?: number | null;
  powertrain?: string | null;
  variant?: string | null;
  market?: string | null;
  yearBreakpoint?: string | null;
}

interface SurveyModule {
  ecuFamily?: string;
  name?: string | null;
  logicalNetwork?: string | null;
  protocol?: string | null;
  requestId?: string | null;
  routeValidation?: string;
  identifierRead?: { status?: string; reasons?: string[] };
  dtcRead?: { status?: string; reasons?: string[] };
}

interface ModuleRead {
  ecu_family?: string;
  operation?: string;
  vehicle?: VehicleContext;
  route_validation?: string;
  decoded_result?: string | null;
  actual_responder?: string | null;
  timestamp_unix_ms?: number;
}

interface LiveRun {
  started_unix_ms?: number;
  rounds?: number;
  samples?: unknown[];
  set?: Array<{ ecu_family?: string; identifier?: string; reads?: number }>;
  route_validation?: string;
}

interface MileageSurvey {
  readings?: Array<{
    ecuFamily?: string;
    parameter?: string;
    value?: string | null;
    unit?: string | null;
    routeValidation?: string;
  }>;
  route_validation?: string;
}

interface PassportRun {
  readings?: Array<{
    ecuFamily?: string;
    identifier?: string;
    parameter?: string;
    value?: string | null;
    routeValidation?: string;
    /** What JLR's catalogue names for this number (ADR-0033). */
    catalogue?: CatalogueLine | null;
  }>;
  route_validation?: string;
}

interface CatalogueLine {
  state?: string;
  expected?: string | null;
  partType?: string | null;
  dated?: string | null;
}

interface CcfRun {
  readings?: Array<{
    ecuFamily?: string;
    parameter?: string;
    groupTitleEn?: string;
    titleEn?: string;
    valueEn?: string | null;
    display?: boolean;
    routeValidation?: string;
  }>;
  master_module?: string | null;
  route_validation?: string;
}

interface BatteryRun {
  readings?: Array<{
    ecuFamily?: string;
    identifier?: string;
    parameter?: string;
    role?: string;
    value?: string | null;
    unit?: string | null;
    routeValidation?: string;
  }>;
  route_validation?: string;
}

/** One table of the document: a heading, its rows, and what they are worth. */
export interface ReportSection {
  id: string;
  title: string;
  worth: string;
  columns: string[];
  rows: string[][];
  /** Said under the table when the section needs a caveat of its own. */
  note?: string;
}

export interface PrintableReport {
  bench: boolean;
  benchScenario: number | null;
  applicationVersion: string;
  applicationBuild: string;
  savedAt: string;
  vehicle: Array<[string, string]>;
  library: Array<[string, string]>;
  adapter: Array<[string, string]>;
  survey: ReportSection | null;
  sections: ReportSection[];
  /** Nothing recorded at all: the document says so rather than pretending. */
  empty: boolean;
}

/** The last four characters only, for a VIN or a serial the tester masks. */
export function maskTail(value: string): string {
  const text = value.trim();
  if (text.length <= 4) return text;
  return `${"•".repeat(Math.min(text.length - 4, 13))}${text.slice(-4)}`;
}

/**
 * One comparison in a printed row (ADR-0033). Four states and no verdict,
 * the same four the panel shows.
 */
function catalogueText(
  comparison: CatalogueLine | null | undefined,
  t: (key: string, values?: Record<string, string | number>) => string,
): string {
  if (!comparison) return "—";
  switch (comparison.state) {
    case "AGREES":
      return t("the same number");
    case "DIFFERS":
      return `${t("the catalogue names")} ${comparison.expected ?? ""}`.trim();
    case "NOT_NAMED":
      return t("the catalogue names no part here");
    case "NO_ASSEMBLY":
      return t("the catalogue does not carry this assembly");
    default:
      return "—";
  }
}

function stamp(unixMs: number | undefined): string {
  if (unixMs === undefined || unixMs === 0) return "—";
  return new Date(unixMs).toLocaleString();
}

function text(value: string | null | undefined, fallback = "—"): string {
  const trimmed = (value ?? "").toString().trim();
  return trimmed === "" ? fallback : trimmed;
}

/**
 * Build the document from the bundle. `mask` hides the VIN and the
 * adapter's serial in this rendering only: the bundle is untouched
 * (ADR-0031, decision 4).
 */
export function buildReport(
  bundle: SessionBundle,
  translate: (key: string, values?: Record<string, string | number>) => string,
  mask: boolean,
): PrintableReport {
  const t = translate;
  const bench = (bundle.session_mode ?? "real") === "bench";
  const worthOf = (value: string | undefined | null): string =>
    bench ? "SYNTHETIC" : text(value, "—");

  const context =
    bundle.survey?.context ?? bundle.module_reads?.[0]?.vehicle ?? ({} as VehicleContext);
  const vehicle: Array<[string, string]> = [
    [t("Programme"), text(context.vehicleProgram)],
    [t("Model year"), context.modelYear === null || context.modelYear === undefined ? "—" : String(context.modelYear)],
    [t("Breakpoint"), text(context.yearBreakpoint)],
    [t("Powertrain"), text(context.powertrain)],
    [t("Market"), text(context.market)],
  ];

  const issue = bundle.library?.issue ?? null;
  const library: Array<[string, string]> = [
    [t("Manifests"), String(bundle.library?.manifestsLoaded ?? 0)],
    [t("Records"), String(bundle.library?.records ?? 0)],
    [t("Sources"), String(bundle.library?.sources ?? 0)],
    [t("Issued to"), text(issue?.issuedTo)],
    [t("Issue code"), text(issue?.issueCode)],
    [t("Valid until"), text(issue?.validUntil)],
  ];

  const info = bundle.adapter?.adapter ?? null;
  const serial = text(info?.serialNumber);
  const adapter: Array<[string, string]> = [
    [t("Adapter"), text(info?.name)],
    [t("Port"), text(info?.port)],
    [t("Serial number"), serial === "—" ? "—" : mask ? maskTail(serial) : serial],
    [t("Backend"), text(info?.backend)],
  ];

  const survey: ReportSection | null =
    bundle.survey === null || bundle.survey === undefined
      ? null
      : {
          id: "survey",
          title: t("Modules"),
          worth: text(bundle.survey.message),
          columns: [t("Module"), t("Bus"), t("Protocol"), t("Address"), t("Identifier read"), t("Fault-code read")],
          rows: (bundle.survey.modules ?? []).map((module) => [
            `${text(module.ecuFamily)}${module.name ? ` — ${module.name}` : ""}`,
            text(module.logicalNetwork),
            text(module.protocol),
            text(module.requestId),
            statusWithReason(module.identifierRead),
            statusWithReason(module.dtcRead),
          ]),
        };

  const sections: ReportSection[] = [];

  const reads = bundle.module_reads ?? [];
  if (reads.length > 0) {
    sections.push({
      id: "module-reads",
      title: t("Module reads"),
      worth: worthOf(reads[0]?.route_validation),
      columns: [t("Module"), t("Operation"), t("Answer"), t("Responder"), t("When")],
      rows: reads.map((read) => [
        text(read.ecu_family),
        text(read.operation),
        text(read.decoded_result),
        text(read.actual_responder),
        stamp(read.timestamp_unix_ms),
      ]),
    });
  }

  for (const [index, run] of (bundle.module_passports ?? []).entries()) {
    const rows = run.readings ?? [];
    if (rows.length === 0) continue;
    sections.push({
      id: `passport-${index}`,
      title: t("Module passport"),
      worth: worthOf(run.route_validation),
      columns: [
        t("Module"),
        t("Identifier"),
        t("Parameter"),
        t("Value"),
        t("JLR's catalogue"),
      ],
      rows: rows.map((row) => [
        text(row.ecuFamily),
        text(row.identifier),
        text(row.parameter),
        text(row.value),
        catalogueText(row.catalogue, t),
      ]),
      note: t(
        "What each module says it is, as the text it holds. The last column is JLR's own part lineage as SDD carries it: a number that differs from it is a difference, not a fault.",
      ),
    });
  }

  for (const [index, run] of (bundle.mileage_surveys ?? []).entries()) {
    const rows = run.readings ?? [];
    if (rows.length === 0) continue;
    sections.push({
      id: `mileage-${index}`,
      title: t("Mileage"),
      worth: worthOf(run.route_validation),
      columns: [t("Module"), t("Parameter"), t("Value")],
      rows: rows.map((row) => [
        text(row.ecuFamily),
        text(row.parameter),
        `${text(row.value)}${row.unit ? ` ${row.unit}` : ""}`,
      ]),
      note: t("Every module that keeps a distance, as it keeps it. No verdict is drawn from a difference."),
    });
  }

  for (const [index, run] of (bundle.battery_reads ?? []).entries()) {
    const rows = (run.readings ?? []).filter((row) => row.value !== null && row.value !== undefined);
    if (rows.length === 0) continue;
    sections.push({
      id: `battery-${index}`,
      title: t("Battery"),
      worth: worthOf(run.route_validation),
      columns: [t("Module"), t("Parameter"), t("Value")],
      rows: rows.map((row) => [
        text(row.ecuFamily),
        text(row.parameter),
        `${text(row.value)}${row.unit ? ` ${row.unit}` : ""}`,
      ]),
      note: t(
        "What the modules hold, as they hold it. This application states no threshold of its own and says nothing about whether the battery is good.",
      ),
    });
  }

  for (const [index, run] of (bundle.ccf_reads ?? []).entries()) {
    const rows = (run.readings ?? []).filter((row) => row.display !== false);
    if (rows.length === 0) continue;
    sections.push({
      id: `ccf-${index}`,
      title: t("Configuration (CCF)"),
      worth: worthOf(run.route_validation),
      columns: [t("Group"), t("Setting"), t("Value"), t("Module")],
      rows: rows.map((row) => [
        text(row.groupTitleEn),
        text(row.titleEn === "" ? row.parameter : row.titleEn),
        text(row.valueEn),
        text(row.ecuFamily),
      ]),
      note: t("The values the modules hold; the rows SDD's own editor shows."),
    });
  }

  for (const [index, run] of (bundle.live_read_runs ?? []).entries()) {
    const set = run.set ?? [];
    if (set.length === 0) continue;
    sections.push({
      id: `live-${index}`,
      title: t("Live reading"),
      worth: worthOf(run.route_validation),
      columns: [t("Module"), t("Identifier"), t("Reads")],
      rows: set.map((entry) => [
        text(entry.ecu_family),
        text(entry.identifier),
        String(entry.reads ?? 0),
      ]),
      note: t("{samples} sample(s) over {rounds} round(s); the series itself is in the bundle and the spreadsheet.", {
        samples: (run.samples ?? []).length,
        rounds: run.rounds ?? 0,
      }),
    });
  }

  const captures = (bundle.captures ?? []).length;
  const standard = (bundle.standard_obd_reads ?? []).length;
  const calibrations = (bundle.calibration_reads ?? []).length;
  if (captures + standard + calibrations > 0) {
    sections.push({
      id: "counts",
      title: t("Also recorded"),
      worth: bench ? "SYNTHETIC" : "—",
      columns: [t("What"), t("Count")],
      rows: [
        [t("Bus captures"), String(captures)],
        [t("Standard OBD-II reads"), String(standard)],
        [t("Calibration reads"), String(calibrations)],
      ],
      note: t("Each is in the bundle whole, with its own bytes and its own validation."),
    });
  }

  return {
    bench,
    benchScenario: bundle.bench_scenario ?? null,
    applicationVersion: text(bundle.application_version),
    applicationBuild: text(bundle.application_build),
    savedAt: stamp(bundle.saved_unix_ms),
    vehicle,
    library,
    adapter,
    survey,
    sections,
    empty: sections.length === 0 && survey === null,
  };
}

function statusWithReason(
  summary: { status?: string; reasons?: string[] } | undefined,
): string {
  if (summary === undefined) return "—";
  const status = text(summary.status);
  const reason = (summary.reasons ?? [])[0];
  return reason === undefined ? status : `${status} — ${reason}`;
}

/**
 * Save the rendered document as one web page. Everything it needs travels
 * inside it — the styles are inlined by the caller — so the file opens and
 * prints anywhere, with no application behind it.
 */
export function saveReportHtml(html: string, stampMs: number): Promise<string | null> {
  return saveTextFile(`prowlone-report-${stampMs}.html`, html, "html");
}

/** Show a saved file in the system's file manager, so it can be attached. */
export async function revealInFolder(path: string): Promise<void> {
  if (!hasTauriRuntime()) return;
  await invoke("reveal_in_folder", { path });
}

/** The document's own styles, inside the document: a saved page carries
 *  them with it and prints the same anywhere (ADR-0031). */
export const REPORT_STYLES = `
.print-report { font-family: system-ui, -apple-system, "Segoe UI", sans-serif; color: #14161a; background: #fff; max-width: 46rem; margin: 0 auto; padding: 1.5rem; line-height: 1.45; }
.print-report h1 { font-size: 1.5rem; margin: 0 0 0.2rem; }
.print-report h2 { font-size: 1rem; margin: 1.3rem 0 0.4rem; border-bottom: 1px solid #d9dde3; padding-bottom: 0.2rem; display: flex; justify-content: space-between; align-items: baseline; gap: 1rem; }
.print-report .print-sub { margin: 0 0 0.8rem; font-size: 0.8rem; color: #5b6470; }
.print-report .print-worth { font-size: 0.7rem; font-weight: 400; color: #5b6470; letter-spacing: 0.03em; }
.print-report .print-bench { border: 2px solid #14161a; padding: 0.5rem 0.7rem; font-size: 0.8rem; margin: 0.6rem 0; }
.print-report .print-facts { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 0.15rem 1.2rem; margin: 0 0 0.6rem; }
.print-report .print-facts > div { display: flex; justify-content: space-between; gap: 0.6rem; border-bottom: 1px dotted #e3e6ea; padding: 0.12rem 0; }
.print-report .print-facts dt { color: #5b6470; font-size: 0.78rem; }
.print-report .print-facts dd { margin: 0; font-size: 0.78rem; text-align: right; }
.print-report table { width: 100%; border-collapse: collapse; font-size: 0.76rem; }
.print-report th { text-align: left; border-bottom: 1px solid #b9c0c9; padding: 0.25rem 0.35rem; color: #5b6470; font-weight: 500; }
.print-report td { border-bottom: 1px solid #eceff2; padding: 0.25rem 0.35rem; vertical-align: top; }
.print-report .print-note { font-size: 0.72rem; color: #5b6470; margin: 0.35rem 0 0; }
.print-report .print-foot { margin-top: 1.5rem; border-top: 1px solid #d9dde3; padding-top: 0.5rem; font-size: 0.72rem; color: #5b6470; }
.print-report .print-section { break-inside: auto; }
.print-report .print-section h2 { break-after: avoid; }
.print-report tr { break-inside: avoid; }
@media print {
  .print-report { max-width: none; padding: 0; }
}
`;
