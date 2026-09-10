import { useCallback, useEffect, useRef, useState } from "react";
import type { ModuleSurveyEntry, VehicleDescription } from "./library";
import {
  LIVE_READ_MAX_ENTRIES,
  createLiveReadSnapshot,
  entryKey,
  type LiveReadClient,
  type LiveReadEntryRequest,
  type LiveReadSnapshot,
} from "./liveRead";

/**
 * How often the interface asks for the next step. The shell refuses anything
 * sooner than its own floor, so asking a little faster than the floor costs
 * nothing and keeps the round tight (ADR-0022, decisions 3 and 4).
 */
const STEP_INTERVAL_MS = 60;

function failed(error: unknown): LiveReadSnapshot {
  return {
    ...createLiveReadSnapshot(),
    state: "STOPPED",
    error: {
      category: "INTERNAL_FAILURE",
      message: "Live read failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

/** Every module-and-identifier pair the survey offers, in the survey's order. */
export function liveCandidates(modules: ModuleSurveyEntry[]): LiveReadEntryRequest[] {
  return modules.flatMap((module) =>
    module.readableIdentifiers.map((entry) => ({
      ecuFamily: module.ecuFamily,
      identifier: entry.identifier,
    })),
  );
}

export function useLiveReadController(
  client: LiveReadClient,
  vehicle: VehicleDescription,
  stepIntervalMs: number = STEP_INTERVAL_MS,
) {
  const [snapshot, setSnapshot] = useState<LiveReadSnapshot>(createLiveReadSnapshot);
  const [set, setSet] = useState<LiveReadEntryRequest[]>([]);
  const [busy, setBusy] = useState(false);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const stepping = useRef(false);

  const clearTimer = useCallback(() => {
    if (timer.current !== null) {
      clearInterval(timer.current);
      timer.current = null;
    }
  }, []);

  useEffect(() => {
    let active = true;
    void client
      .getState()
      .then((next) => {
        if (active) setSnapshot(next);
      })
      .catch((error: unknown) => {
        if (active) setSnapshot(failed(error));
      });
    return () => {
      active = false;
    };
  }, [client]);

  // A run never outlives the panel: the timer stops with it, and the shell's
  // own cap stops the run.
  useEffect(() => clearTimer, [clearTimer]);

  /** Add or remove one module-and-identifier pair; the set is capped at sixteen. */
  const toggle = useCallback((entry: LiveReadEntryRequest) => {
    setSet((current) => {
      const key = entryKey(entry);
      const kept = current.filter((held) => entryKey(held) !== key);
      if (kept.length !== current.length) return kept;
      if (current.length >= LIVE_READ_MAX_ENTRIES) return current;
      return [...current, entry];
    });
  }, []);

  const clearSet = useCallback(() => setSet([]), []);

  const stop = useCallback(async () => {
    clearTimer();
    try {
      setSnapshot(await client.stop());
    } catch (error) {
      setSnapshot(failed(error));
    }
  }, [clearTimer, client]);

  const start = useCallback(async () => {
    if (set.length === 0) return;
    setBusy(true);
    clearTimer();
    try {
      const started = await client.start({ entries: set, context: vehicle });
      setSnapshot(started);
      if (started.state !== "RUNNING") return;
      timer.current = setInterval(() => {
        // One step at a time: a slow answer must not pile requests up.
        if (stepping.current) return;
        stepping.current = true;
        void client
          .step()
          .then((next) => {
            setSnapshot(next);
            if (next.state !== "RUNNING") clearTimer();
          })
          .catch((error: unknown) => {
            setSnapshot(failed(error));
            clearTimer();
          })
          .finally(() => {
            stepping.current = false;
          });
      }, stepIntervalMs);
    } catch (error) {
      setSnapshot(failed(error));
    } finally {
      setBusy(false);
    }
  }, [clearTimer, client, set, stepIntervalMs, vehicle]);

  return {
    snapshot,
    set,
    busy,
    running: snapshot.state === "RUNNING",
    toggle,
    clearSet,
    start,
    stop,
  };
}
