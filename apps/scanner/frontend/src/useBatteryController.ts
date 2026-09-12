import { useCallback, useEffect, useRef, useState } from "react";
import {
  createBatterySnapshot,
  type BatteryClient,
  type BatteryReadSnapshot,
} from "./battery";
import type { VehicleDescription } from "./library";

/** How often the interface asks for the next read; one request at a time. */
const STEP_INTERVAL_MS = 40;

function failed(error: unknown): BatteryReadSnapshot {
  return {
    ...createBatterySnapshot(),
    state: "FINISHED",
    error: {
      category: "INTERNAL_FAILURE",
      message: "Reading the battery failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

/**
 * The run walks every read the shell planned, one request at a time. The
 * shell owns the plan and the decoding; this owns the timer and the stop, so
 * what was read stays when the run is abandoned.
 */
export function useBatteryController(
  client: BatteryClient,
  vehicle: VehicleDescription,
  stepIntervalMs: number = STEP_INTERVAL_MS,
) {
  const [snapshot, setSnapshot] =
    useState<BatteryReadSnapshot>(createBatterySnapshot);
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

  useEffect(() => clearTimer, [clearTimer]);

  const stop = useCallback(async () => {
    clearTimer();
    try {
      setSnapshot(await client.finish());
    } catch (error) {
      setSnapshot(failed(error));
    }
  }, [clearTimer, client]);

  const start = useCallback(async () => {
    setBusy(true);
    clearTimer();
    try {
      const started = await client.start(vehicle);
      setSnapshot(started);
      if (started.state !== "RUNNING") return;
      timer.current = setInterval(() => {
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
  }, [clearTimer, client, stepIntervalMs, vehicle]);

  return {
    snapshot,
    busy,
    running: snapshot.state === "RUNNING",
    start,
    stop,
  };
}
