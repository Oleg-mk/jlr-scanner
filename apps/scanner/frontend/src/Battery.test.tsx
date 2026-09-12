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
        disabledReason={null}
      />,
    );

    expect(screen.getByText("78%")).toBeVisible();
    expect(screen.getByText("12.6 V")).toBeVisible();
    expect(screen.getByText("-1.5 A")).toBeVisible();
    expect(screen.getByText("21 °C")).toBeVisible();
    // The card is the face only: the groups are the panel's.
    expect(screen.queryByRole("heading", { name: "Parked drain" })).toBeNull();
    expect(screen.queryByText("23")).toBeNull();
    // Not one word of judgement over any reading. The caveat below the card
    // is the one place the word "good" may appear, and only to say that the
    // application does not use it.
    const card = screen.getByLabelText(/State of charge/).closest("section");
    const readings = Array.from(
      card?.querySelectorAll(
        ".battery-card__tiles, .battery-card__group, .battery-card__reading",
      ) ?? [],
    )
      .map((element) => element.textContent ?? "")
      .join(" ");
    expect(readings).not.toMatch(
      /good|bad|poor|healthy|weak|replace|failing|warning|critical/i,
    );
    const caveat = card?.querySelector(".battery-card__note")?.textContent ?? "";
    expect(caveat).toMatch(/nothing here is a verdict/);
  });

  it("says when the reading was taken and that a bench value is a bench value", () => {
    vi.useFakeTimers();
    const taken = Date.now() - 7 * 60_000;
    render(
      <BatteryCard
        snapshot={snapshot({
          readings: [reading({ routeValidation: "SYNTHETIC" })],
          routeValidation: "SYNTHETIC",
          readUnixMs: taken,
        })}
        busy={false}
        running={false}
        onRead={() => undefined}
        onStop={() => undefined}
        disabledReason={null}
      />,
    );
    expect(screen.getByText("read 7 min ago")).toBeVisible();
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
    const button = screen.getByRole("button", { name: "Read the battery" });
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
    expect(screen.getByRole("heading", { name: "Акумулятор" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Прочитати ще раз" })).toBeVisible();
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
