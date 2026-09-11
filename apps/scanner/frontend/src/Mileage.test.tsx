import { act, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { MileagePanel } from "./components/MileagePanel";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createVehicleDescription } from "./library";
import {
  createMileageSnapshot,
  type MileageClient,
  type MileageReading,
} from "./mileage";
import { useMileageController } from "./useMileageController";

/**
 * The odometer read from every module (ADR-0024). The table reports what each
 * module answered and does the subtraction; it never says what a difference
 * means, a module that stayed silent is silent and not a zero, and the
 * reference is the highest running total rather than the cluster's — the
 * cluster being precisely what gets rewritten.
 */
function reading(partial: Partial<MileageReading>): MileageReading {
  return {
    ecuFamily: "ABS",
    identifier: "0xDD01",
    parameter: "Total distance",
    kind: "CURRENT",
    state: "SUCCEEDED",
    value: "268905",
    unit: "km",
    number: 268_905,
    raw: 268_905,
    difference: 0,
    routeId: "hs-can",
    routeValidation: "SOURCE_BACKED",
    rawResponseHex: "62 DD 01 00 04 1A 19",
    negativeResponse: null,
    note: null,
    reason: null,
    ...partial,
  };
}

class ScriptedClient implements MileageClient {
  steps = 0;
  starts = 0;
  finishes = 0;
  plan = 3;
  private snapshot = createMileageSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    this.starts += 1;
    this.snapshot = { ...createMileageSnapshot(), state: "RUNNING", planned: this.plan };
    return Promise.resolve(this.snapshot);
  }

  step() {
    this.steps += 1;
    const done = this.steps >= this.plan;
    this.snapshot = {
      ...this.snapshot,
      state: done ? "FINISHED" : "RUNNING",
      asked: this.steps,
      answered: this.steps,
    };
    return Promise.resolve(this.snapshot);
  }

  finish() {
    this.finishes += 1;
    this.snapshot = { ...this.snapshot, state: "FINISHED" };
    return Promise.resolve(this.snapshot);
  }
}

describe("the mileage survey", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("walks every planned module and stops asking when the shell says it is finished", async () => {
    vi.useFakeTimers();
    try {
      const client = new ScriptedClient();
      const { result } = renderHook(() =>
        useMileageController(client, createVehicleDescription(), 10),
      );
      await act(async () => {
        await result.current.start();
      });
      expect(client.starts).toBe(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(200);
      });
      expect(client.steps).toBe(3);
      expect(result.current.running).toBe(false);
      // Nothing more is asked once the survey is done.
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(client.steps).toBe(3);
    } finally {
      vi.useRealTimers();
    }
  });

  it("reports each module and the difference, and never says what it means", () => {
    render(
      <MileagePanel
        snapshot={{
          ...createMileageSnapshot(),
          state: "FINISHED",
          planned: 4,
          asked: 4,
          answered: 3,
          highest: 268_905,
          highestModule: "ABS",
          unit: "km",
          readings: [
            reading({}),
            reading({ ecuFamily: "IPC", identifier: "0x61BB", value: "142310", number: 142_310, difference: -126_595 }),
            reading({
              ecuFamily: "TCM",
              value: null,
              number: null,
              raw: null,
              unit: null,
              difference: null,
              state: "FAILED",
              reason: "no answer within the timeout",
            }),
            reading({
              ecuFamily: "TCM",
              identifier: "0x1EC2",
              parameter: "Gearbox stall event history  -  Odometer reading",
              kind: "EVENT",
              value: "271400",
              number: 271_400,
              difference: 2_495,
            }),
          ],
        }}
        running={false}
        adapterReady
        surveyed
      />,
    );

    // The highest running total is the reference, and it is named.
    expect(screen.getByText(/The highest reading on this car/)).toBeInTheDocument();
    expect(screen.getByText("the highest")).toBeInTheDocument();
    // The subtraction is done for the reader, sign and all.
    expect(screen.getByText("−126 595")).toBeInTheDocument();
    // An event stamped above the highest total is a positive number.
    expect(screen.getByText("+2 495")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Recorded at an event" })).toBeInTheDocument();
    // A module that said nothing says nothing, not zero.
    expect(screen.getByText("no answer")).toBeInTheDocument();
    expect(screen.queryByText("0 km")).not.toBeInTheDocument();
    // The honest causes are stated; no word for the dishonest one appears.
    expect(screen.getByText(/These are readings, not a conclusion/)).toBeInTheDocument();
    for (const verdict of [/rolled/i, /tamper/i, /fraud/i, /скруч/i]) {
      expect(document.body.textContent).not.toMatch(verdict);
    }
  });

  it("says what to do first, and says it in the interface's language", () => {
    setCurrentLanguage("uk");
    render(
      <MileagePanel
        snapshot={createMileageSnapshot()}
        running={false}
        adapterReady={false}
        surveyed={false}
      />,
    );
    expect(screen.getByText("Спершу підключи і перевір адаптер або стенд.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Що кожен модуль каже про пробіг" })).toBeInTheDocument();
  });
});
