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
  /** The test's identifier in the ODST pack: `202` for the self test; empty for a reset. */
  testId: string;
  /**
   * A reset of the reviewed table (ADR-0036, amendment of 2026-11-28): the
   * routine identifier as the module's index writes it, `0x4013`. Absent
   * for the self test.
   */
  routine?: string | null;
  context: VehicleDescription;
}

export interface RoutineRunSnapshot {
  state: RoutineRunState;
  ecuFamily: string;
  /** `0x0202` for the self test, the reset's identifier otherwise. */
  routine: string;
  testId: string;
  /** SDD's own name for the test or the reset. */
  testName: string;
  /** What the routine does, by the reviewed table: SELF_TEST, LEARNED_VALUES, COUNTER or RECORD. */
  kind: string;
  /** Whether the data gives the routine a screen with a time; a reset has none, and the start's answer is its result. */
  screen: boolean;
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
  kind: "SELF_TEST",
  screen: true,
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
    const reset = request.routine ?? null;
    const digits = reset === null ? "02 02" : `${reset.slice(2, 4)} ${reset.slice(4, 6)}`;
    this.last = {
      ...createRoutineRunSnapshot(),
      state: "RUNNING",
      ecuFamily: request.ecuFamily,
      routine: reset ?? "0x0202",
      testId: request.testId,
      testName: reset === null ? `ODST_0202_${request.ecuFamily}_HLP` : "Reset All Adaptions",
      kind: reset === null ? "SELF_TEST" : "LEARNED_VALUES",
      screen: reset === null,
      routeId: "hs-can",
      routeValidation: "BROWSER_PREVIEW",
      session: "0x03",
      timeMs: reset === null ? 3000 : 0,
      timeoutMs: reset === null ? 5000 : 0,
      startedUnixMs: this.startedAt,
      resultHex: reset === null ? null : "00",
      exchanges: [
        ["10 03", "50 03 00 32 01 F4"],
        [`31 01 ${digits}`, reset === null ? `71 01 ${digits}` : `71 01 ${digits} 00`],
      ],
    };
    return Promise.resolve(this.last);
  }

  step() {
    if (this.last.state !== "RUNNING") return Promise.resolve(this.last);
    if (!this.last.screen) {
      // A reset: the start's answer was the result, the request for results
      // is refused as out of sequence, and the module is brought back.
      const digits = `${this.last.routine.slice(2, 4)} ${this.last.routine.slice(4, 6)}`;
      this.last = {
        ...this.last,
        state: "COMPLETED",
        elapsedMs: Date.now() - this.startedAt,
        refusal: "RequestSequenceError",
        codesAfter: [],
        codesFound: [],
        exchanges: [
          ...this.last.exchanges,
          ["3E 00", "7E 00"],
          [`31 03 ${digits}`, "7F 31 24"],
          ["10 01", "50 01 00 32 01 F4"],
        ],
        reportAvailable: true,
      };
      return Promise.resolve(this.last);
    }
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
