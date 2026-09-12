import { act, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PassportPanel } from "./components/PassportPanel";
import { LANGUAGE_STORAGE_KEY, passportLabel, setCurrentLanguage } from "./i18n";
import { createVehicleDescription } from "./library";
import {
  createPassportSnapshot,
  type PassportClient,
  type PassportReading,
} from "./passport";
import { usePassportController } from "./usePassportController";

/**
 * The module passport (ADR-0027): the text a module holds in its
 * identification identifiers, shown as it is, under this project's own label
 * with SDD's name beneath as the row's identity. Nothing is parsed out of a
 * part number, nothing is compared, and no word judges what was read.
 */
function reading(partial: Partial<PassportReading>): PassportReading {
  return {
    ecuFamily: "PCM",
    identifier: "0xF111",
    parameter: "ECU Core Assembly Number",
    state: "SUCCEEDED",
    value: "8X23-14C088-AB",
    routeId: "hs-can",
    routeValidation: "SOURCE_BACKED",
    rawResponseHex: "62 F1 11 38 58 32 33",
    negativeResponse: null,
    note: null,
    reason: null,
    ...partial,
  };
}

class ScriptedClient implements PassportClient {
  steps = 0;
  starts = 0;
  finishes = 0;
  families: string[] | undefined;
  plan = 4;
  private snapshot = createPassportSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start(_context: unknown, ecuFamilies?: string[]) {
    this.starts += 1;
    this.families = ecuFamilies;
    this.snapshot = { ...createPassportSnapshot(), state: "RUNNING", planned: this.plan, modules: 2 };
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

describe("the module passport", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("walks every planned identifier and stops asking when the shell says it is finished", async () => {
    vi.useFakeTimers();
    try {
      const client = new ScriptedClient();
      const { result } = renderHook(() =>
        usePassportController(client, createVehicleDescription(), 10),
      );
      await act(async () => {
        await result.current.start(["PCM", "ABS"]);
      });
      expect(client.starts).toBe(1);
      expect(client.families).toEqual(["PCM", "ABS"]);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(200);
      });
      expect(client.steps).toBe(4);
      expect(result.current.running).toBe(false);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(client.steps).toBe(4);
    } finally {
      vi.useRealTimers();
    }
  });

  it("shows each text as the module holds it, under our label with SDD's name beneath, and judges nothing", () => {
    render(
      <PassportPanel
        snapshot={{
          ...createPassportSnapshot(),
          state: "FINISHED",
          planned: 4,
          asked: 4,
          answered: 3,
          modules: 2,
          readings: [
            reading({}),
            reading({ identifier: "0xF18C", parameter: "ECU Serial Number", value: "0000471D3E92A1" }),
            reading({
              identifier: "0xF1A0",
              parameter: "synth_no_text",
              value: "01 02 80",
              note: "not printable text; shown as bytes",
            }),
            reading({
              ecuFamily: "ABS",
              identifier: "0xF188",
              parameter: "ECU Software Number",
              value: null,
              state: "FAILED",
              reason: "no answer within the timeout",
            }),
          ],
        }}
        running={false}
        adapterReady
        surveyed
      />,
    );

    // One table per module, the module named as its heading.
    expect(screen.getByRole("heading", { name: "PCM" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "ABS" })).toBeInTheDocument();
    // Our label, SDD's name beneath it, the text as held.
    expect(screen.getByText("Core assembly part number")).toBeInTheDocument();
    expect(screen.getByText("ECU Core Assembly Number")).toBeInTheDocument();
    expect(screen.getByText("8X23-14C088-AB")).toBeInTheDocument();
    expect(screen.getByText("Serial number")).toBeInTheDocument();
    // An identifier we have no label for shows SDD's name as the label.
    expect(screen.getAllByText("synth_no_text").length).toBeGreaterThan(0);
    expect(screen.getByText("not printable text; shown as bytes")).toBeInTheDocument();
    // A module that said nothing says nothing.
    expect(screen.getByText("no answer")).toBeInTheDocument();
    expect(screen.getByText(/4 of 4 identifier reads over 2 modules, 3 answered/)).toBeInTheDocument();
    for (const verdict of [/outdated/i, /obsolete/i, /wrong part/i, /застаріл/i]) {
      expect(document.body.textContent).not.toMatch(verdict);
    }
  });

  it("labels the identifiers in the interface's language and says what to do first", () => {
    setCurrentLanguage("uk");
    expect(passportLabel("0xF111")).toBe("Номер деталі основного вузла");
    expect(passportLabel("0xF18C")).toBe("Серійний номер");
    expect(passportLabel("0xF190")).toBe("VIN, який зберігає модуль");
    // An identifier SDD names inconsistently across modules has no label of ours.
    expect(passportLabel("0xF108")).toBeNull();
    render(
      <PassportPanel
        snapshot={createPassportSnapshot()}
        running={false}
        adapterReady={false}
        surveyed={false}
      />,
    );
    expect(screen.getByText("Спершу підключи і перевір адаптер або стенд.")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Що кожен модуль каже про себе" })).toBeInTheDocument();
  });
});
