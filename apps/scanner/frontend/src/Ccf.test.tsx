import { act, fireEvent, render, renderHook, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createCcfSnapshot, type CcfClient, type CcfReading } from "./ccf";
import { CcfPanel } from "./components/CcfPanel";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createVehicleDescription } from "./library";
import { useCcfController } from "./useCcfController";

/**
 * The car configuration file (ADR-0028): the values the modules hold, under
 * SDD's group titles, in the interface's language where SDD has it; rows SDD
 * hides behind one switch; a copy's difference shown beside the master's
 * value; nothing judged.
 */
function reading(partial: Partial<CcfReading>): CcfReading {
  return {
    ecuFamily: "RSJB",
    role: "sync",
    block: "CCF",
    parameter: "PARAM_CCF_BRAND",
    group: "GROUP_CCF_BRAND",
    groupTitleEn: "Brand",
    groupTitleRu: "Марка",
    titleEn: "",
    titleRu: "",
    kind: "ENUM",
    display: true,
    scope: "base",
    valueEn: "Jaguar",
    valueRu: "Jaguar",
    optionName: "JAGUAR",
    optionCode: "VS_J",
    raw: 1,
    hex: null,
    note: null,
    ...partial,
  };
}

class ScriptedClient implements CcfClient {
  steps = 0;
  starts = 0;
  plan = 2;
  private snapshot = createCcfSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start() {
    this.starts += 1;
    this.snapshot = { ...createCcfSnapshot(), state: "RUNNING", planned: this.plan, scheme: "did" };
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
    this.snapshot = { ...this.snapshot, state: "FINISHED" };
    return Promise.resolve(this.snapshot);
  }
}

describe("the configuration read", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("walks every planned block read and stops asking when the shell says it is finished", async () => {
    vi.useFakeTimers();
    try {
      const client = new ScriptedClient();
      const { result } = renderHook(() =>
        useCcfController(client, createVehicleDescription(), 10),
      );
      await act(async () => {
        await result.current.start();
      });
      expect(client.starts).toBe(1);
      await act(async () => {
        await vi.advanceTimersByTimeAsync(200);
      });
      expect(client.steps).toBe(2);
      expect(result.current.running).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });

  it("shows the values under SDD's titles, hides what SDD hides behind a switch, and shows a copy's difference", () => {
    render(
      <CcfPanel
        snapshot={{
          ...createCcfSnapshot(),
          state: "FINISHED",
          scheme: "did",
          masterModule: "RSJB",
          planned: 2,
          asked: 2,
          answered: 2,
          hidden: 1,
          readings: [
            reading({}),
            reading({
              parameter: "PARAM_CCF_HEATED_SEATS",
              group: "GROUP_CCF_SEATS",
              groupTitleEn: "Seats",
              groupTitleRu: "Сиденья",
              titleEn: "Heated seats",
              titleRu: "Подогрев сидений",
              kind: "BOOL",
              valueEn: "Fitted",
              valueRu: "Установлено",
              optionName: "TRUE",
              optionCode: null,
            }),
            reading({
              parameter: "PARAM_CCF_TYRE_RADIUS",
              group: "GROUP_CCF_TYRES",
              groupTitleEn: "Tyres",
              groupTitleRu: "",
              kind: "BIN",
              display: false,
              valueEn: "1013",
              valueRu: "1013",
              optionName: null,
              optionCode: null,
              raw: 1013,
            }),
          ],
          differences: [
            {
              block: "CCF",
              parameter: "PARAM_CCF_HEATED_SEATS",
              masterModule: "RSJB",
              masterValue: "TRUE",
              copyModule: "PCM",
              copyValue: "FALSE",
            },
          ],
        }}
        running={false}
        adapterReady
        surveyed
      />,
    );

    expect(screen.getByText("Master copy read from RSJB")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Brand" })).toBeInTheDocument();
    expect(screen.getByText("Jaguar")).toBeInTheDocument();
    expect(screen.getByText("VS_J")).toBeInTheDocument();
    expect(screen.getByText("Heated seats")).toBeInTheDocument();
    expect(screen.getByText("Fitted")).toBeInTheDocument();
    // The copy's difference sits beside the master's value, both shown.
    expect(screen.getByText("differs in PCM: FALSE")).toBeInTheDocument();
    expect(screen.getByText(/1 parameter\(s\) differ/)).toBeInTheDocument();
    // What SDD hides is hidden until asked for.
    expect(screen.queryByText("1013")).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText(/Show the 1 rows SDD's editor hides/));
    expect(screen.getByText("1013")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Tyres" })).toBeInTheDocument();
    // The caveat says nothing here says what a value should be; no other
    // word judges what was read.
    for (const verdict of [/corrupt/i, /wrong/i, /invalid/i, /outdated/i]) {
      expect(document.body.textContent).not.toMatch(verdict);
    }
  });

  it("follows the interface into Russian where SDD has the text, and says what to do first in Ukrainian", () => {
    setCurrentLanguage("ru");
    const { unmount } = render(
      <CcfPanel
        snapshot={{
          ...createCcfSnapshot(),
          state: "FINISHED",
          masterModule: "RSJB",
          readings: [reading({})],
        }}
        running={false}
        adapterReady
        surveyed
      />,
    );
    expect(screen.getByRole("heading", { name: "Марка" })).toBeInTheDocument();
    unmount();

    setCurrentLanguage("uk");
    render(
      <CcfPanel
        snapshot={createCcfSnapshot()}
        running={false}
        adapterReady={false}
        surveyed={false}
      />,
    );
    expect(screen.getByText("Спершу підключи і перевір адаптер або стенд.")).toBeInTheDocument();
    expect(
      screen.getByRole("heading", { name: "Чим авто оснащене і як налаштоване" }),
    ).toBeInTheDocument();
  });
});
