import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  createBatterySnapshot,
  stateOfCharge,
  type BatteryClient,
  type BatteryReadSnapshot,
  type BatteryReading,
} from "./battery";
import { BatteryCard } from "./components/BatteryCard";
import { BatteryPanel } from "./components/BatteryPanel";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createVehicleDescription } from "./library";
import { useBatteryController } from "./useBatteryController";
import { BatteryPrecondition } from "./components/BatteryPrecondition";
import { batteryPreconditionText } from "./batteryFormat";
import { createLiveReadSnapshot } from "./liveRead";
import type { StandardObdValue } from "./standardObd";
import { latestVoltage, type VoltageInputs } from "./voltage";

/**
 * The battery (ADR-0030): the level a person reads at a glance, the three
 * readings a session turns on, the rest in groups, and not one word of
 * verdict anywhere.
 */
function reading(partial: Partial<BatteryReading>): BatteryReading {
  return {
    ecuFamily: "BCM",
    identifier: "0x4028",
    parameter: "Vehicle Battery Estimated State of Charge",
    role: "CHARGE",
    headline: true,
    state: "SUCCEEDED",
    value: "78",
    unit: "pct",
    routeId: "hs-can",
    routeValidation: "SOURCE_BACKED",
    rawResponseHex: "62 40 28 4E",
    negativeResponse: null,
    note: null,
    reason: null,
    ...partial,
  };
}

function snapshot(partial: Partial<BatteryReadSnapshot>): BatteryReadSnapshot {
  return {
    ...createBatterySnapshot(),
    state: "FINISHED",
    planned: 4,
    asked: 4,
    answered: 4,
    modules: 1,
    routeValidation: "SOURCE_BACKED",
    readUnixMs: Date.now(),
    ...partial,
  };
}

class ScriptedClient implements BatteryClient {
  steps = 0;
  starts = 0;
  plan = 2;
  private state = createBatterySnapshot();

  getState() {
    return Promise.resolve(this.state);
  }

  start() {
    this.starts += 1;
    this.state = {
      ...createBatterySnapshot(),
      state: "RUNNING",
      planned: this.plan,
      readUnixMs: Date.now(),
    };
    return Promise.resolve(this.state);
  }

  step() {
    this.steps += 1;
    const done = this.steps >= this.plan;
    this.state = {
      ...this.state,
      state: done ? "FINISHED" : "RUNNING",
      asked: this.steps,
      answered: this.steps,
      readings: [reading({}), reading({ role: "CURRENT", parameter: "Battery current", value: "-1.5", unit: "A", identifier: "0x4090" })].slice(0, this.steps),
      reportAvailable: done,
    };
    return Promise.resolve(this.state);
  }

  finish() {
    this.state = { ...this.state, state: "FINISHED", reportAvailable: true };
    return Promise.resolve(this.state);
  }
}

/** The key's voltage the way the application derives it: from the battery read alone here. */
function voltsOf(snapshot: BatteryReadSnapshot) {
  return latestVoltage({
    battery: snapshot,
    obdValues: [],
    obdAtMs: null,
    obdRouteValidation: "",
    live: createLiveReadSnapshot(),
    liveStartedAtMs: null,
  });
}

afterEach(() => {
  window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  setCurrentLanguage("en");
  vi.useRealTimers();
});

