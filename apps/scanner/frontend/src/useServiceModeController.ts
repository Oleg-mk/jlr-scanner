import { useCallback, useEffect, useState } from "react";
import type { VehicleDescription } from "./library";
import {
  createDtcClearSnapshot,
  type DtcClearSnapshot,
  type ServiceClient,
} from "./serviceMode";

/**
 * The one collective clear as it runs (ADR-0036, decision 3): the modules
 * it was confirmed for, in the order they are asked, the one being asked
 * now, and every answer so far. Null while none has been started.
 */
export interface ClearSequence {
  families: string[];
  /** The module being asked now; null once every one has answered. */
  current: string | null;
  outcomes: DtcClearSnapshot[];
}

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
  /** Every clear that answered, as it answered: what the module holds now. */
  onCleared?: (clear: DtcClearSnapshot) => void,
) {
  // The consent is shown once, before the mode goes on; nothing goes on
  // until the person accepts it.
  const [consentOpen, setConsentOpen] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [clear, setClear] = useState<DtcClearSnapshot>(createDtcClearSnapshot);
  const [clearing, setClearing] = useState(false);
  const [sequence, setSequence] = useState<ClearSequence | null>(null);
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
        const answered = await client.clearDtcs({ ecuFamily, context: vehicle });
        setClear(answered);
        onCleared?.(answered);
        await onModeChanged();
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
      } finally {
        setClearing(false);
      }
    },
    [client, onCleared, onModeChanged, serviceMode, vehicle],
  );

  /**
   * The one collective clear (ADR-0036, decision 3): after the one question
   * that listed them, every module is asked in turn and every answer is
   * recorded on its own - the shell knows no sequence, only clears. A
   * refusal or a failure of one module does not stop the rest.
   */
  const clearMany = useCallback(
    async (families: string[]) => {
      if (!serviceMode || families.length === 0) return;
      setClearing(true);
      setError(null);
      const outcomes: DtcClearSnapshot[] = [];
      try {
        for (const ecuFamily of families) {
          setSequence({ families, current: ecuFamily, outcomes: [...outcomes] });
          try {
            const answered = await client.clearDtcs({ ecuFamily, context: vehicle });
            outcomes.push(answered);
            setClear(answered);
            onCleared?.(answered);
          } catch (caught) {
            outcomes.push({
              ...createDtcClearSnapshot(),
              state: "FAILED",
              ecuFamily,
              error: {
                category: "ADAPTER",
                message: caught instanceof Error ? caught.message : String(caught),
                technicalDetails: null,
                stage: "EXECUTION",
              },
            });
          }
        }
        setSequence({ families, current: null, outcomes });
        await onModeChanged();
      } finally {
        setClearing(false);
      }
    },
    [client, onCleared, onModeChanged, serviceMode, vehicle],
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
    clearMany,
    sequence,
    error,
  };
}
