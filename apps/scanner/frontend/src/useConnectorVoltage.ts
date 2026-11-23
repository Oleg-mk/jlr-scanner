import { useEffect, useRef, useState } from "react";
import type { AdapterClient, AdapterVoltageSnapshot } from "./adapter";

/** How often the adapter is asked while nothing else holds it (ADR-0030, amended 2026-11-15). */
export const CONNECTOR_VOLTAGE_INTERVAL_MS = 5_000;

/** The last value the adapter gave, with when it was read. */
export interface ConnectorVoltageReading {
  millivolts: number;
  atMs: number;
  synthetic: boolean;
}

/**
 * The battery voltage at the connector, as the adapter measures it, the
 * whole time an adapter is connected (ADR-0030, amended 2026-11-15): asked
 * once on the connect, every five seconds after, and again whenever
 * `activity` changes (an operation ended and was recorded). Not asked while
 * `paused` — a live run or a capture — and the shell itself answers BUSY
 * without asking while an operation holds the adapter. A refusal or a
 * silence leaves the last good reading with its age; once the firmware says
 * the value is not supported nothing more is asked on this connection; a
 * disconnect forgets the reading.
 */
export function useConnectorVoltage(
  client: AdapterClient,
  ready: boolean,
  paused: boolean,
  activity: unknown,
  intervalMs: number = CONNECTOR_VOLTAGE_INTERVAL_MS,
) {
  const [reading, setReading] = useState<ConnectorVoltageReading | null>(null);
  const [last, setLast] = useState<AdapterVoltageSnapshot | null>(null);
  const [unsupported, setUnsupported] = useState(false);
  const asking = useRef(false);

  useEffect(() => {
    if (!ready) {
      setReading(null);
      setLast(null);
      setUnsupported(false);
    }
  }, [ready]);

  const active = ready && !paused && !unsupported && client.readVoltage !== undefined;

  useEffect(() => {
    if (!active || client.readVoltage === undefined) return undefined;
    const read = client.readVoltage.bind(client);
    let live = true;
    const ask = async () => {
      if (asking.current) return;
      asking.current = true;
      try {
        const answer = await read();
        if (!live) return;
        if (answer.state === "BUSY" || answer.state === "NO_ADAPTER") return;
        setLast(answer);
        if (answer.state === "MEASURED" && answer.millivolts !== null) {
          setReading({
            millivolts: answer.millivolts,
            atMs: answer.readUnixMs > 0 ? answer.readUnixMs : Date.now(),
            synthetic: answer.synthetic,
          });
        } else if (answer.state === "UNSUPPORTED") {
          setUnsupported(true);
        }
      } catch {
        // The shell did not answer; the last reading stands with its age.
      } finally {
        asking.current = false;
      }
    };
    void ask();
    const timer = setInterval(() => void ask(), intervalMs);
    return () => {
      live = false;
      clearInterval(timer);
    };
  }, [active, client, intervalMs, activity]);

  return { reading, last, unsupported };
}
