import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LibraryPanel } from "./components/LibraryPanel";
import { hasTauriRuntime, pickDirectory, saveTextFile } from "./files";
import type { LibrarySnapshot } from "./library";

const idleLibrary: LibrarySnapshot = {
  state: "NOT_LOADED",
  directory: null,
  manifestsLoaded: 0,
  manifestsFailed: 0,
  sources: 0,
  records: 0,
  failures: [],
  message: "Built-in data only.",
  issue: null,
};

describe("saving files and choosing folders", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("offers a download in the browser, under the suggested name", async () => {
    const click = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
    URL.createObjectURL = vi.fn(() => "blob:report");
    URL.revokeObjectURL = vi.fn();
    let downloadName = "";
    const originalCreate = document.createElement.bind(document);
    vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      const element = originalCreate(tag);
      if (tag === "a") {
        Object.defineProperty(element, "download", {
          set: (value: string) => {
            downloadName = value;
          },
          get: () => downloadName,
        });
      }
      return element;
    });

    expect(hasTauriRuntime()).toBe(false);
    await expect(saveTextFile("prowlone-session-1.json", "{}")).resolves.toBe(
      "prowlone-session-1.json",
    );
    expect(click).toHaveBeenCalledTimes(1);
    expect(downloadName).toBe("prowlone-session-1.json");
  });

  it("cannot choose a folder in the browser, and the panel does not offer to", async () => {
    await expect(pickDirectory()).resolves.toBeNull();
    render(
      <LibraryPanel
        snapshot={idleLibrary}
        directory=""
        busy={false}
        onDirectoryChange={() => {}}
        onLoad={() => {}}
      />,
    );
    expect(screen.queryByRole("button", { name: "Choose folder…" })).toBeNull();
  });

  it("offers the folder dialog when the shell provides one", () => {
    const choose = vi.fn();
    render(
      <LibraryPanel
        snapshot={idleLibrary}
        directory=""
        busy={false}
        onDirectoryChange={() => {}}
        onChooseDirectory={choose}
        onLoad={() => {}}
      />,
    );
    screen.getByRole("button", { name: "Choose folder…" }).click();
    expect(choose).toHaveBeenCalledTimes(1);
  });
});