describe("battery", () => {
  it("shows the level, the readings a session turns on, and no verdict", () => {
    render(
      <BatteryCard
        snapshot={snapshot({
          readings: [
            reading({}),
            reading({
              identifier: "0x402A",
              parameter: "Vehicle Battery Voltage",
              role: "VOLTAGE",
              value: "12.6",
              unit: "V",
            }),
            reading({
              identifier: "0x4090",
              parameter: "Battery current",
              role: "CURRENT",
              value: "-1.5",
              unit: "A",
            }),
            reading({
              identifier: "0x4029",
              parameter: "Vehicle Battery Temperature - Estimated",
              role: "TEMPERATURE",
              value: "21",
              unit: "degC",
            }),
            reading({
              identifier: "0x4025",
              parameter: "Average Vehicle Quiesent Current - Previous 24 Hours (mA)",
              role: "DRAIN",
              headline: true,
              value: "23",
              unit: null,
            }),
          ],
        })}
        busy={false}
        running={false}
        onRead={() => undefined}
        onStop={() => undefined}
        volts={voltsOf(
          snapshot({
            readings: [
              reading({
                identifier: "0x402A",
                parameter: "Vehicle Battery Voltage",
                role: "VOLTAGE",
                value: "12.6",
                unit: "V",
              }),
            ],
          }),
        )}
        disabledReason={null}
      />,
    );

    // The voltage is on the cell, in bold; the level is the cell's own fill
    // and the figure it carries for a reader who cannot see it.
    const cell = screen.getByLabelText("State of charge: 78%");
    expect(screen.getByText("12.6 V")).toBeVisible();
    expect(cell.querySelector(".battery-key__fill")).toHaveStyle({ width: "78%" });
    // Everything else the monitor holds is the panel's.
    expect(screen.queryByText("-1.5 A")).toBeNull();
    expect(screen.queryByText("21 °C")).toBeNull();
    expect(screen.queryByRole("heading", { name: "Parked drain" })).toBeNull();
    expect(screen.queryByText("23")).toBeNull();
    // Not one word of judgement anywhere on the key.
    expect(cell.parentElement?.textContent ?? "").not.toMatch(
      /good|bad|poor|healthy|weak|replace|failing|warning|critical/i,
    );
  });

  it("says when the reading was taken and that a bench value is a bench value", () => {
    vi.useFakeTimers();
    const taken = Date.now() - 7 * 60_000;
    // The key is the cell and nothing else; when a reading was taken, and
    // that it came off a bench rather than a car, are the panel's to say.
    render(
      <BatteryPanel
        snapshot={snapshot({
          readings: [reading({ routeValidation: "SYNTHETIC" })],
          routeValidation: "SYNTHETIC",
          readUnixMs: taken,
        })}
        running={false}
        adapterReady
        surveyed
        bench
      />,
    );
    expect(screen.getByText(/read 7 min ago/)).toBeVisible();
    expect(
      screen.getByText("Bench values: nothing here was measured on a car."),
    ).toBeVisible();
  });

  it("draws no level where the data names no state of charge", () => {
    const rows = [
      reading({
        identifier: "0x4027",
        parameter: "Battery Time in Service (Days)",
        role: "HISTORY",
        headline: true,
        value: "1287",
        unit: "int",
      }),
    ];
    expect(stateOfCharge(snapshot({ readings: rows }))).toBeNull();
    render(
      <BatteryCard
        snapshot={snapshot({ readings: rows })}
        busy={false}
        running={false}
        onRead={() => undefined}
        onStop={() => undefined}
        disabledReason={null}
      />,
    );
    expect(screen.getByLabelText("State of charge: not read")).toBeVisible();
    // The row itself is the panel's, not the card's.
    expect(screen.queryByText("1287")).toBeNull();
  });

  it("says why it cannot read instead of hiding the button", () => {
    render(
      <BatteryCard
        snapshot={createBatterySnapshot()}
        busy={false}
        running={false}
        onRead={() => undefined}
        onStop={() => undefined}
        disabledReason="Connect the adapter first."
      />,
    );
    const button = screen.getByLabelText("State of charge: not read");
    expect(button).toBeDisabled();
    expect(button).toHaveAttribute("title", "Connect the adapter first.");
  });

  it("walks the plan one read at a time and keeps what was read", async () => {
    vi.useFakeTimers();
    const client = new ScriptedClient();
    const { result } = renderHook(() =>
      useBatteryController(client, createVehicleDescription(), 5),
    );

    await act(async () => {
      await result.current.start();
    });
    expect(client.starts).toBe(1);
    expect(result.current.running).toBe(true);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(20);
    });
    expect(client.steps).toBe(client.plan);
    expect(result.current.snapshot.state).toBe("FINISHED");
    expect(result.current.snapshot.readings).toHaveLength(client.plan);
  });

  it("words the card in the user language", () => {
    setCurrentLanguage("uk");
    render(
      <BatteryCard
        snapshot={snapshot({ readings: [reading({})] })}
        busy={false}
        running={false}
        onRead={() => undefined}
        onStop={() => undefined}
        disabledReason={null}
      />,
    );
    expect(screen.getByText("Прочитати")).toBeVisible();
    expect(screen.getByLabelText("Рівень заряду: 78%")).toBeVisible();
  });

  it("offers every row in the panel, including the ones that answered nothing", () => {
    render(
      <BatteryPanel
        snapshot={snapshot({
          readings: [
            reading({}),
            reading({
              identifier: "0x4058",
              parameter: "Battery Type",
              role: "CONFIGURATION",
              headline: false,
              value: null,
              state: "FAILED",
              reason: "the module answered nothing",
            }),
          ],
        })}
        running={false}
        adapterReady
        surveyed
      />,
    );
    expect(screen.queryByText("the module answered nothing")).toBeNull();
    fireEvent.click(screen.getByRole("checkbox"));
    expect(screen.getByText("the module answered nothing")).toBeVisible();
    // And the groups are here, under our own short headings.
    expect(screen.getByRole("heading", { name: "Declared" })).toBeVisible();
  });
});

/**
 * A low battery is a precondition of the session (ADR-0030, amendments of
 * 2026-09-19): said in SDD's own terms, with the number the shell carries
 * across from the data layer, from whichever read last brought a voltage,
 * and never when there is nothing to say.
 */
