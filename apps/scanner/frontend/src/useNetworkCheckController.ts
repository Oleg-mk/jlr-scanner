import { useCallback, useRef, useState } from "react";
import type { ModuleSurveyEntry, VehicleDescription } from "./library";
import { createModuleReadSnapshot, type ModuleReadClient, type ModuleReadSnapshot } from "./moduleRead";

function failedCheck(ecuFamily: string, error: unknown): ModuleReadSnapshot {
  return {
    ...createModuleReadSnapshot(),
    state: "FAILED",
    ecuFamily,
    error: {
      category: "INTERNAL_FAILURE",
      message: "Module read failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

/**
 * SDD's network integrity test, read-only: one confirmed-fault-code request
 * to each module in turn, each answer handed back as it arrives. Stopping
 * finishes the module in flight and goes no further.
 */
export function useNetworkCheckController(
  client: ModuleReadClient,
  vehicle: VehicleDescription,
  onResult: (snapshot: ModuleReadSnapshot) => void,
) {
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [current, setCurrent] = useState<string | null>(null);
  const [includeHypothesis, setIncludeHypothesis] = useState(true);
  const stopRequested = useRef(false);

  const run = useCallback(
    async (modules: ModuleSurveyEntry[]) => {
      if (modules.length === 0) return;
      stopRequested.current = false;
      setProgress({ done: 0, total: modules.length });
      for (const [index, module] of modules.entries()) {
        if (stopRequested.current) break;
        setCurrent(module.ecuFamily);
        let snapshot: ModuleReadSnapshot;
        try {
          snapshot = await client.read({
            ecuFamily: module.ecuFamily,
            kind: "FAULT_CODES",
            identifier: null,
            context: vehicle,
          });
        } catch (error) {
          snapshot = failedCheck(module.ecuFamily, error);
        }
        onResult(snapshot);
        setProgress({ done: index + 1, total: modules.length });
      }
      setCurrent(null);
      setProgress(null);
    },
    [client, onResult, vehicle],
  );

  const stop = useCallback(() => {
    stopRequested.current = true;
  }, []);

  return { progress, current, includeHypothesis, setIncludeHypothesis, run, stop };
}
