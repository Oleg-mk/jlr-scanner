import { invoke } from "@tauri-apps/api/core";
import { browserDemoEnabled } from "./adapter";
import type { VehicleDescription } from "./library";
import type { DtcSummary } from "./moduleRead";
import type { SessionReportSnapshot } from "./sessionReport";

/**
 * The service mode and its first operation (ADR-0036, stage 2 step 1).
 *
 * The mode is a property of the session, kept by the shell: off on every
 * start and every new session, on after the person's one consent. While it
 * is on, the operations of stage 2 exist in the interface; the first of them
 * is the clear of a module's fault codes, offered for a module whose codes
 * were read in this session, after its own confirmation, and recorded with
 * what it erased and what the module answered afterwards.
 */

export type DtcClearState = "IDLE" | "CLEARED" | "REFUSED" | "FAILED";

export interface DtcClearRequest {
  ecuFamily: string;
  context: VehicleDescription;
}

export interface DtcClearSnapshot {
  state: DtcClearState;
  ecuFamily: string;
  /** `DTC_CLEAR`. */
  operation: string;
  /** `SERVICE_ROUTINE`. */
  safetyClass: string;
  routeId: string;
  routeValidation: string;
  protocol: string;
  /** The diagnostic session the module answered in, or null on a serial line. */
  session: string | null;
  requestHex: string;
  rawResponseHex: string | null;
  /** The module's refusal, in the protocol's own word for it. */
  refusal: string | null;
  /** The codes the module held before the clear, as read in this session. */
  codesBefore: DtcSummary[];
  /** What the module answered when read again after the clear; null until then. */
  codesAfter: DtcSummary[] | null;
  error: {
    category: string;
    message: string;
    technicalDetails: string | null;
    stage: string;
  } | null;
  reportAvailable: boolean;
}

export interface ServiceClient {
  /** Switch the service mode for this session; the shell keeps the state. */
  setServiceMode(on: boolean): Promise<SessionReportSnapshot>;
  getClearState(): Promise<DtcClearSnapshot>;
  clearDtcs(request: DtcClearRequest): Promise<DtcClearSnapshot>;
}

export const createDtcClearSnapshot = (): DtcClearSnapshot => ({
  state: "IDLE",
  ecuFamily: "",
  operation: "DTC_CLEAR",
  safetyClass: "SERVICE_ROUTINE",
  routeId: "",
  routeValidation: "",
  protocol: "",
  session: null,
  requestHex: "",
  rawResponseHex: null,
  refusal: null,
  codesBefore: [],
  codesAfter: null,
  error: null,
  reportAvailable: false,
});

class TauriServiceClient implements ServiceClient {
  setServiceMode(on: boolean) {
    return invoke<SessionReportSnapshot>("set_service_mode", { on });
  }

  getClearState() {
    return invoke<DtcClearSnapshot>("get_dtc_clear_state");
  }

  clearDtcs(request: DtcClearRequest) {
    return invoke<DtcClearSnapshot>("clear_dtcs", { request });
  }
}

/**
 * The browser preview has no shell and no module: the mode switches, and a
 * clear answers as a module that accepted it would, with nothing behind it.
 * Every value says it is a preview.
 */
class BrowserServiceClient implements ServiceClient {
  private on = false;
  private last: DtcClearSnapshot = createDtcClearSnapshot();

  setServiceMode(on: boolean) {
    this.on = on;
    return Promise.resolve({
      captures: 0,
      moduleReads: 0,
      calibrationReads: 0,
      standardObdReads: 0,
      liveReadRuns: 0,
      mileageSurveys: 0,
      modulePassports: 0,
      ccfReads: 0,
      batteryReads: 0,
      dtcClears: this.last.state === "IDLE" ? 0 : 1,
      routineRuns: 0,
      serviceMode: on,
      reportAvailable: this.last.state !== "IDLE",
      mode: null,
    } satisfies SessionReportSnapshot);
  }

  getClearState() {
    return Promise.resolve(this.last);
  }

  clearDtcs(request: DtcClearRequest) {
    this.last = {
      ...createDtcClearSnapshot(),
      state: this.on ? "CLEARED" : "FAILED",
      ecuFamily: request.ecuFamily,
      routeId: "hs-can",
      routeValidation: "BROWSER_PREVIEW",
      protocol: "ISO14229",
      session: this.on ? "0x01" : null,
      requestHex: "14 FF FF FF",
      rawResponseHex: this.on ? "54" : null,
      codesAfter: this.on ? [] : null,
      error: this.on
        ? null
        : {
            category: "UNSUPPORTED_VEHICLE_PROFILE",
            message: "The clear is not offered",
            technicalDetails: "the service mode is off for this session",
            stage: "PREPARATION",
          },
      reportAvailable: true,
    };
    return Promise.resolve(this.last);
  }
}

export const defaultServiceClient: ServiceClient = browserDemoEnabled()
  ? new BrowserServiceClient()
  : new TauriServiceClient();
