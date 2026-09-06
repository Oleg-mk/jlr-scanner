import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "./App";

describe("a new session", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("restarts the interface once the user confirms", async () => {
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const restart = vi.fn();
    render(<App onNewSession={restart} pollIntervalMs={100_000} />);
    screen.getByRole("button", { name: "New session" }).click();
    await waitFor(() => expect(restart).toHaveBeenCalledTimes(1));
  });

  it("changes nothing when the user cancels", async () => {
    const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
    const restart = vi.fn();
    render(<App onNewSession={restart} pollIntervalMs={100_000} />);
    screen.getByRole("button", { name: "New session" }).click();
    await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
    expect(restart).not.toHaveBeenCalled();
  });
});
