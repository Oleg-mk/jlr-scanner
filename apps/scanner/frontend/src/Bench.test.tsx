import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
import { useAdapterController } from "./useAdapterController";
import {
  createBenchSnapshot,
  createEmptySnapshot,
  type AdapterClient,
  type AdapterSnapshot,
} from "./adapter";
import {
  createSessionReportSnapshot,
  type SessionReportClient,
  type SessionReportSnapshot,
} from "./sessionReport";

/**
 * A machine where looking for adapters takes as long as it likes. Windows is
 * such a machine: enumerating the ports is not instant, and the panel looks
 * for them every second and a half.
 */
class SlowDiscoveryClient implements AdapterClient {
  private answerPoll: ((snapshot: AdapterSnapshot) => void) | null = null;
  bench = false;

  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(this.bench ? createBenchSnapshot(1) : createEmptySnapshot());
  }
  discover(): Promise<AdapterSnapshot> {
    return new Promise((resolve) => {
      this.answerPoll = resolve;
    });
  }
  connect(): Promise<AdapterSnapshot> {
    this.bench = false;
    return this.getState();
  }
  connectBench(): Promise<AdapterSnapshot> {
    this.bench = true;
    return this.getState();
  }
  disconnect(): Promise<AdapterSnapshot> {
    this.bench = false;
    return this.getState();
  }
  /** Let the poll that is out answer at last, with what it found then. */
  finishPoll() {
    this.answerPoll?.(createEmptySnapshot());
    this.answerPoll = null;
  }
}

/** No adapter on any port; the bench connects on request and stays connected. */
class BenchAdapterClient implements AdapterClient {
  bench = false;
  benchRequests = 0;
  scenario: number | null = null;

  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(
      this.bench ? createBenchSnapshot(this.scenario ?? 1) : createEmptySnapshot(),
    );
  }
  discover() {
    return this.getState();
  }
  connect() {
    this.bench = false;
    return this.getState();
  }
  connectBench(scenario: number) {
    this.bench = true;
    this.benchRequests += 1;
    this.scenario = scenario;
    return this.getState();
  }
  disconnect() {
    this.bench = false;
    return this.getState();
  }
}

/**
 * A bench that takes its time. Building the virtual vehicle reads the
 * library once per surveyed module, so on a full car it is not instant, and
 * what the person sees while it happens is the whole of this test.
 */
class HeldBenchClient implements AdapterClient {
  private release: (() => void) | null = null;

  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(createEmptySnapshot());
  }
  discover() {
    return this.getState();
  }
  connect() {
    return this.getState();
  }
  connectBench(): Promise<AdapterSnapshot> {
    return new Promise((resolve) => {
      this.release = () => resolve(createBenchSnapshot(1));
    });
  }
  disconnect() {
    return this.getState();
  }
  /** Let the bench finish building at last. */
  finish() {
    this.release?.();
    this.release = null;
  }
}

class FixedReportClient implements SessionReportClient {
  constructor(private readonly state: SessionReportSnapshot) {}
  getState() {
    return Promise.resolve(this.state);
  }
  getReportJson() {
    return Promise.resolve('{"schema":"jlr-scanner.session-report","session_mode":"bench"}');
  }
}

