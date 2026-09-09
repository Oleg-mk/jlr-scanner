import { invoke } from "@tauri-apps/api/core";
import { saveTextFile } from "./files";
import { browserDemoEnabled } from "./adapter";
import type { VehicleDescription } from "./library";

export type CaptureState = "IDLE" | "COMPLETED" | "FAILED";

export interface CaptureIdCount {
  id: string;
  extended: boolean;
  count: number;
}

export interface CaptureSnapshot {
  state: CaptureState;
  routeId: string;
  pins: string;
  bitrateBps: number | null;
  requestedSeconds: number;
  listenedMs: number;
  frames: number;
  framesPerSecond: number;
  distinctIds: number;
  standardFrames: number;
  extendedFrames: number;
  droppedFrames: number;
  truncated: boolean;
  topIds: CaptureIdCount[];
  verdict: string;
  error: string | null;
  captureAvailable: boolean;
  /** Heard on the bench (ADR-0020): synthetic, never evidence. */
  synthetic: boolean;
}

export interface CaptureClient {
  getState(): Promise<CaptureSnapshot>;
  listen(routeId: string, seconds: number, vehicle: VehicleDescription): Promise<CaptureSnapshot>;
  getCaptureJson(): Promise<string>;
}

export const captureRoutes = [
  { id: "hs-can", label: "HS-CAN, pins 6/14, 500 kbit/s" },
  { id: "ms-can", label: "MS-CAN, pins 3/11, 125 kbit/s" },
] as const;

export const createCaptureSnapshot = (): CaptureSnapshot => ({
  state: "IDLE",
  routeId: "",
  pins: "",
  bitrateBps: null,
  requestedSeconds: 0,
  listenedMs: 0,
  frames: 0,
  framesPerSecond: 0,
  distinctIds: 0,
  standardFrames: 0,
  extendedFrames: 0,
  droppedFrames: 0,
  truncated: false,
  topIds: [],
  verdict: "",
  error: null,
  captureAvailable: false,
  synthetic: false,
});

class TauriCaptureClient implements CaptureClient {
  getState() {
    return invoke<CaptureSnapshot>("get_capture_state");
  }

  listen(routeId: string, seconds: number, vehicle: VehicleDescription) {
    return invoke<CaptureSnapshot>("capture_bus", { routeId, seconds, context: vehicle });
  }

  getCaptureJson() {
    return invoke<string>("get_capture_json");
  }
}

/** The browser preview has no adapter; in demo mode it shows a synthetic result. */
class BrowserCaptureClient implements CaptureClient {
  private last: CaptureSnapshot = createCaptureSnapshot();

  getState() {
    return Promise.resolve(this.last);
  }

  listen(routeId: string, seconds: number): Promise<CaptureSnapshot> {
    if (!browserDemoEnabled()) {
      this.last = {
        ...createCaptureSnapshot(),
        state: "FAILED",
        routeId,
        error: "Listening on a bus needs the desktop application and a connected adapter.",
      };
      return Promise.resolve(this.last);
    }
    this.last = {
      ...createCaptureSnapshot(),
      state: "COMPLETED",
      routeId,
      pins: routeId === "hs-can" ? "6/14" : "3/11",
      bitrateBps: routeId === "hs-can" ? 500_000 : 125_000,
      requestedSeconds: seconds,
      listenedMs: seconds * 1000,
      frames: 1240,
      framesPerSecond: Math.round(1240 / seconds),
      distinctIds: 3,
      standardFrames: 1240,
      extendedFrames: 0,
      droppedFrames: 0,
      truncated: false,
      topIds: [
        { id: "0x321", extended: false, count: 620 },
        { id: "0x3A0", extended: false, count: 400 },
        { id: "0x0C0", extended: false, count: 220 },
      ],
      verdict:
        "Synthetic preview: traffic present. A live bus is on this pair. Which of the vehicle's buses it is cannot be told from listening alone.",
      error: null,
      captureAvailable: true,
      synthetic: false,
    };
    return Promise.resolve(this.last);
  }

  getCaptureJson() {
    return Promise.resolve(
      JSON.stringify({
        schema_version: 1,
        fixture_class: "captured",
        name: "capture-browser-demo",
        note: "Development browser demo — no adapter or vehicle interaction",
        frames: [],
      }),
    );
  }
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultCaptureClient: CaptureClient = hasTauriRuntime()
  ? new TauriCaptureClient()
  : new BrowserCaptureClient();

/** Save the capture where the user chooses; resolves to the path, or null when cancelled. */
export function saveCaptureFile(json: string) {
  const parsed = JSON.parse(json) as { name?: string };
  const name = parsed.name ?? "capture";
  return saveTextFile(`jlr-scanner-${name}.json`, json);
}
