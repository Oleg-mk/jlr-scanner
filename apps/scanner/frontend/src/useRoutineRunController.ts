import { useCallback, useEffect, useRef, useState } from "react";
import type { VehicleDescription } from "./library";
import {
  createRoutineRunSnapshot,
  type RoutineRunClient,
  type RoutineRunSnapshot,
} from "./routineRun";

function failed(error: unknown): RoutineRunSnapshot {
  return {
    ...createRoutineRunSnapshot(),
    state: "FAILED",
    error: {
      category: "INTERNAL_FAILURE",
      message: error instanceof Error ? error.message : String(error),
      technicalDetails: null,
      stage: "PREPARATION",
    },
  };
}

/**
 * A routine run as the interface drives it (ADR-0036, step 2): the start is
 * one request, then a step every second - the shell decides whether the
 * step is a keep-alive or the request for results, by its clock and the
 * data's time - until the shell says the run has ended. A stop is one
 * request too, and gets through between two steps. The confirmation is the
 * caller's, before the start.
 */
export function useRoutineRunController(
  client: RoutineRunClient,
  vehicle: VehicleDescription,
  serviceMode: boolean,
  onEnded: () => void | Promise<void>,
  stepIntervalMs = 1000,
) {
  const [snapshot, setSnapshot] = useState<RoutineRunSnapshot>(createRoutineRunSnapshot);
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
    let live = true;
    void client
      .getState()
      .then((next) => {
        if (live) setSnapshot(next);
      })
      .catch(() => {
        // No state yet is the idle state.
      });
    return () => {
      live = false;
      clearTimer();
    };
  }, [client, clearTimer]);

  const start = useCallback(
    async (ecuFamily: string, testId: string) => {
      if (!serviceMode) return;
      setBusy(true);
      clearTimer();
      try {
        const started = await client.start({ ecuFamily, testId, context: vehicle });
        setSnapshot(started);
        if (started.state !== "RUNNING") {
          await onEnded();
          return;
        }
        timer.current = setInterval(() => {
          // One step at a time: a slow answer must not pile requests up.
          if (stepping.current) return;
          stepping.current = true;
          void client
            .step()
            .then((next) => {
              setSnapshot(next);
              if (next.state !== "RUNNING") {
                clearTimer();
                void onEnded();
              }
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
    },
    [clearTimer, client, onEnded, serviceMode, stepIntervalMs, vehicle],
  );

  const stop = useCallback(async () => {
    clearTimer();
    try {
      setSnapshot(await client.stop());
      await onEnded();
    } catch (error) {
      setSnapshot(failed(error));
    }
  }, [clearTimer, client, onEnded]);

  return {
    snapshot,
    running: snapshot.state === "RUNNING",
    busy,
    start,
    stop,
  };
}
