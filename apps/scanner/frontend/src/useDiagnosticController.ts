import { useCallback, useEffect, useState } from "react";
import {
  createDiagnosticSnapshot,
  saveDiagnosticReportFile,
  type DiagnosticClient,
  type DiagnosticSnapshot,
} from "./diagnostic";

function frontendFailure(error: unknown): DiagnosticSnapshot {
  return {
    ...createDiagnosticSnapshot(),
    state: "FAILED",
    error: {
      category: "INTERNAL_FAILURE",
      message: "Diagnostic execution failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

export function useDiagnosticController(
  client: DiagnosticClient,
  adapterReady: boolean,
) {
  const [snapshot, setSnapshot] = useState<DiagnosticSnapshot>(createDiagnosticSnapshot);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getState()
      .then((next) => {
        if (active) setSnapshot(next);
      })
      .catch((error: unknown) => {
        if (active) setSnapshot(frontendFailure(error));
      });
    return () => {
      active = false;
    };
  }, [adapterReady, client]);

  const read = useCallback(async () => {
    setSnapshot((current) => ({ ...current, state: "RUNNING", error: null }));
    try {
      setSnapshot(await client.readCalibrationIdentification());
    } catch (error) {
      setSnapshot(frontendFailure(error));
    }
  }, [client]);

  const saveReport = useCallback(async () => {
    setSaving(true);
    try {
      await saveDiagnosticReportFile(await client.getReportJson());
    } catch (error) {
      setSnapshot(frontendFailure(error));
    } finally {
      setSaving(false);
    }
  }, [client]);

  return { snapshot, read, saveReport, saving };
}
