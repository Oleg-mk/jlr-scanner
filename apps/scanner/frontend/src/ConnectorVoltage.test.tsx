import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createEmptySnapshot,
  type AdapterClient,
  type AdapterVoltageSnapshot,
  type AdapterVoltageState,
} from "./adapter";
import { createBatterySnapshot } from "./battery";
import { batteryPreconditionText, voltageSourceText } from "./batteryFormat";
import { createLiveReadSnapshot } from "./liveRead";
import { useConnectorVoltage } from "./useConnectorVoltage";
import { latestVoltage } from "./voltage";

/**
 * The voltage at the connector (ADR-0030, amended 2026-11-15): the adapter's
 * own measurement, asked every few seconds while it is connected, the
 * freshest of the four sources whenever there is one.
 */
function answer(state: AdapterVoltageState, millivolts: number | null, atMs = 0): AdapterVoltageSnapshot {
  return {
    state,
    millivolts,
    status: state === "MEASURED" ? "0x00000000" : null,
    statusName: state === "MEASURED" ? "eSuccess" : null,
    message: null,
    word: null,
    readUnixMs: atMs,
    synthetic: false,
  };
}

class VoltageClient implements AdapterClient {
  answers: AdapterVoltageSnapshot[] = [];
  asked = 0;
  getState() {
    return Promise.resolve(createEmptySnapshot());
  }
  discover() {
    return this.getState();
  }
  connect() {
    return this.getState();
  }
  disconnect() {
    return this.getState();
  }
  connectBench() {
    return this.getState();
  }
  readVoltage() {
    this.asked += 1;
    const next = this.answers.length > 1 ? this.answers.shift()! : this.answers[0];
    return Promise.resolve(next);
  }
}

afterEach(() => {
  vi.useRealTimers();
});

describe("the voltage at the connector", () => {
  it("is the freshest source, said as measured by the adapter, to a tenth of a volt", () => {
    const reading = latestVoltage({
      battery: createBatterySnapshot(),
      obdValues: [],
      obdAtMs: null,
      obdRouteValidation: "",
      live: createLiveReadSnapshot(),
      liveStartedAtMs: null,
      adapter: { millivolts: 11_240, atMs: 5_000, synthetic: false },
    });
    expect(reading).toMatchObject({ volts: 11.2, text: "11.2", source: "adapter" });
    expect(voltageSourceText(reading!)).toBe("measured by the adapter at the connector");
    expect(batteryPreconditionText(reading, 11_600, 5_000)).toMatch(
      /11\.2 V.*measured by the adapter at the connector/,
    );
  });

  it("is asked on the connect and every few seconds, skips BUSY, and forgets on a disconnect", async () => {
    vi.useFakeTimers();
    const client = new VoltageClient();
    client.answers = [
      answer("MEASURED", 12_600, 1_000),
      answer("BUSY", null),
      answer("MEASURED", 12_400, 3_000),
    ];
    const { result, rerender } = renderHook(
      ({ ready, paused }) => useConnectorVoltage(client, ready, paused, 0, 1_000),
      { initialProps: { ready: true, paused: false } },
    );
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.reading).toEqual({ millivolts: 12_600, atMs: 1_000, synthetic: false });

    // A BUSY answer changes nothing: an operation held the adapter.
    await act(async () => {
      vi.advanceTimersByTime(1_000);
      await Promise.resolve();
    });
    expect(result.current.reading?.millivolts).toBe(12_600);
    await act(async () => {
      vi.advanceTimersByTime(1_000);
      await Promise.resolve();
    });
    expect(result.current.reading).toEqual({ millivolts: 12_400, atMs: 3_000, synthetic: false });

    // Paused for a live run or a capture: nothing is asked.
    const before = client.asked;
    rerender({ ready: true, paused: true });
    await act(async () => {
      vi.advanceTimersByTime(5_000);
      await Promise.resolve();
    });
    expect(client.asked).toBe(before);

    // A disconnect forgets the reading.
    rerender({ ready: false, paused: false });
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.reading).toBeNull();
  });

  it("stops asking once the firmware says the value is not supported, and keeps the last good reading on a refusal", async () => {
    vi.useFakeTimers();
    const client = new VoltageClient();
    client.answers = [answer("MEASURED", 12_600, 1_000), answer("REFUSED", null, 2_000), answer("UNSUPPORTED", null, 3_000)];
    const { result } = renderHook(() => useConnectorVoltage(client, true, false, 0, 1_000));
    await act(async () => {
      await Promise.resolve();
    });
    await act(async () => {
      vi.advanceTimersByTime(1_000);
      await Promise.resolve();
    });
    expect(result.current.last?.state).toBe("REFUSED");
    expect(result.current.reading?.millivolts).toBe(12_600);
    await act(async () => {
      vi.advanceTimersByTime(1_000);
      await Promise.resolve();
    });
    expect(result.current.unsupported).toBe(true);
    const asked = client.asked;
    await act(async () => {
      vi.advanceTimersByTime(10_000);
      await Promise.resolve();
    });
    expect(client.asked).toBe(asked);
  });
});
