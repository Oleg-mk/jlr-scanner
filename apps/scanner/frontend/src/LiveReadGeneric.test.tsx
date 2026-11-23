import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LiveReadPanel } from "./components/LiveReadPanel";
import type { ModuleSurveyEntry, ReadableIdentifierSummary } from "./library";
import { createLiveReadSnapshot } from "./liveRead";
import { isVehicleQualified } from "./useLiveReadController";

/**
 * SDD's generic identifiers (ADR-0044): the catalogue's entries tied to no
 * module and no car. As SDD does, the chooser offers them to every module
 * without claiming the module holds them - found by name, added to the
 * set, shown beside the module's own list and marked, declined by the
 * module at read time. Nothing is sent to find them.
 */

const pcm: ModuleSurveyEntry = {
  ecuFamily: "PCM",
  name: null,
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
  readableIdentifiers: [
    { identifier: "0xDD01", parameters: ["Odometer"], quantity: true, qualification: "vehicle" },
  ],
  selfTests: [],
};

const generic: ReadableIdentifierSummary[] = [
  { identifier: "0xF411", parameters: ["Absolute throttle position"], quantity: true, qualification: "generic" },
  { identifier: "0xF447", parameters: ["Absolute throttle position B"], quantity: true, qualification: "generic" },
  { identifier: "0x032F", parameters: ["Actual variable valve timing position  -  Exhaust valve  -  Bank 1"], quantity: true, qualification: "generic" },
];

function panel(set: Array<{ ecuFamily: string; identifier: string }>, onToggle = vi.fn()) {
  const view = render(
    <LiveReadPanel
      snapshot={createLiveReadSnapshot()}
      set={set}
      modules={[pcm]}
      generic={generic}
      running={false}
      busy={false}
      adapterReady
      onToggle={onToggle}
      onChoose={vi.fn()}
      onClearSet={vi.fn()}
      onStart={vi.fn()}
      onStop={vi.fn()}
      onSaveCsv={vi.fn()}
      saved={null}
    />,
  );
  return { ...view, onToggle };
}

describe("SDD's generic identifiers in the live read (ADR-0044)", () => {
  it("lists SDD's other values behind a fold, filters them by name, and ticks one into the set", () => {
    const { onToggle } = panel([]);
    // The module's own list holds its own only.
    expect(screen.getByText("0xDD01")).toBeInTheDocument();
    expect(screen.queryByText("0xF411")).toBeNull();

    // The fold says how many there are, and lists none until it is opened
    // or filtered: a thousand rows under every module would weigh.
    expect(screen.getByText("Other values SDD describes (3)")).toBeInTheDocument();
    expect(screen.queryByText("0x032F")).toBeNull();
    // Opening the fold lists them all, alphabetically by name.
    const fold = screen.getByText("Other values SDD describes (3)").closest("details")!;
    fold.open = true;
    fireEvent(fold, new Event("toggle"));
    expect(screen.getByText("0x032F")).toBeInTheDocument();
    const search = screen.getByLabelText("Filter by name or address");
    fireEvent.change(search, { target: { value: "throttle" } });
    expect(screen.getByText("0xF411")).toBeInTheDocument();
    expect(screen.getByText("0xF447")).toBeInTheDocument();
    expect(screen.queryByText("0x032F")).toBeNull();

    // Ticked as the module's own are ticked.
    const row = screen.getByText("0xF411").closest("tr");
    fireEvent.click(row!.querySelector("input")!);
    expect(onToggle).toHaveBeenCalledWith({ ecuFamily: "PCM", identifier: "0xF411" });
  });

  it("shows a chosen generic value with the module, marked as tied to no module in SDD", () => {
    panel([{ ecuFamily: "PCM", identifier: "0x032F" }]);
    const row = screen.getByText("0x032F").closest("tr");
    expect(row).not.toBeNull();
    expect(row?.textContent).toContain("no module in SDD");
    expect(row?.querySelector("input")?.checked).toBe(true);
    // Not claimed as this car's own: the dials and the "own" count stay honest.
    expect(isVehicleQualified({ qualification: "generic" })).toBe(false);
  });

  it("says when nothing matches, and never lists the module's own identifiers twice", () => {
    panel([]);
    const search = screen.getByLabelText("Filter by name or address");
    fireEvent.change(search, { target: { value: "zzzz" } });
    expect(screen.getByText("No value matches.")).toBeInTheDocument();
    fireEvent.change(search, { target: { value: "0xDD" } });
    expect(screen.getByText("No value matches.")).toBeInTheDocument();
  });
});