describe("the bench (ADR-0020)", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("connects from the empty adapter panel and colours the whole screen", async () => {
    const client = new BenchAdapterClient();
    const { container } = render(<App client={client} pollIntervalMs={100_000} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect the bench (virtual vehicle)" }),
    );
    await waitFor(() => expect(container.querySelector(".app-shell--bench")).not.toBeNull());
    expect(client.benchRequests).toBe(1);
    // The badge in the header, in the bench's own colour, with the scenario
    // in a circle on its right - the bands are gone (the owner, 2026-09-18);
    // the panel; the whole screen tinted. The scenario is named, because
    // what the screen shows depends on it.
    expect(client.scenario).toBe(1);
    expect(screen.queryByText(/BENCH ·/)).toBeNull();
    // One word and a colour: a badge names the thing and the dot carries the
    // state, so that the header keeps the same geometry in every language
    // (2026-09-18).
    expect(screen.getAllByText("Bench").length).toBeGreaterThan(0);
    expect(screen.getByLabelText("Scenario 1")).toHaveTextContent("1");
    expect(screen.getByText("Virtual vehicle (bench)")).toBeInTheDocument();
    expect(
      screen.getByText("Bench: virtual vehicle from the library; every value is synthetic."),
    ).toBeInTheDocument();
    // Leaving the bench returns to the ordinary adapter panel.
    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(container.querySelector(".app-shell--bench")).toBeNull());
    expect(screen.getByText("Adapter not detected")).toBeInTheDocument();
  });

  it("says what it is connecting and counts the seconds while it does", async () => {
    /*
     * The card used to fall through to the empty state while a connection
     * was being made: a button reading "detect adapter" and nothing else,
     * for as long as the build took. The owner sat in front of it for
     * minutes and took the application for hung (2026-09-18). It now names
     * what is being connected and counts the seconds, as the library load
     * and the module survey already do.
     */
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const client = new HeldBenchClient();
      const { container } = render(<App client={client} pollIntervalMs={100_000} />);
      fireEvent.click(
        await screen.findByRole("button", { name: "Connect the bench (virtual vehicle)" }),
      );

      await waitFor(() =>
        expect(container.querySelector(".adapter-panel .library-status")).not.toBeNull(),
      );
      const status = container.querySelector(".adapter-panel .library-status");
      expect(status?.textContent).toMatch(/Connecting the bench…\s*\d+\s*sec/);
      // The button that does nothing new is not what a person waits in front of.
      expect(screen.queryByRole("button", { name: "Detect adapter" })).toBeNull();
      // And the badge in the header says the same rather than "no adapter".
      // The badge in the header names the state in as few words as it can,
      // so that the row keeps one geometry in every language (2026-09-18);
      // which of the two is being connected is on the card.
      expect(container.querySelector(".app-header .status-badge")?.textContent).toMatch(
        /Connecting…/,
      );

      await act(async () => {
        await vi.advanceTimersByTimeAsync(3_000);
      });
      const moved = container.querySelector(".adapter-panel .library-status");
      expect(moved?.textContent).toMatch(/Connecting the bench…\s*[1-9]\d*\s*sec/);

      // Built at last: the ordinary bench panel, and no counter left running.
      await act(async () => {
        client.finish();
      });
      await waitFor(() => expect(screen.getByText("Virtual vehicle (bench)")).toBeInTheDocument());
      expect(container.querySelector(".adapter-panel .library-status")).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it("names a failed bench call as the bench's failure, and offers the bench again", async () => {
    // The window was restarting, or the shell answered with an error: the
    // panel used to answer with advice about a USB cable (2026-09-17).
    const client = new BenchAdapterClient();
    client.connectBench = () => Promise.reject(new Error("the window was restarting"));
    render(<App client={client} pollIntervalMs={100_000} />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Connect the bench (virtual vehicle)" }),
    );
    expect((await screen.findAllByText("The bench could not be connected.")).length).toBeGreaterThan(0);
    expect(screen.getByText("Try the bench again; it needs no adapter.")).toBeInTheDocument();
    expect(screen.queryByText("Check the USB connection and try again.")).toBeNull();
    expect(screen.getByText("the window was restarting")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Connect the bench (virtual vehicle)" }),
    ).toBeInTheDocument();
  });

  it("connects on the scenario the tester chose, and 0 says the vehicle is healthy", async () => {
    const client = new BenchAdapterClient();
    render(<App client={client} pollIntervalMs={100_000} />);
    fireEvent.change(await screen.findByLabelText("Scenario"), { target: { value: "0" } });
    fireEvent.click(screen.getByRole("button", { name: "Connect the bench (virtual vehicle)" }));
    await waitFor(() => expect(client.scenario).toBe(0));
    expect(
      await screen.findByText("0 — a vehicle in good order, no fault codes"),
    ).toBeInTheDocument();
    expect(screen.getByLabelText("Scenario 0")).toHaveTextContent("0");
  });

  it("shows the report on screen and offers no file while on the bench", async () => {
    const client = new BenchAdapterClient();
    client.bench = true;
    render(
      <App
        client={client}
        sessionReportClient={
          new FixedReportClient({
            ...createSessionReportSnapshot(),
            moduleReads: 1,
            reportAvailable: true,
            mode: "bench",
          })
        }
        pollIntervalMs={100_000}
      />,
    );
    await screen.findByText("Virtual vehicle (bench)");
    expect(screen.queryByRole("button", { name: "Save session report" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Save capture" })).toBeDisabled();
    expect(screen.getAllByText("Bench: nothing is saved.").length).toBeGreaterThan(0);
    expect(screen.getByText("Review the report on screen")).toBeInTheDocument();
    const show = await screen.findByRole("button", { name: "Show the report on screen" });
    await waitFor(() => expect(show).toBeEnabled());
    fireEvent.click(show);
    const preview = await screen.findByLabelText("Session report (bench, not saved)");
    expect(preview.textContent).toContain('"session_mode":"bench"');
    fireEvent.click(screen.getByRole("button", { name: "Hide the report" }));
    expect(screen.queryByLabelText("Session report (bench, not saved)")).toBeNull();
  });

  it("asks for a new session before the bench joins a session with real records", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    const client = new BenchAdapterClient();
    render(
      <App
        client={client}
        sessionReportClient={
          new FixedReportClient({
            ...createSessionReportSnapshot(),
            captures: 1,
            reportAvailable: true,
            mode: "real",
          })
        }
        pollIntervalMs={100_000}
      />,
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save session report" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Connect the bench (virtual vehicle)" }));
    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(client.benchRequests).toBe(0);
    expect(confirm.mock.calls[0][0]).toContain("A session is either on the bench or on a car");
  });

  it("connects the bench and restarts once the user agrees to a new session", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const restart = vi.fn();
    const client = new BenchAdapterClient();
    render(
      <App
        client={client}
        onNewSession={restart}
        sessionReportClient={
          new FixedReportClient({
            ...createSessionReportSnapshot(),
            captures: 1,
            reportAvailable: true,
            mode: "real",
          })
        }
        pollIntervalMs={100_000}
      />,
    );
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Save session report" })).toBeEnabled(),
    );
    fireEvent.click(screen.getByRole("button", { name: "Connect the bench (virtual vehicle)" }));
    await waitFor(() => expect(restart).toHaveBeenCalledTimes(1));
    expect(client.benchRequests).toBe(1);
  });
});

/**
 * The panel looks for adapters every second and a half, and until
 * 2026-09-16 that poll and the buttons shared one "busy" flag: a click that
 * landed while the poll was out was dropped in silence — no action, no
 * error, the panel exactly as it was. The owner could not connect the bench
 * at all, and there was nothing on screen to say why.
 */
describe("a click while the machine is being searched", () => {
  it("connects the bench anyway, and the late answer does not paint over it", async () => {
    const client = new SlowDiscoveryClient();
    const { result } = renderHook(() => useAdapterController(client, 100_000));
    // The poll that runs on mount is out and has not answered.
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.snapshot.state).not.toBe("CONNECTED");

    await act(async () => {
      await result.current.connectBench(1);
    });
    expect(result.current.snapshot.state).toBe("CONNECTED");

    // The poll answers at last, with the picture from before the click.
    await act(async () => {
      client.finishPoll();
      await Promise.resolve();
    });
    expect(result.current.snapshot.state).toBe("CONNECTED");
  });
});
