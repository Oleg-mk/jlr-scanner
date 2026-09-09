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
  const inFlightRef = useRef(false);

  useEffect(() => {
    snapshotRef.current = snapshot;
    if (snapshot.selectedAdapterPort !== null) {
      setSelectedPort(snapshot.selectedAdapterPort);
    }
  }, [snapshot]);

  const refresh = useCallback(async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    try {
      setSnapshot(await client.discover());
    } catch (error) {
      setSnapshot((current) => frontendFailure(current, error));
    } finally {
      inFlightRef.current = false;
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
    if (inFlightRef.current) return;
    inFlightRef.current = true;
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
      inFlightRef.current = false;
    }
  }, [client, selectedPort]);

  const connectBench = useCallback(async (scenario: number) => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
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
      inFlightRef.current = false;
    }
  }, [client]);

  const disconnect = useCallback(async () => {
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    try {
      setSnapshot(await client.disconnect());
    } catch (error) {
      setSnapshot((current) => frontendFailure(current, error));
    } finally {
      inFlightRef.current = false;
    }
  }, [client]);

  return {
    snapshot,
    selectedPort,
    setSelectedPort,
    refresh,
    connect,
    connectBench,
    disconnect,
  };
}
