import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { VehicleDescription } from "./library";
import type { DtcSummary } from "./moduleRead";

/**
 * A run of a routine a module declares (ADR-0036, step 2): the on-demand
 * self test, `0x0202`. The shell owns the run - the session it opens and
 * holds, the keep-alive between steps, the request for results once the
 * data's time has run, the way back - and the interface steps it, one
 * request at a time, as it steps a live read. The result is the bytes the
 * module answered with; nothing is read into them.
 */
export type RoutineRunState =
  | "IDLE"
  | "RUNNING"
  | "COMPLETED"
  | "REFUSED"
  | "STOPPED"
  | "TIMED_OUT"
  | "FAILED";

export interface RoutineRunRequest {
  ecuFamily: string;
  /** The test's identifier in the ODST pack: `202` for the self test. */
  testId: string;
  context: VehicleDescription;
}

export interface RoutineRunSnapshot {
  state: RoutineRunState;
  ecuFamily: string;
  /** `0x0202`. */
  routine: string;
  testId: string;
  /** SDD's own name for the test. */
  testName: string;
  /** `ROUTINE_RUN`. */
  operation: string;
  /** `SERVICE_ROUTINE`. */
  safetyClass: string;
  routeId: string;
  routeValidation: string;
  session: string | null;
  /** How long the data says the test runs, and the longest the tool waits. */
  timeMs: number;
  timeoutMs: number;
  startedUnixMs: number | null;
  elapsedMs: number;
  /** The routine status record the module answered with, as bytes. */
  resultHex: string | null;
  refusal: string | null;
  /** The codes the module held before the run, as read in this session. */
  codesBefore: DtcSummary[];
  /** What the module answered when read again once the run had ended; null until then. */
  codesAfter: DtcSummary[] | null;
  /** The codes after the run that were not there before: what the test logged. */
  codesFound: DtcSummary[];
  /** Every exchange so far, request then answer, in hex. */
  exchanges: [string, string][];
  error: {
    category: string;
    message: string;
    technicalDetails: string | null;
    stage: string;
  } | null;
  reportAvailable: boolean;
}

export interface RoutineRunClient {
  getState(): Promise<RoutineRunSnapshot>;
  start(request: RoutineRunRequest): Promise<RoutineRunSnapshot>;
  step(): Promise<RoutineRunSnapshot>;
  stop(): Promise<RoutineRunSnapshot>;
}

export const createRoutineRunSnapshot = (): RoutineRunSnapshot => ({
  state: "IDLE",
  ecuFamily: "",
  routine: "0x0202",
  testId: "",
  testName: "",
  operation: "ROUTINE_RUN",
  safetyClass: "SERVICE_ROUTINE",
  routeId: "",
  routeValidation: "",
  session: null,
  timeMs: 0,
  timeoutMs: 0,
  startedUnixMs: null,
  elapsedMs: 0,
  resultHex: null,
  refusal: null,
  codesBefore: [],
  codesAfter: null,
  codesFound: [],
  exchanges: [],
  error: null,
  reportAvailable: false,
});

class TauriRoutineRunClient implements RoutineRunClient {
  getState() {
    return invoke<RoutineRunSnapshot>("get_routine_run_state");
  }

  start(request: RoutineRunRequest) {
    return invoke<RoutineRunSnapshot>("start_routine_run", { request });
  }

  step() {
    return invoke<RoutineRunSnapshot>("routine_run_step");
  }

  stop() {
    return invoke<RoutineRunSnapshot>("stop_routine_run");
  }
}

/**
 * The browser preview has no shell and no module: a run starts, counts its
 * three seconds, and answers as a module that completed would, with nothing
 * behind it. Every value says it is a preview.
 */
class BrowserRoutineRunClient implements RoutineRunClient {
  private last: RoutineRunSnapshot = createRoutineRunSnapshot();
  private startedAt = 0;

  getState() {
    return Promise.resolve(this.last);
  }

  start(request: RoutineRunRequest) {
    this.startedAt = Date.now();
    this.last = {
      ...createRoutineRunSnapshot(),
      state: "RUNNING",
      ecuFamily: request.ecuFamily,
      testId: request.testId,
      testName: `ODST_0202_${request.ecuFamily}_HLP`,
      routeId: "hs-can",
      routeValidation: "BROWSER_PREVIEW",
      session: "0x03",
      timeMs: 3000,
      timeoutMs: 5000,
      startedUnixMs: this.startedAt,
      exchanges: [
        ["10 03", "50 03 00 32 01 F4"],
        ["31 01 02 02", "71 01 02 02"],
      ],
    };
    return Promise.resolve(this.last);
  }

  step() {
    if (this.last.state !== "RUNNING") return Promise.resolve(this.last);
    const elapsedMs = Date.now() - this.startedAt;
    if (elapsedMs < this.last.timeMs) {
      this.last = {
        ...this.last,
        elapsedMs,
        exchanges: [...this.last.exchanges, ["3E 00", "7E 00"]],
      };
    } else {
      this.last = {
        ...this.last,
        state: "COMPLETED",
        elapsedMs,
        resultHex: "00 A5 5A",
        codesAfter: [],
        codesFound: [],
        exchanges: [
          ...this.last.exchanges,
          ["3E 00", "7E 00"],
          ["31 03 02 02", "71 03 02 02 00 A5 5A"],
          ["10 01", "50 01 00 32 01 F4"],
        ],
        reportAvailable: true,
      };
    }
    return Promise.resolve(this.last);
  }

  stop() {
    if (this.last.state === "RUNNING") {
      this.last = {
        ...this.last,
        state: "STOPPED",
        elapsedMs: Date.now() - this.startedAt,
        exchanges: [
          ...this.last.exchanges,
          ["3E 00", "7E 00"],
          ["31 02 02 02", "71 02 02 02"],
          ["10 01", "50 01 00 32 01 F4"],
        ],
        reportAvailable: true,
      };
    }
    return Promise.resolve(this.last);
  }
}

export const defaultRoutineRunClient: RoutineRunClient = browserDemoEnabled()
  ? new BrowserRoutineRunClient()
  : new TauriRoutineRunClient();

/** The test identifiers the ODST pack gives the self test: the digits of `0x0202`. */
export const SELF_TEST_IDS = ["202", "0202"];

/** The routine the on-demand self test runs, as SDD's index writes it. */
export const SELF_TEST_ROUTINE = 0x0202;
