import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { PrintableReport } from "./components/PrintableReport";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage, t } from "./i18n";
import { buildReport, maskTail, type SessionBundle } from "./printableReport";

/**
 * The readable report (ADR-0031): the session as a document, every section
 * saying what its values are worth, no verdict anywhere, and the VIN and the
 * adapter's serial masked when the tester asks.
 */
function bundle(partial: Partial<SessionBundle> = {}): SessionBundle {
  return {
    application_version: "0.9.8",
    application_build: "a1b2c3d",
    session_mode: "real",
    saved_unix_ms: 1_788_480_000_000,
    adapter: {
      adapter: {
        name: "MongoosePro JLR",
        port: "COM3",
        serialNumber: "MJ1234567890",
        backend: "mongoose-jlr",
      },
    },
    library: {
      state: "LOADED",
      manifestsLoaded: 6515,
      sources: 6564,
      records: 355_112,
      issue: { issuedTo: "Oleg", issueCode: "D9C9-6B50", validUntil: "2026-10-12" },
    },
    survey: {
      context: { vehicleProgram: "X250", modelYear: 2010, yearBreakpoint: "MY10" },
      modules: [
        {
          ecuFamily: "PCM",
          logicalNetwork: "PT_HSCAN",
          protocol: "ISO14229",
          requestId: "0x7E0",
          identifierRead: { status: "REACHABLE", reasons: [] },
          dtcRead: { status: "REACHABLE", reasons: [] },
        },
        {
          ecuFamily: "SASM",
          logicalNetwork: "DS2",
          protocol: "DS2",
          requestId: "0x72",
          identifierRead: {
            status: "HYPOTHESIS",
            reasons: ["the adapter has not opened a K-line yet"],
          },
          dtcRead: { status: "HYPOTHESIS", reasons: [] },
        },
      ],
      reachable: 1,
      hypothesis: 1,
      unreachable: 0,
      message: "2 modules known: 1 reachable over the adapter, 1 on a hypothesised route, 0 not",
    },
    module_reads: [
      {
        ecu_family: "PCM",
        operation: "READ_DTC_INFORMATION",
        route_validation: "SOURCE_BACKED",
        decoded_result: "P0300, P0301",
        actual_responder: "0x7E8",
        timestamp_unix_ms: 1_788_479_000_000,
      },
    ],
    battery_reads: [
      {
        route_validation: "SOURCE_BACKED",
        readings: [
          {
            ecuFamily: "BCM",
            identifier: "0x4028",
            parameter: "Vehicle Battery Estimated State of Charge",
            value: "78",
            unit: "pct",
          },
        ],
      },
    ],
    ...partial,
  };
}

afterEach(() => {
  window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  setCurrentLanguage("en");
});

describe("the readable report", () => {
  it("shows the car, the library, the modules and every read, each with what it is worth", () => {
    render(<PrintableReport report={buildReport(bundle(), t, false)} />);

    expect(screen.getByRole("heading", { name: "Diagnostic session report" })).toBeVisible();
    expect(screen.getByText(/ProwlOne 0\.9\.8/)).toBeVisible();
    // The car and the library it was read with.
    expect(screen.getByText("X250")).toBeVisible();
    expect(screen.getByText("D9C9-6B50")).toBeVisible();
    // The modules, with the reason beside the one that is a hypothesis.
    expect(screen.getByText(/the adapter has not opened a K-line yet/)).toBeVisible();
    // The reads.
    expect(screen.getByText("P0300, P0301")).toBeVisible();
    expect(screen.getByText("78 pct")).toBeVisible();
    // What the values are worth, said in the document and not only on screen.
    expect(screen.getAllByText("SOURCE_BACKED").length).toBeGreaterThan(0);
    expect(
      screen.getByText(/nothing in it is vehicle-confirmed by being here/),
    ).toBeVisible();
  });

  it("says a bench session is a bench session, on every table and across the head", () => {
    const report = buildReport(
      bundle({ session_mode: "bench", bench_scenario: 3 }),
      t,
      false,
    );
    render(<PrintableReport report={report} />);
    expect(screen.getByText(/BENCH SESSION/)).toBeVisible();
    expect(screen.getByText(/Scenario 3/)).toBeVisible();
    for (const worth of screen.getAllByText("SYNTHETIC")) {
      expect(worth).toBeVisible();
    }
    expect(screen.queryByText("SOURCE_BACKED")).toBeNull();
  });

  it("masks the adapter's serial when the tester asks, and leaves the bundle alone", () => {
    const source = bundle();
    const masked = buildReport(source, t, true);
    render(<PrintableReport report={masked} />);
    expect(screen.getByText(/•+7890/)).toBeVisible();
    expect(screen.queryByText("MJ1234567890")).toBeNull();
    // The bundle itself is untouched: masking is a property of the document.
    expect(source.adapter?.adapter?.serialNumber).toBe("MJ1234567890");
    expect(maskTail("SALVA2BG9FH123456")).toBe("•••••••••••••3456");
    expect(maskTail("ABC")).toBe("ABC");
  });

  it("draws no conclusion of its own", () => {
    render(<PrintableReport report={buildReport(bundle(), t, false)} />);
    const document = screen.getByRole("article");
    expect(document.textContent ?? "").not.toMatch(
      /\b(healthy|faulty|good condition|needs replacing|recommend|urgent|critical)\b/i,
    );
  });

  it("says so when the session recorded nothing", () => {
    const empty = buildReport(
      { application_version: "0.9.8", session_mode: "real" },
      t,
      false,
    );
    expect(empty.empty).toBe(true);
    render(<PrintableReport report={empty} />);
    expect(screen.getByText("This session recorded nothing yet.")).toBeVisible();
  });

  it("words the document in the interface's language", () => {
    setCurrentLanguage("uk");
    render(<PrintableReport report={buildReport(bundle(), t, false)} />);
    expect(screen.getByRole("heading", { name: "Звіт діагностичного сеансу" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Автомобіль" })).toBeVisible();
  });
});
