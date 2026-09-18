import { useCallback, useEffect, useState } from "react";
import type { VehicleDescription } from "./library";
import {
  createDtcClearSnapshot,
  type DtcClearSnapshot,
  type ServiceClient,
} from "./serviceMode";

/**
 * The service mode (ADR-0036): the switch, the consent that stands before
 * it, and the first operation behind it. The mode's truth is the shell's -
 * the session snapshot says whether it is on - and this controller asks the
 * shell to change it and reports what the shell answered.
 */
export function useServiceModeController(
  client: ServiceClient,
  vehicle: VehicleDescription,
  serviceMode: boolean,
  onModeChanged: () => void | Promise<void>,
) {
  // The consent is shown once, before the mode goes on; nothing goes on
  // until the person accepts it.
  const [consentOpen, setConsentOpen] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [clear, setClear] = useState<DtcClearSnapshot>(createDtcClearSnapshot);
  const [clearing, setClearing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let live = true;
    void client
      .getClearState()
      .then((next) => {
        if (live) setClear(next);
      })
      .catch(() => {
        // No state yet is the idle state.
      });
    return () => {
      live = false;
    };
  }, [client]);

  const askToTurnOn = useCallback(() => setConsentOpen(true), []);
  const declineConsent = useCallback(() => setConsentOpen(false), []);

  const setMode = useCallback(
    async (on: boolean) => {
      setSwitching(true);
      setError(null);
      try {
        await client.setServiceMode(on);
        await onModeChanged();
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setSwitching(false);
        setConsentOpen(false);
      }
    },
    [client, onModeChanged],
  );

  const acceptConsent = useCallback(() => setMode(true), [setMode]);
  const turnOff = useCallback(() => setMode(false), [setMode]);

  /** Clear one module's codes; the confirmation is the caller's, before this. */
  const clearCodes = useCallback(
    async (ecuFamily: string) => {
      if (!serviceMode) return;
      setClearing(true);
      setError(null);
      try {
        setClear(await client.clearDtcs({ ecuFamily, context: vehicle }));
        await onModeChanged();
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setClearing(false);
      }
    },
    [client, onModeChanged, serviceMode, vehicle],
  );

  return {
    on: serviceMode,
    consentOpen,
    switching,
    askToTurnOn,
    declineConsent,
    acceptConsent,
    turnOff,
    clear,
    clearing,
    clearCodes,
    error,
  };
}
