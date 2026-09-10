import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { DiagnosticError } from "./diagnostic";
import { hasTauriRuntime } from "./files";
import type { VehicleDescription } from "./library";

/**
 * Live reading (ADR-0022): the same read-only reads, repeated until stopped.
 * The set is modules and identifiers the loaded library lists; the shell owns
 * the floor, the cap and the failure counting, and this side owns only the
 * timer that asks for the next step.
 */
export type LiveReadState = "IDLE" | "RUNNING" | "STOPPED";

export interface LiveReadEntryRequest {
  ecuFamily: string;
  identifier: string;
}

export interface LiveReadRequest {
  entries: LiveReadEntryRequest[];
  context: VehicleDescription;
}

export interface LiveReadEntryStatus {
  ecuFamily: string;
  identifier: string;
  routeId: string;
  routeValidation: string;
  reads: number;
  failures: number;
  dropped: boolean;
  reason: string | null;
  lastResponseHex: string | null;
  negativeResponse: string | null;
}

export interface LiveReadValue {
  ecuFamily: string;
  identifier: string;
  name: string;
  value: string | null;
  unit: string | null;
  state: string | null;
  note: string | null;
  raw: number | null;
  minimum: number | null;
  maximum: number | null;
  samples: number;
  atMs: number;
}

export interface LiveReadSnapshot {
  state: LiveReadState;
  entries: LiveReadEntryStatus[];
  values: LiveReadValue[];
  rounds: number;
  samples: number;
  elapsedMs: number;
  roundMs: number | null;
  cadenceFloorMs: number;
  timeCapMs: number;
  routeValidation: string;
  stoppedReason: string | null;
  error: DiagnosticError | null;
  reportAvailable: boolean;
}

export interface LiveReadClient {
  getState(): Promise<LiveReadSnapshot>;
  start(request: LiveReadRequest): Promise<LiveReadSnapshot>;
  step(): Promise<LiveReadSnapshot>;
  stop(): Promise<LiveReadSnapshot>;
}

/** At most sixteen entries in a set; the shell refuses the rest by the same rule. */
export const LIVE_READ_MAX_ENTRIES = 16;

export const createLiveReadSnapshot = (): LiveReadSnapshot => ({
  state: "IDLE",
  entries: [],
  values: [],
  rounds: 0,
  samples: 0,
  elapsedMs: 0,
  roundMs: null,
  cadenceFloorMs: 100,
  timeCapMs: 600_000,
  routeValidation: "",
  stoppedReason: null,
  error: null,
  reportAvailable: false,
});

/** `SYNTHMOD 0x1945`: an entry named the way the set lists it. */
export function entryKey(entry: LiveReadEntryRequest): string {
  return `${entry.ecuFamily} ${entry.identifier}`;
}

class TauriLiveReadClient implements LiveReadClient {
  getState() {
    return invoke<LiveReadSnapshot>("get_live_read_state");
  }

  start(request: LiveReadRequest) {
    return invoke<LiveReadSnapshot>("start_live_read", { request });
  }

  step() {
    return invoke<LiveReadSnapshot>("live_read_step");
  }

  stop() {
    return invoke<LiveReadSnapshot>("stop_live_read");
  }
}

/** A wandering number for the browser preview; nothing behind it. */
class BrowserLiveReadClient implements LiveReadClient {
  private snapshot = createLiveReadSnapshot();
  private startedAt = 0;
  private lastStepAt = 0;

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start(request: LiveReadRequest) {
    if (!browserDemoEnabled()) {
      this.snapshot = {
        ...createLiveReadSnapshot(),
        error: {
          category: "ADAPTER_NOT_FOUND",
          message: "Adapter not found",
          technicalDetails: "connect and verify the adapter before a live read",
          stage: "ADAPTER_VALIDATION",
        },
      };
      return Promise.resolve(this.snapshot);
    }
    this.startedAt = Date.now();
    this.lastStepAt = 0;
    this.snapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      routeValidation: "DEMO",
      entries: request.entries.slice(0, LIVE_READ_MAX_ENTRIES).map((entry) => ({
        ecuFamily: entry.ecuFamily,
        identifier: entry.identifier,
        routeId: "hs-can",
        routeValidation: "DEMO",
        reads: 0,
        failures: 0,
        dropped: false,
        reason: null,
        lastResponseHex: null,
        negativeResponse: null,
      })),
    };
    return Promise.resolve(this.snapshot);
  }

  step() {
    if (this.snapshot.state !== "RUNNING" || this.snapshot.entries.length === 0) {
      return Promise.resolve(this.snapshot);
    }
    const now = Date.now();
    if (now - this.lastStepAt < this.snapshot.cadenceFloorMs) return Promise.resolve(this.snapshot);
    this.lastStepAt = now;
    const atMs = now - this.startedAt;
    const index = this.snapshot.samples % this.snapshot.entries.length;
    const entry = this.snapshot.entries[index];
    // A number that wanders the way a live one does. The preview cannot know
    // what an identifier means, so it does not pretend to: the parameter is
    // named after the identifier and says where the number came from.
    const wandering = Math.round(748 + Math.sin(atMs / 900) * 22);
    const value: LiveReadValue = {
      ecuFamily: entry.ecuFamily,
      identifier: entry.identifier,
      name: `parameter ${entry.identifier}`,
      value: String(wandering),
      unit: null,
      state: null,
      note: "browser preview only",
      raw: wandering,
      minimum: Math.min(wandering, this.snapshot.values[index]?.minimum ?? wandering),
      maximum: Math.max(wandering, this.snapshot.values[index]?.maximum ?? wandering),
      samples: (this.snapshot.values[index]?.samples ?? 0) + 1,
      atMs,
    };
    const values = [...this.snapshot.values];
    values[index] = value;
    const entries = this.snapshot.entries.map((kept, at) =>
      at === index ? { ...kept, reads: kept.reads + 1, lastResponseHex: "62 00 00" } : kept,
    );
    this.snapshot = {
      ...this.snapshot,
      entries,
      values,
      samples: this.snapshot.samples + 1,
      rounds: Math.floor((this.snapshot.samples + 1) / this.snapshot.entries.length),
      elapsedMs: atMs,
      roundMs: this.snapshot.entries.length * 150,
      reportAvailable: true,
    };
    return Promise.resolve(this.snapshot);
  }

  stop() {
    this.snapshot = {
      ...this.snapshot,
      state: this.snapshot.state === "RUNNING" ? "STOPPED" : this.snapshot.state,
      stoppedReason: this.snapshot.state === "RUNNING" ? "stopped by the tester" : null,
    };
    return Promise.resolve(this.snapshot);
  }
}

export const defaultLiveReadClient: LiveReadClient = hasTauriRuntime()
  ? new TauriLiveReadClient()
  : new BrowserLiveReadClient();
