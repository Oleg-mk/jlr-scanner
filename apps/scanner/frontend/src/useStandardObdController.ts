import { useCallback, useEffect, useState } from "react";
import type { VehicleDescription } from "./library";
import {
  createStandardObdSnapshot,
  hexByte,
  isSupportItem,
  type LabelledValue,
  type StandardObdClient,
  type StandardObdMonitor,
  type StandardObdReadKind,
  type StandardObdSnapshot,
  type StandardObdValue,
} from "./standardObd";

/** ISO 15765-4 lets one request carry six PIDs, or six MIDs. */
const ITEMS_PER_REQUEST = 6;

/** The last support map of each mode: PIDs run to 0xC4, MIDs to 0xFF, InfoTypes stay in the first map. */
const LAST_PID_MAP = 0xc0;
const LAST_MID_MAP = 0xe0;
const LAST_INFO_MAP = 0x00;

/**
 * The vehicle-information types that are reads: VIN, calibrations, their
 * verification numbers, the in-use counters (spark and compression), the
 * ECU's name and serial number. The rest of mode 09 is message counts for
 * K-line, which CAN never reports.
 */
const READ_ONLY_INFO_TYPES = ["0x02", "0x04", "0x06", "0x08", "0x0A", "0x0B", "0x0D"];

function failedRead(error: unknown): StandardObdSnapshot {
  return {
    ...createStandardObdSnapshot(),
    state: "FAILED",
    error: {
      category: "INTERNAL_FAILURE",
      message: "Standard OBD-II read failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

/** One answer's values over the table so far: a PID read again replaces its row, the maps stay out. */
function merge(current: StandardObdValue[], incoming: StandardObdValue[]): StandardObdValue[] {
  const kept = current.filter(
    (value) => !incoming.some((next) => next.pid === value.pid && next.name === value.name),
  );
  return [...kept, ...incoming.filter((value) => value.kind !== "supported")];
}

/**
 * The legislated services, one read at a time, plus the walks that read
 * everything a module supports: the support maps first, then the supported
 * items — PIDs and MIDs six to a request, freeze-frame PIDs and InfoTypes one
 * at a time — gathered into their tables as they come.
 */
export function useStandardObdController(client: StandardObdClient, vehicle: VehicleDescription) {
  const [snapshot, setSnapshot] = useState<StandardObdSnapshot>(createStandardObdSnapshot);
  const [values, setValues] = useState<StandardObdValue[]>([]);
  const [frameValues, setFrameValues] = useState<StandardObdValue[]>([]);
  const [monitors, setMonitors] = useState<StandardObdMonitor[]>([]);
  const [information, setInformation] = useState<LabelledValue[]>([]);
  const [responder, setResponder] = useState(0);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getState()
      .then((next) => {
        if (active) setSnapshot(next);
      })
      .catch((error: unknown) => {
        if (active) setSnapshot(failedRead(error));
      });
    return () => {
      active = false;
    };
  }, [client]);

  const request = useCallback(
    (kind: StandardObdReadKind, items: string[]) =>
      client.read({ kind, responder, items, context: vehicle }),
    [client, responder, vehicle],
  );

  const read = useCallback(
    async (kind: StandardObdReadKind, items: string[] = []) => {
      setBusy(true);
      try {
        const next = await request(kind, items);
        setSnapshot(next);
        return next;
      } catch (error) {
        const failed = failedRead(error);
        setSnapshot(failed);
        return failed;
      } finally {
        setBusy(false);
      }
    },
    [request],
  );

  /**
   * Walk a mode's support maps: 0x00 says which of 0x01–0x20 answer and
   * whether 0x20 exists, and so on up the chain. The supported items without
   * the maps themselves; null when the first map went unanswered, the
   * snapshot then holding the module's own refusal.
   */
  const walk = useCallback(
    async (kind: StandardObdReadKind, last: number): Promise<string[] | null> => {
      const supported: string[] = [];
      let base = 0x00;
      for (;;) {
        const map = await request(kind, [hexByte(base)]);
        setSnapshot(map);
        if (map.state !== "SUCCEEDED" || map.negativeResponse !== null) {
          return base === 0x00 ? null : supported;
        }
        supported.push(...map.supported.filter((item) => !isSupportItem(item)));
        const next = base + 0x20;
        if (next > last || !map.supported.includes(hexByte(next))) return supported;
        base = next;
      }
    },
    [request],
  );

  const run = useCallback(async (work: () => Promise<void>) => {
    setBusy(true);
    try {
      await work();
    } catch (error) {
      setSnapshot(failedRead(error));
    } finally {
      setBusy(false);
    }
  }, []);

  const readEverything = useCallback(
    () =>
      run(async () => {
        setValues([]);
        const supported = await walk("CURRENT_DATA", LAST_PID_MAP);
        if (supported === null) return;
        for (let start = 0; start < supported.length; start += ITEMS_PER_REQUEST) {
          const answer = await request("CURRENT_DATA", supported.slice(start, start + ITEMS_PER_REQUEST));
          setSnapshot(answer);
          if (answer.state !== "SUCCEEDED") break;
          setValues((current) => merge(current, answer.values));
        }
      }),
    [request, run, walk],
  );

  /**
   * Freeze frame 0: the code that froze it first, then every PID the frame
   * carries, one to a request — the codec asks one PID a frame. A PID the
   * module refuses inside the frame is skipped, not the whole frame.
   */
  const readFreezeFrame = useCallback(
    () =>
      run(async () => {
        setFrameValues([]);
        const supported = await walk("FREEZE_FRAME", LAST_PID_MAP);
        if (supported === null) return;
        for (const pid of ["0x02", ...supported.filter((item) => item !== "0x02")]) {
          const answer = await request("FREEZE_FRAME", [pid]);
          setSnapshot(answer);
          if (answer.state !== "SUCCEEDED") break;
          if (answer.negativeResponse !== null) continue;
          setFrameValues((current) => merge(current, answer.values));
        }
      }),
    [request, run, walk],
  );

  const readMonitors = useCallback(
    () =>
      run(async () => {
        setMonitors([]);
        const mids = await walk("MONITOR_RESULTS", LAST_MID_MAP);
        if (mids === null) return;
        for (let start = 0; start < mids.length; start += ITEMS_PER_REQUEST) {
          const answer = await request("MONITOR_RESULTS", mids.slice(start, start + ITEMS_PER_REQUEST));
          setSnapshot(answer);
          if (answer.state !== "SUCCEEDED") break;
          setMonitors((current) => [...current, ...answer.monitors]);
        }
      }),
    [request, run, walk],
  );

  const readVehicleInformation = useCallback(
    () =>
      run(async () => {
        setInformation([]);
        const types = await walk("VEHICLE_INFORMATION", LAST_INFO_MAP);
        if (types === null) return;
        for (const type of types.filter((item) => READ_ONLY_INFO_TYPES.includes(item))) {
          const answer = await request("VEHICLE_INFORMATION", [type]);
          setSnapshot(answer);
          if (answer.state !== "SUCCEEDED") break;
          if (answer.negativeResponse !== null) continue;
          setInformation((current) => [...current, ...answer.information]);
        }
      }),
    [request, run, walk],
  );

  const clear = useCallback(() => {
    setValues([]);
    setFrameValues([]);
    setMonitors([]);
    setInformation([]);
  }, []);

  return {
    snapshot,
    values,
    frameValues,
    monitors,
    information,
    busy,
    responder,
    setResponder,
    read,
    readEverything,
    readFreezeFrame,
    readMonitors,
    readVehicleInformation,
    clear,
  };
}
