import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LibraryPanel } from "./components/LibraryPanel";
import { createLibrarySnapshot } from "./library";

/**
 * A library of the real size takes seconds to read on a fast machine and
 * tens of seconds on an old one, and until 2026-09-10 the only sign of life
 * was a disabled button. The owner's 2016 Mac read his 177 MB copy with
 * nothing moving on screen and he took the application for hung. The panel
 * now counts the seconds out loud, which is the cheapest possible proof that
 * something is still happening.
 */
function panel(busy: boolean) {
  return render(
    <LibraryPanel
      snapshot={{ ...createLibrarySnapshot(), message: "Built-in data only." }}
      directory="C:\\library"
      busy={busy}
      onDirectoryChange={() => {}}
      onLoad={() => {}}
    />,
  );
}

describe("while the library loads", () => {
  it("says what it is doing, counts the seconds and explains the wait", () => {
    panel(true);
    // One line, so read it whole: the words, the count, and the reason.
    const status = screen.getByRole("status");
    expect(status.textContent).toMatch(/Reading the library…\s*0/);
    expect(status.textContent).toMatch(/tens of seconds on an old one/);
    // The button says so too, and refuses a second load.
    expect(screen.getByRole("button", { name: "Working…" })).toBeDisabled();
  });

  it("shows the library's own message once the reading is over", () => {
    panel(false);
    expect(screen.getByText("Built-in data only.")).toBeVisible();
    expect(screen.queryByText(/Reading the library…/)).toBeNull();
  });
});
