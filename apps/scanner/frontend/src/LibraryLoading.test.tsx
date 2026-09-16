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
function panel(busy: boolean, restoring = false) {
  return render(
    <LibraryPanel
      snapshot={{ ...createLibrarySnapshot(), message: "Built-in data only." }}
      directory="C:\\library"
      busy={busy}
      restoring={restoring}
      onDirectoryChange={() => {}}
      onChooseDirectory={() => {}}
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

  /**
   * The folder remembered from last time is read on start, and on a large
   * copy that takes tens of seconds. Until 2026-09-16 it disabled both
   * buttons for all of that time, so the owner could not put a newer library
   * in place of the older one it was busy reading. A restore now shows the
   * same count of seconds and takes nothing away.
   */
  it("keeps the choice open while the remembered folder is read on start", () => {
    panel(false, true);
    const status = screen.getByRole("status");
    expect(status.textContent).toMatch(/Reading the folder remembered from last time…\s*0/);
    expect(status.textContent).toMatch(/the newer one wins/);
    expect(screen.getByRole("button", { name: "Choose folder…" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Load library" })).toBeEnabled();
  });

  it("shows the library's own message once the reading is over", () => {
    panel(false);
    expect(screen.getByText("Built-in data only.")).toBeVisible();
    expect(screen.queryByText(/Reading the library…/)).toBeNull();
  });
});
