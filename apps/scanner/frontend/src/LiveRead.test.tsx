import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { LiveReadPanel } from "./components/LiveReadPanel";
import { saveLiveReadCsv } from "./liveRead";
import { createVehicleDescription, type ModuleSurveyEntry } from "./library";
import {
  createLiveReadSnapshot,
  type LiveReadClient,
  type LiveReadRequest,
  type LiveReadSnapshot,
} from "./liveRead";
import { liveCandidates, useLiveReadController } from "./useLiveReadController";

/**
 * Live reading (ADR-0022): the interface owns the timer and nothing else. It
 * asks for one step at a time, never overlaps two, and stops asking the
 * moment the shell says the run is over — at the cap, on an adapter error,
 * or because every entry was dropped.
 */
class ScriptedClient implements LiveReadClient {
  started: LiveReadRequest[] = [];
  steps = 0;
  stops = 0;
  /** The step the shell reports the run has ended on. */
  endsAfter = Number.POSITIVE_INFINITY;
  private snapshot = createLiveReadSnapshot();

  getState() {
    return Promise.resolve(this.snapshot);
  }

  start(request: LiveReadRequest) {
    this.started.push(request);
    this.snapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      entries: request.entries.map((entry) => ({
        ecuFamily: entry.ecuFamily,
        identifier: entry.identifier,
        routeId: "hs-can",
        routeValidation: "SOURCE_BACKED",
        reads: 0,
        failures: 0,
        dropped: false,
        reason: null,
        lastResponseHex: null,
        negativeResponse: null,
      })),
    };
    return Promise.resolve(this.snapshot);
  }

  step() {
    this.steps += 1;
    const ended = this.steps >= this.endsAfter;
    this.snapshot = {
      ...this.snapshot,
      state: ended ? "STOPPED" : "RUNNING",
      stoppedReason: ended ? "the run reached its ten-minute cap" : null,
      samples: this.snapshot.samples + 1,
      rounds: this.snapshot.samples + 1,
      roundMs: 320,
    };
    return Promise.resolve(this.snapshot);
  }

  stop() {
    this.stops += 1;
    this.snapshot = { ...this.snapshot, state: "STOPPED", stoppedReason: "stopped by the tester" };
    return Promise.resolve(this.snapshot);
  }
}

function module(ecuFamily: string, identifiers: string[]): ModuleSurveyEntry {
  return {
    ecuFamily,
    name: `${ecuFamily} module`,
    names: {},
    applicability: "APPLICABLE",
    logicalNetwork: "CAN_HS",
    requestId: "0x7E0",
    responseId: "0x7E8",
    backendRoute: "hs-can",
    pins: "6/14",
    bitrateBps: 500_000,
    protocol: "ISO14229",
    routeValidation: "SOURCE_BACKED",
    identifierRead: { status: "REACHABLE", reasons: [] },
    dtcRead: { status: "REACHABLE", reasons: [] },
    readableIdentifiers: identifiers.map((identifier) => ({
      identifier,
      parameters: [`parameter ${identifier}`],
    })),
    selfTests: [],
  };
}

const idleHandlers = {
  onToggle: () => {},
  onChoose: () => {},
  onClearSet: () => {},
  onStart: () => {},
  onStop: () => {},
  onSaveCsv: () => {},
  saved: null,
};

