import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";
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

/** No adapter on any port; the bench connects on request and stays connected. */
class BenchAdapterClient implements AdapterClient {
  bench = false;
  benchRequests = 0;

  getState(): Promise<AdapterSnapshot> {
    return Promise.resolve(this.bench ? createBenchSnapshot() : createEmptySnapshot());
  }
  discover() {
    return this.getState();
  }
  connect() {
    this.bench = false;
    return this.getState();
  }
  connectBench() {
    this.bench = true;
    this.benchRequests += 1;
    return this.getState();
  }
  disconnect() {
    this.bench = false;
    return this.getState();
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
    // The band at the top and at the bottom, the pill, the panel.
    expect(screen.getAllByText("BENCH · virtual vehicle · synthetic data")).toHaveLength(2);
    expect(screen.getByText("Bench: virtual vehicle")).toBeInTheDocument();
    expect(screen.getByText("Virtual vehicle (bench)")).toBeInTheDocument();
    expect(
      screen.getByText("Bench: virtual vehicle from the library; every value is synthetic."),
    ).toBeInTheDocument();
    // Leaving the bench returns to the ordinary adapter panel.
    fireEvent.click(screen.getByRole("button", { name: "Disconnect" }));
    await waitFor(() => expect(container.querySelector(".app-shell--bench")).toBeNull());
    expect(screen.getByText("Adapter not detected")).toBeInTheDocument();
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