describe("the low-battery precondition (ADR-0030, 2026-09-19)", () => {
  const voltage = (value: string) =>
    reading({
      role: "VOLTAGE",
      identifier: "0x402A",
      parameter: "Vehicle Battery Voltage",
      value,
      unit: "V",
    });
  const inputs = (battery: BatteryReadSnapshot, extra: Partial<VoltageInputs> = {}): VoltageInputs => ({
    battery,
    obdValues: [],
    obdAtMs: null,
    obdRouteValidation: "",
    live: createLiveReadSnapshot(),
    liveStartedAtMs: null,
    ...extra,
  });

  it("says so when the battery read is under SDD's low band, in SDD's number", () => {
    const low = snapshot({ readings: [voltage("11.2")], sddLowVoltageMaxMv: 11_600 });
    const read = latestVoltage(inputs(low));
    expect(read).toMatchObject({ volts: 11.2, text: "11.2", source: "battery", module: "BCM" });
    const text = batteryPreconditionText(read, 11_600, Date.now());
    expect(text).toMatch(/11\.2 V/);
    expect(text).toMatch(/11\.6 V/);
    expect(text).toMatch(/from the battery monitor/);
    expect(text).toMatch(/external power supply/);
    expect(text).toMatch(/read just now/);
    render(<BatteryPrecondition reading={read} sddLowVoltageMaxMv={11_600} />);
    expect(screen.getByRole("status")).toHaveTextContent(/Connect an external power supply/);
  });

  it("says nothing for a healthy reading, for no reading, and for no band", () => {
    const healthy = latestVoltage(inputs(snapshot({ readings: [voltage("12.6")], sddLowVoltageMaxMv: 11_600 })));
    expect(batteryPreconditionText(healthy, 11_600, Date.now())).toBeNull();
    expect(latestVoltage(inputs(snapshot({ readings: [], sddLowVoltageMaxMv: 11_600 })))).toBeNull();
    // No band known: the interface holds no number of its own to fall back on.
    const low = latestVoltage(inputs(snapshot({ readings: [voltage("11.2")] })));
    expect(batteryPreconditionText(low, 0, Date.now())).toBeNull();
    const { container } = render(<BatteryPrecondition reading={healthy} sddLowVoltageMaxMv={11_600} />);
    expect(container.querySelector(".battery-precondition")).toBeNull();
  });

  it("takes the freshest of the three reads, and names where it came from", () => {
    // A car whose battery monitor reports no voltage at all, like the
    // owner's X250: the OBD read and the live read are what there is.
    const none = snapshot({ readings: [], sddLowVoltageMaxMv: 11_600 });
    const obd: StandardObdValue = {
      pid: "0x42",
      name: "control module voltage",
      unit: "V",
      value: "11.4",
      number: 11.4,
      rawHex: "2C 88",
      kind: "number",
    };
    const live = {
      ...createLiveReadSnapshot(),
      values: [
        {
          ecuFamily: "PCM",
          identifier: "0xDD02",
          name: "Control module voltage",
          value: "13.50",
          unit: "V",
          state: null,
          note: null,
          raw: 270,
          minimum: 13.5,
          maximum: 13.5,
          samples: 1,
          atMs: 2_000,
        },
      ],
    };
    // The OBD read is the later one: it is the voltage, and it is low.
    const later = latestVoltage(
      inputs(none, { obdValues: [obd], obdAtMs: 10_000, obdRouteValidation: "SOURCE_BACKED", live, liveStartedAtMs: 1_000 }),
    );
    expect(later).toMatchObject({ volts: 11.4, source: "obd" });
    expect(batteryPreconditionText(later, 11_600, 10_000)).toMatch(/from the OBD read/);
    // The live read is the later one: it is the voltage, healthy, and named.
    const live_later = latestVoltage(
      inputs(none, { obdValues: [obd], obdAtMs: 1_000, obdRouteValidation: "SOURCE_BACKED", live, liveStartedAtMs: 10_000 }),
    );
    expect(live_later).toMatchObject({ volts: 13.5, source: "live", module: "PCM", atMs: 12_000 });
    expect(batteryPreconditionText(live_later, 11_600, 12_000)).toBeNull();
    // A reading with no known time never outranks one with a time.
    const timeless = latestVoltage(inputs(none, { obdValues: [obd], obdAtMs: null, obdRouteValidation: "", live, liveStartedAtMs: 5_000 }));
    expect(timeless?.source).toBe("live");
  });

  it("marks a bench reading as synthetic, like everything else the bench answers", () => {
    const low = latestVoltage(
      inputs(snapshot({ readings: [voltage("11.2")], sddLowVoltageMaxMv: 11_600, routeValidation: "SYNTHETIC" })),
    );
    expect(low?.synthetic).toBe(true);
    expect(batteryPreconditionText(low, 11_600, Date.now())).toMatch(/bench, synthetic/);
  });
});
