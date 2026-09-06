import { useCallback, useEffect, useState } from "react";
import {
  createCaptureSnapshot,
  saveCaptureFile,
  type CaptureClient,
  type CaptureSnapshot,
} from "./capture";
import type { VehicleDescription } from "./library";

function failedCapture(routeId: string, error: unknown): CaptureSnapshot {
  return {
    ...createCaptureSnapshot(),
    state: "FAILED",
    routeId,
    error: `Capture failed: ${error instanceof Error ? error.message : String(error)}`,
  };
}

export function useCaptureController(client: CaptureClient, vehicle: VehicleDescription) {
  const [snapshot, setSnapshot] = useState<CaptureSnapshot>(createCaptureSnapshot);
  const [routeId, setRouteId] = useState("hs-can");
  const [seconds, setSeconds] = useState(5);
  const [busy, setBusy] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getState()
      .then((next) => {
        if (active) setSnapshot(next);
      })
      .catch((error: unknown) => {
        if (active) setSnapshot(failedCapture("", error));
      });
    return () => {
      active = false;
    };
  }, [client]);

  const listen = useCallback(async () => {
    setBusy(true);
    try {
      setSnapshot(await client.listen(routeId, seconds, vehicle));
    } catch (error) {
      setSnapshot(failedCapture(routeId, error));
    } finally {
      setBusy(false);
    }
  }, [client, routeId, seconds, vehicle]);

  const save = useCallback(async () => {
    setSaving(true);
    try {
      await saveCaptureFile(await client.getCaptureJson());
    } catch (error) {
      setSnapshot(failedCapture(routeId, error));
    } finally {
      setSaving(false);
    }
  }, [client, routeId]);

  return { snapshot, routeId, setRouteId, seconds, setSeconds, busy, saving, listen, save };
}