describe("live reading", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("steps one at a time and stops asking when the shell says the run ended", async () => {
    vi.useFakeTimers();
    try {
      const client = new ScriptedClient();
      client.endsAfter = 3;
      const { result } = renderHook(() =>
        useLiveReadController(client, createVehicleDescription(), 10),
      );
      act(() => {
        result.current.toggle({ ecuFamily: "PCM", identifier: "0xF40C" });
        result.current.toggle({ ecuFamily: "TCM", identifier: "0x1945" });
        // The same entry twice takes it out again.
        result.current.toggle({ ecuFamily: "TCM", identifier: "0x1945" });
      });
      expect(result.current.set).toEqual([{ ecuFamily: "PCM", identifier: "0xF40C" }]);

      await act(async () => {
        await result.current.start();
      });
      expect(client.started[0].entries).toEqual([{ ecuFamily: "PCM", identifier: "0xF40C" }]);
      expect(result.current.running).toBe(true);

      await act(async () => {
        await vi.advanceTimersByTimeAsync(100);
      });
      // Three steps ended the run; the timer must not have asked for a fourth.
      expect(client.steps).toBe(3);
      expect(result.current.running).toBe(false);
      expect(result.current.snapshot.stoppedReason).toBe("the run reached its ten-minute cap");

      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });
      expect(client.steps).toBe(3);
    } finally {
      vi.useRealTimers();
    }
  });

  it("stops the run when the tester stops it", async () => {
    vi.useFakeTimers();
    try {
      const client = new ScriptedClient();
      const { result } = renderHook(() =>
        useLiveReadController(client, createVehicleDescription(), 10),
      );
      act(() => result.current.toggle({ ecuFamily: "PCM", identifier: "0xF40C" }));
      await act(async () => {
        await result.current.start();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(30);
      });
      const stepsWhenStopped = client.steps;
      expect(stepsWhenStopped).toBeGreaterThan(0);
      await act(async () => {
        await result.current.stop();
      });
      await act(async () => {
        await vi.advanceTimersByTimeAsync(200);
      });
      expect(client.stops).toBe(1);
      expect(client.steps).toBe(stepsWhenStopped);
      expect(result.current.snapshot.state).toBe("STOPPED");
    } finally {
      vi.useRealTimers();
    }
  });

  it("offers every identifier the survey lists and caps the set at sixteen", () => {
    const modules = [module("PCM", ["0x01", "0x02"]), module("TCM", ["0x03"])];
    expect(liveCandidates(modules)).toEqual([
      { ecuFamily: "PCM", identifier: "0x01" },
      { ecuFamily: "PCM", identifier: "0x02" },
      { ecuFamily: "TCM", identifier: "0x03" },
    ]);

    const client = new ScriptedClient();
    const { result } = renderHook(() => useLiveReadController(client, createVehicleDescription()));
    act(() => {
      for (let index = 0; index < 20; index += 1) {
        result.current.toggle({ ecuFamily: "PCM", identifier: `0x${index}` });
      }
    });
    expect(result.current.set).toHaveLength(16);
    act(() => result.current.clearSet());
    expect(result.current.set).toEqual([]);
  });

  it("shows the values with what the run has seen, the achieved round, and why an entry was dropped", async () => {
    const snapshot: LiveReadSnapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      rounds: 12,
      samples: 24,
      elapsedMs: 4_200,
      roundMs: 350,
      routeValidation: "SYNTHETIC",
      entries: [
        {
          ecuFamily: "PCM",
          identifier: "0xF40C",
          routeId: "hs-can",
          routeValidation: "SYNTHETIC",
          reads: 12,
          failures: 0,
          dropped: false,
          reason: null,
          lastResponseHex: "62 F4 0C 0B B8",
          negativeResponse: null,
        },
        {
          ecuFamily: "TCM",
          identifier: "0x1945",
          routeId: "hs-can",
          routeValidation: "SYNTHETIC",
          reads: 0,
          failures: 3,
          dropped: true,
          reason: "the module declined: RequestOutOfRange",
          lastResponseHex: null,
          negativeResponse: "RequestOutOfRange",
        },
      ],
      values: [
        {
          ecuFamily: "PCM",
          identifier: "0xF40C",
          name: "engine speed",
          value: "750",
          unit: "rpm",
          state: null,
          note: null,
          raw: 3000,
          minimum: 742,
          maximum: 768,
          samples: 12,
          atMs: 4_100,
        },
      ],
    };
    render(
      <LiveReadPanel
        snapshot={snapshot}
        set={[{ ecuFamily: "PCM", identifier: "0xF40C" }]}
        modules={[module("PCM", ["0xF40C"])]}
        running
        busy={false}
        adapterReady
        bench
        {...idleHandlers}
      />,
    );
    await waitFor(() => expect(screen.getByText("SYNTHETIC")).toBeInTheDocument());
    expect(screen.getAllByText("engine speed")[0]).toBeInTheDocument();
    expect(screen.getByText("750 rpm")).toBeInTheDocument();
    expect(screen.getByText("742 … 768")).toBeInTheDocument();
    // The cadence the run actually achieves, not one it promises.
    expect(screen.getByText(/a round every 350 ms/)).toBeInTheDocument();
    expect(screen.getByText(/the module declined: RequestOutOfRange/)).toBeInTheDocument();
    // A run in progress does not let the set be changed under it.
    expect(screen.getByRole("checkbox")).toBeDisabled();
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
  });

  /**
   * F15, the chart: every chosen parameter with readings is one line over the
   * same stretch of time, and the legend is the key. No dial anywhere.
   */
  it("draws only the rows marked for the chart, and tiles only the rows marked for a tile", () => {
    const snapshot: LiveReadSnapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      rounds: 3,
      samples: 6,
      elapsedMs: 900,
      values: [
        {
          ecuFamily: "PCM",
          identifier: "0xF40C",
          name: "engine speed",
          value: "760",
          unit: "rpm",
          state: null,
          note: null,
          raw: 3040,
          minimum: 742,
          maximum: 768,
          samples: 3,
          atMs: 800,
          series: [
            { atMs: 0, value: 742 },
            { atMs: 400, value: 768 },
            { atMs: 800, value: 760 },
          ],
        },
        {
          ecuFamily: "PCM",
          identifier: "0xDD02",
          name: "battery voltage",
          value: "13.75",
          unit: "V",
          state: null,
          note: null,
          raw: 55,
          minimum: 13.5,
          maximum: 14,
          samples: 3,
          atMs: 900,
          series: [
            { atMs: 100, value: 13.5 },
            { atMs: 500, value: 14 },
            { atMs: 900, value: 13.75 },
          ],
        },
      ],
    };
    render(
      <LiveReadPanel
        snapshot={snapshot}
        set={[{ ecuFamily: "PCM", identifier: "0xF40C" }]}
        modules={[module("PCM", ["0xF40C"])]}
        running
        busy={false}
        adapterReady
        {...idleHandlers}
      />,
    );
    // Nothing marked: the table alone, and a line saying how to get more.
    expect(screen.queryByRole("img", { name: "the run, every marked parameter over time" })).toBeNull();
    expect(screen.queryAllByRole("article")).toHaveLength(0);
    expect(screen.getByText("Mark a row in the table to see it as a tile or on the chart.")).toBeInTheDocument();

    // Mark engine speed for the chart and battery voltage for a tile.
    const chartMarks = screen.getAllByRole("button", { name: "Chart" });
    const tileMarks = screen.getAllByRole("button", { name: "Tile" });
    fireEvent.click(chartMarks[0]);
    fireEvent.click(tileMarks[1]);
    const chart = screen.getByRole("img", { name: "the run, every marked parameter over time" });
    expect(chart.querySelectorAll("polyline")).toHaveLength(1);
    expect(screen.getAllByRole("article")).toHaveLength(1);
    expect(screen.getByRole("article", { name: "battery voltage" })).toBeInTheDocument();
    // The chart has no legend of its own; the table row carries the colour.
    expect(document.querySelector(".live-chart-legend")).toBeNull();
    expect(document.querySelectorAll(".module-table .live-chart-swatch")).toHaveLength(1);
    // And there is no dial anywhere.
    expect(document.querySelector(".gauge")).toBeNull();
  });

  /**
   * The instrument panel (the owner, 2026-09-17): one button turns it on,
   * it sits under the buttons, the speedometer and the tachometer take the
   * speed and the engine speed, and the middle shows what has crossed a
   * limit the person set - or the key readings while nothing has.
   */
  it("shows the instrument panel on request, with what crossed a limit in the middle", () => {
    const value = (
      identifier: string,
      name: string,
      text: string,
      unit: string,
      number: number,
    ) => ({
      ecuFamily: "PCM",
      identifier,
      name,
      value: text,
      unit,
      state: null,
      note: null,
      raw: number,
      minimum: number,
      maximum: number,
      samples: 2,
      atMs: 400,
      series: [
        { atMs: 0, value: number },
        { atMs: 400, value: number },
      ],
    });
    const snapshot: LiveReadSnapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      rounds: 2,
      samples: 8,
      elapsedMs: 400,
      values: [
        value("0xF40C", "Engine speed", "812.00", "rpm", 812),
        value("0xF40D", "Vehicle speed", "66", "kph", 66),
        value("0xF405", "Engine coolant temperature", "97.0", "degC", 97),
        value("0xDD02", "Battery voltage", "13.75", "V", 13.75),
      ],
    };
    // The person's own alarm on the coolant, set on an earlier run.
    window.localStorage.setItem(
      "prowlone.liveLimits",
      JSON.stringify({
        "PCM|0xF405|Engine coolant temperature": {
          warnLow: null,
          warnHigh: 90,
          alarmLow: null,
          alarmHigh: 95,
        },
      }),
    );
    window.localStorage.removeItem("prowlone.liveDashboard");
    try {
      render(
        <LiveReadPanel
          snapshot={snapshot}
          set={[{ ecuFamily: "PCM", identifier: "0xF40C" }]}
          modules={[module("PCM", ["0xF40C"])]}
          running
          busy={false}
          adapterReady
          {...idleHandlers}
        />,
      );
      // Off until asked.
      expect(document.querySelector(".live-dash")).toBeNull();
      const toggle = screen.getByRole("button", { name: "Instrument panel" });
      expect(toggle).toHaveAttribute("aria-pressed", "false");
      fireEvent.click(toggle);
      expect(toggle).toHaveAttribute("aria-pressed", "true");
      // Under the buttons, before the table.
      const panel = document.querySelector(".live-dash");
      expect(panel).not.toBeNull();
      const actions = document.querySelector(".live-marks-actions");
      const table = document.querySelector(".module-table");
      expect(actions!.compareDocumentPosition(panel!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(panel!.compareDocumentPosition(table!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      // The dials read the speed and the engine speed, as whole numbers.
      expect(screen.getByRole("img", { name: "Vehicle speed: 66 km/h" })).toBeInTheDocument();
      expect(screen.getByRole("img", { name: "Engine speed: 812 rpm" })).toBeInTheDocument();
      // The middle: the coolant crossed the person's alarm; the battery, within its limits, is not there.
      expect(screen.getByText("Out of limits")).toBeInTheDocument();
      const cells = document.querySelectorAll(".live-dash-cell");
      expect(cells).toHaveLength(1);
      expect(cells[0].className).toContain("live-dash-cell--alarm");
      expect(cells[0].textContent).toContain("Engine coolant temperature");
      expect(cells[0].textContent).toContain("97.0 °C");
      // Off again on the same button.
      fireEvent.click(toggle);
      expect(document.querySelector(".live-dash")).toBeNull();
    } finally {
      window.localStorage.removeItem("prowlone.liveLimits");
      window.localStorage.removeItem("prowlone.liveDashboard");
    }
  });

  it("shows the key readings in the middle while nothing has crossed a limit, and a dial that is not watched says so", () => {
    const snapshot: LiveReadSnapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      rounds: 1,
      samples: 2,
      elapsedMs: 200,
      values: [
        {
          ecuFamily: "PCM",
          identifier: "0xF405",
          name: "Engine coolant temperature",
          value: "88.0",
          unit: "degC",
          state: null,
          note: null,
          raw: 128,
          minimum: 88,
          maximum: 88,
          samples: 1,
          atMs: 200,
          series: [{ atMs: 200, value: 88 }],
        },
        {
          ecuFamily: "PCM",
          identifier: "0xF40C",
          name: "Engine speed",
          value: "760.00",
          unit: "rpm",
          state: null,
          note: null,
          raw: 3040,
          minimum: 760,
          maximum: 760,
          samples: 1,
          atMs: 200,
          series: [{ atMs: 200, value: 760 }],
        },
      ],
    };
    window.localStorage.setItem("prowlone.liveDashboard", "on");
    try {
      render(
        <LiveReadPanel
          snapshot={snapshot}
          set={[{ ecuFamily: "PCM", identifier: "0xF40C" }]}
          modules={[module("PCM", ["0xF40C"])]}
          running
          busy={false}
          adapterReady
          {...idleHandlers}
        />,
      );
      // Remembered from last time: on without a click.
      expect(document.querySelector(".live-dash")).not.toBeNull();
      expect(screen.getByText("Key readings")).toBeInTheDocument();
      expect(document.querySelectorAll(".live-dash-cell")).toHaveLength(1);
      expect(screen.getByRole("img", { name: "Engine speed: 760 rpm" })).toBeInTheDocument();
      // No speed is being read: the speedometer says so instead of guessing.
      expect(screen.getByRole("img", { name: "Vehicle speed: —" })).toBeInTheDocument();
      expect(screen.getByText("not watched")).toBeInTheDocument();
    } finally {
      window.localStorage.removeItem("prowlone.liveDashboard");
    }
  });

  /**
   * A parameter that reads zero and has never moved says nothing during a
   * run, and a module can have dozens. They are hidden by default behind
   * one button that says the opposite of the current state.
   */
  it("hides rows at zero until asked, and the button says which way it goes", () => {
    const snapshot: LiveReadSnapshot = {
      ...createLiveReadSnapshot(),
      state: "RUNNING",
      rounds: 2,
      samples: 4,
      elapsedMs: 600,
      values: [
        {
          ecuFamily: "PCM",
          identifier: "0xF40C",
          name: "engine speed",
          value: "760",
          unit: "rpm",
          state: null,
          note: null,
          raw: 3040,
          minimum: 742,
          maximum: 768,
          samples: 2,
          atMs: 500,
        },
        {
          ecuFamily: "PCM",
          identifier: "0xD901",
          name: "brake switch 1",
          value: "0",
          unit: "int",
          state: null,
          note: null,
          raw: 0,
          minimum: 0,
          maximum: 0,
          samples: 2,
          atMs: 600,
        },
      ],
    };
    render(
      <LiveReadPanel
        snapshot={snapshot}
        set={[{ ecuFamily: "PCM", identifier: "0xF40C" }]}
        modules={[module("PCM", ["0xF40C"])]}
        running
        busy={false}
        adapterReady
        {...idleHandlers}
      />,
    );
    expect(screen.getAllByText("engine speed").length).toBeGreaterThan(0);
    expect(screen.queryByText("brake switch 1")).toBeNull();
    expect(screen.getByText("1 rows at zero hidden")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Show all" }));
    expect(screen.getByText("brake switch 1")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Only informative" })).toBeInTheDocument();
  });

  it("says what to do first when nothing is surveyed or no adapter is there", () => {
    render(
      <LiveReadPanel
        snapshot={createLiveReadSnapshot()}
        set={[]}
        modules={[]}
        running={false}
        busy={false}
        adapterReady={false}
        {...idleHandlers}
      />,
    );
    expect(
      screen.getByText("Survey the vehicle first: the set is chosen from what the data describes."),
    ).toBeInTheDocument();
    expect(screen.getByText("Connect and verify the adapter, or the bench, first.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Start" })).toBeDisabled();
    expect(screen.getByText("0 of 16 chosen")).toBeInTheDocument();
  });

  it("says the decoder's own words and the reason an entry was dropped in the interface's language", () => {
    setCurrentLanguage("uk");
    render(
      <LiveReadPanel
        snapshot={{
          ...createLiveReadSnapshot(),
          state: "STOPPED",
          stoppedReason: "stopped by the tester",
          entries: [
            {
              ecuFamily: "TCM",
              identifier: "0x1945",
              routeId: "hs-can",
              routeValidation: "SOURCE_BACKED",
              reads: 0,
              failures: 3,
              dropped: true,
              reason: "the module declined: RequestOutOfRange",
              lastResponseHex: null,
              negativeResponse: "RequestOutOfRange",
            },
          ],
          values: [
            {
              ecuFamily: "PCM",
              identifier: "0xF40C",
              name: "engine speed",
              value: null,
              unit: null,
              state: null,
              // The decoder's own words, which are ours and not SDD's.
              note: "raw counts; the catalogue records no scaling",
              raw: 3000,
              minimum: 3000,
              maximum: 3000,
              samples: 1,
              atMs: 10,
            },
          ],
        }}
        set={[]}
        modules={[]}
        running={false}
        busy={false}
        adapterReady
        {...idleHandlers}
      />,
    );
    expect(screen.getByText("сирі відліки; каталог не описує масштабування")).toBeInTheDocument();
    // The protocol's own name for the refusal stays as the standard writes it.
    expect(screen.getByText(/модуль відмовив: RequestOutOfRange/)).toBeInTheDocument();
    expect(screen.getByText(/зупинив тестувальник/)).toBeInTheDocument();
    // SDD's own name for the parameter is English because SDD holds it in
    // English; inventing a translation would be inventing data.
    expect(screen.getAllByText("engine speed")[0]).toBeInTheDocument();
  });

  it("offers the series as a spreadsheet once a run has samples", async () => {
    const saveCsv = vi.fn();
    const { rerender } = render(
      <LiveReadPanel
        snapshot={createLiveReadSnapshot()}
        set={[]}
        modules={[]}
        running={false}
        busy={false}
        adapterReady
        {...idleHandlers}
        onSaveCsv={saveCsv}
      />,
    );
    // Nothing read yet: nothing to save.
    expect(screen.queryByRole("button", { name: "Save the series (CSV)" })).toBeNull();

    rerender(
      <LiveReadPanel
        snapshot={{ ...createLiveReadSnapshot(), state: "STOPPED", samples: 12 }}
        set={[]}
        modules={[]}
        running={false}
        busy={false}
        adapterReady
        {...idleHandlers}
        onSaveCsv={saveCsv}
        saved={{ path: "C:/reports/prowlone-live-1.csv" }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Save the series (CSV)" }));
    expect(saveCsv).toHaveBeenCalledTimes(1);
    expect(
      screen.getByText("Series written to C:/reports/prowlone-live-1.csv"),
    ).toBeInTheDocument();
  });

  it("passes the spreadsheet to the saver as a spreadsheet", async () => {
    const client = {
      samplesCsv: () => Promise.resolve("at_ms,ecu_family\n0,PCM\n"),
      getState: () => Promise.resolve(createLiveReadSnapshot()),
      start: () => Promise.resolve(createLiveReadSnapshot()),
      step: () => Promise.resolve(createLiveReadSnapshot()),
      stop: () => Promise.resolve(createLiveReadSnapshot()),
    };
    // jsdom has no object URLs and no real download; the browser path is
    // stubbed the way Files.test.tsx stubs it.
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    URL.createObjectURL = vi.fn(() => "blob:series");
    URL.revokeObjectURL = vi.fn();
    const path = await saveLiveReadCsv(client, "42");
    // In the browser preview the file is offered as a download and the
    // suggested name comes back.
    expect(path).toBe("prowlone-live-42.csv");
  });
});
