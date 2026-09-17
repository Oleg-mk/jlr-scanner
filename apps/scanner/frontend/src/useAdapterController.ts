import { useCallback, useEffect, useRef, useState } from "react";
import {
  createEmptySnapshot,
  type AdapterClient,
  type AdapterSnapshot,
} from "./adapter";

const DEFAULT_POLL_INTERVAL_MS = 1_500;

function frontendFailure(snapshot: AdapterSnapshot, error: unknown): AdapterSnapshot {
  return {
    ...snapshot,
    state: "ERROR",
    boardCommunication: "FAILED",
    error: {
      code: "DISCOVERY_FAILED",
      message: "Unable to update adapter state",
      technicalDetails: error instanceof Error ? error.message : String(error),
    },
  };
}

export function useAdapterController(
  client: AdapterClient,
  pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
) {
  const [snapshot, setSnapshot] = useState<AdapterSnapshot>(createEmptySnapshot);
  const [selectedPort, setSelectedPort] = useState<string | null>(null);
  const snapshotRef = useRef(snapshot);
  /**
   * The background poll and what the person asks for used to share one
   * "busy" flag, so a click that landed while the poll was out was dropped
   * where nobody could see it: no action, no error, the panel simply as it
   * was. On Windows, where enumerating the ports is not instant, that was
   * most clicks — the owner could not connect the bench at all (2026-09-16).
   *
   * They are now two. A poll stands aside for a person; a person never
   * stands aside for a poll.
   */
  const pollInFlight = useRef(false);
  const actionInFlight = useRef(false);
  /**
   * Which answer is the current one. A poll that was already out when the
   * person acted must not paint its older picture over the newer one.
   */
  const answer = useRef(0);

  useEffect(() => {
    snapshotRef.current = snapshot;
    if (snapshot.selectedAdapterPort !== null) {
      setSelectedPort(snapshot.selectedAdapterPort);
    }
  }, [snapshot]);

  const refresh = useCallback(async () => {
    if (pollInFlight.current || actionInFlight.current) return;
    pollInFlight.current = true;
    const ticket = answer.current;
    try {
      const next = await client.discover();
      // Nothing the person did overtook this poll while it was out.
      if (ticket === answer.current) setSnapshot(next);
    } catch (error) {
      if (ticket === answer.current) {
        setSnapshot((current) => frontendFailure(current, error));
      }
    } finally {
      pollInFlight.current = false;
    }
  }, [client]);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => {
      const current = snapshotRef.current;
      const retryingHotUnplug =
        current.state === "ERROR" &&
        current.error?.code === "ADAPTER_DISCONNECTED";
      if (current.state !== "ERROR" || retryingHotUnplug) {
        void refresh();
      }
    }, pollIntervalMs);
    return () => window.clearInterval(interval);
  }, [pollIntervalMs, refresh]);

  const connect = useCallback(async () => {
    if (actionInFlight.current) return;
    actionInFlight.current = true;
    answer.current += 1;
    setSnapshot((current) => ({
      ...current,
      state: "CONNECTING",
      boardCommunication: "PENDING",
      error: null,
    }));
    try {
      setSnapshot(await client.connect(selectedPort));
    } catch (error) {
      setSnapshot((current) => frontendFailure(current, error));
    } finally {
      actionInFlight.current = false;
    }
  }, [client, selectedPort]);

  const connectBench = useCallback(async (scenario: number) => {
    if (actionInFlight.current) return;
    actionInFlight.current = true;
    answer.current += 1;
    setSnapshot((current) => ({
      ...current,
      state: "CONNECTING",
      boardCommunication: "PENDING",
      error: null,
    }));
    try {
      setSnapshot(await client.connectBench(scenario));
    } catch (error) {
      setSnapshot((current) => frontendFailure(current, error));
    } finally {
      actionInFlight.current = false;
    }
  }, [client]);

  const disconnect = useCallback(async () => {
    if (actionInFlight.current) return;
    actionInFlight.current = true;
    answer.current += 1;
    try {
      setSnapshot(await client.disconnect());
    } catch (error) {
      setSnapshot((current) => frontendFailure(current, error));
    } finally {
      actionInFlight.current = false;
    }
  }, [client]);

  /**
   * Say on the panel that something the person asked for did not happen.
   * A click that vanishes without a word is worse than one that fails: the
   * owner pressed "connect the bench" and got silence, and nothing on screen
   * or in the shell said why (2026-09-16).
   */
  const reportRefusal = useCallback((message: string, details: string | null) => {
    answer.current += 1;
    setSnapshot((current) => ({
      ...current,
      error: {
        code: "DISCOVERY_FAILED",
        message,
        technicalDetails: details,
      },
    }));
  }, []);

  return {
    snapshot,
    selectedPort,
    setSelectedPort,
    refresh,
    connect,
    connectBench,
    disconnect,
    reportRefusal,
  };
}
