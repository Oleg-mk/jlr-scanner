import { invoke } from "@tauri-apps/api/core";
import { saveTextFile } from "./files";
import { t } from "./i18n";

export interface SessionReportSnapshot {
  captures: number;
  moduleReads: number;
  calibrationReads: number;
  reportAvailable: boolean;
}

export interface SessionReportClient {
  getState(): Promise<SessionReportSnapshot>;
  getReportJson(): Promise<string>;
}

export const createSessionReportSnapshot = (): SessionReportSnapshot => ({
  captures: 0,
  moduleReads: 0,
  calibrationReads: 0,
  reportAvailable: false,
});

class TauriSessionReportClient implements SessionReportClient {
  getState() {
    return invoke<SessionReportSnapshot>("get_session_report_state");
  }

  getReportJson() {
    return invoke<string>("get_session_report_json");
  }
}

/** The browser preview records nothing, so there is never a report to save. */
class BrowserSessionReportClient implements SessionReportClient {
  getState(): Promise<SessionReportSnapshot> {
    return Promise.resolve(createSessionReportSnapshot());
  }

  getReportJson(): Promise<string> {
    return Promise.reject(new Error("Saving a session report needs the desktop application."));
  }
}

function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

export const defaultSessionReportClient: SessionReportClient = hasTauriRuntime()
  ? new TauriSessionReportClient()
  : new BrowserSessionReportClient();

/** Save the session report where the user chooses; resolves to the path, or null when cancelled. */
export function saveSessionReportFile(json: string) {
  const parsed = JSON.parse(json) as { saved_unix_ms?: number };
  const stamp = parsed.saved_unix_ms ?? Date.now();
  return saveTextFile(`jlr-scanner-session-${stamp}.json`, json);
}

export type SessionStepState = "done" | "next" | "todo";

export interface SessionStep {
  id: string;
  title: string;
  hint: string;
  state: SessionStepState;
  optional: boolean;
}

export interface SessionProgress {
  adapterReady: boolean;
  libraryLoaded: boolean;
  vehicleDescribed: boolean;
  surveyed: boolean;
  captures: number;
  moduleReads: number;
  reportAvailable: boolean;
}

/**
 * The session as one ordered flow. A step is `done` when its evidence exists,
 * `next` when it is the first required step still open, `todo` otherwise.
 * Optional steps never block the flow; the order is the order the panels
 * appear in.
 */
export function sessionSteps(progress: SessionProgress): SessionStep[] {
  const steps: Array<Omit<SessionStep, "state"> & { done: boolean }> = [];
  const push = (step: Omit<SessionStep, "state"> & { done: boolean }) => steps.push(step);
  push({
    id: "adapter",
    title: t("Connect the adapter"),
    hint: t("Detect the MongoosePro JLR and verify board communication."),
    optional: false,
    done: progress.adapterReady,
  });
  push({
    id: "library",
    title: t("Load the data library"),
    hint: t("The exported library folder given to you with the application."),
    optional: false,
    done: progress.libraryLoaded,
  });
  push({
    id: "vehicle",
    title: t("Choose the vehicle"),
    hint: t("Programme and model years from the library; engine if known."),
    optional: false,
    done: progress.vehicleDescribed,
  });
  push({
    id: "survey",
    title: t("Survey the modules"),
    hint: t("Nothing is transmitted; every module is listed with its reach."),
    optional: false,
    done: progress.surveyed,
  });
  push({
    id: "capture",
    title: t("Listen to a bus"),
    hint: t("Optional, zero-risk first contact: the adapter only listens."),
    optional: true,
    done: progress.captures > 0,
  });
  push({
    id: "read",
    title: t("Read a module"),
    hint: t("Fault codes or one identifier from a reachable module."),
    optional: false,
    done: progress.moduleReads > 0,
  });
  push({
    id: "save",
    title: t("Save the session report"),
    hint: t("One file with everything recorded, for the tester programme."),
    optional: false,
    done: false,
  });

  let nextAssigned = false;
  return steps.map(({ done, ...step }) => {
    let state: SessionStepState = done ? "done" : "todo";
    if (!done && !step.optional && !nextAssigned) {
      state = "next";
      nextAssigned = true;
    }
    if (step.id === "save" && !progress.reportAvailable && state === "next") {
      state = "todo";
    }
    return { ...step, state };
  });
}
