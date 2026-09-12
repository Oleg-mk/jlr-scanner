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
  };
}

const idleHandlers = {
  onToggle: () => {},
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
    expect(screen.getByText("engine speed")).toBeInTheDocument();
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
    expect(screen.getByText("engine speed")).toBeInTheDocument();
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
