import { act, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { StandardObdPanel } from "./components/StandardObdPanel";
import { LANGUAGE_STORAGE_KEY, setCurrentLanguage } from "./i18n";
import { createVehicleDescription } from "./library";
import {
  createStandardObdSnapshot,
  type StandardObdClient,
  type StandardObdRequest,
  type StandardObdSnapshot,
  type StandardObdValue,
} from "./standardObd";
import { useStandardObdController } from "./useStandardObdController";

/**
 * The legislated services (ADR-0022, decision 7): the panel shows a decoded
 * answer as the standard words it, and the controller's walks read every
 * supported item — PIDs six to a request, as ISO 15765-4 allows, freeze-frame
 * PIDs and InfoTypes one at a time.
 */
class ScriptedClient implements StandardObdClient {
  requests: StandardObdRequest[] = [];

  getState() {
    return Promise.resolve(createStandardObdSnapshot());
  }

  read(request: StandardObdRequest): Promise<StandardObdSnapshot> {
    this.requests.push(request);
    const answer: StandardObdSnapshot = {
      ...createStandardObdSnapshot(),
      state: "SUCCEEDED",
      kind: request.kind,
      responder: "0x7E0 → 0x7E8",
      operation: "scripted",
      routeValidation: "SOURCE_BACKED",
      reportAvailable: true,
    };
    const [first] = request.items;
    const value = (pid: string): StandardObdValue => ({
      pid,
      name: `parameter ${pid}`,
      unit: "",
      value: "1",
      number: 1,
      rawHex: "01",
      kind: "number",
    });
    switch (request.kind) {
      case "CURRENT_DATA":
        if (first === "0x00") {
          // Seven PIDs in the first map, and the next map exists.
          answer.supported = ["0x04", "0x05", "0x0C", "0x0D", "0x0F", "0x11", "0x1F", "0x20"];
        } else if (first === "0x20") {
          answer.supported = ["0x21", "0x2F"];
        } else {
          answer.values = request.items.map(value);
        }
        break;
      case "FREEZE_FRAME":
        answer.freezeFrame = 0;
        if (first === "0x00") {
          answer.supported = ["0x02", "0x04", "0x0C", "0x0D"];
        } else if (first === "0x02") {
          answer.values = [
            { ...value("0x02"), name: "fault code that froze the frame", value: "P0300", number: null, kind: "text" },
          ];
        } else if (first === "0x0D") {
          // A PID the frame does not carry after all: refused, not fatal.
          answer.negativeResponse = "0x31 request out of range";
        } else {
          answer.values = [value(first)];
        }
        break;
      case "MONITOR_RESULTS":
        if (first === "0x00") {
          answer.supported = ["0x01", "0x21"];
        } else {
          answer.monitors = request.items.map((mid) => ({
            mid,
            monitor: `monitor ${mid}`,
            tid: "0x80",
            uas: "0x0B",
            unit: "V",
            value: 0.5,
            minimum: 0,
            maximum: 1,
            rawValue: 500,
            rawMinimum: 0,
            rawMaximum: 1000,
            passed: true,
          }));
        }
        break;
      case "VEHICLE_INFORMATION":
        if (first === "0x00") {
          // Message counts (0x01, 0x03) are K-line only; the walk must skip them.
          answer.supported = ["0x01", "0x02", "0x03", "0x04", "0x0A"];
        } else if (first === "0x02") {
          answer.information = [{ label: "VIN", value: "SAJTEST0000000001" }];
        } else if (first === "0x04") {
          answer.information = [{ label: "Calibration ID", value: "AX23-14C204-AC" }];
        } else if (first === "0x0A") {
          answer.information = [{ label: "ECU name", value: "ECM -EngineControl" }];
        }
        break;
      default:
        break;
    }
    return Promise.resolve(answer);
  }
}

function controller(client: StandardObdClient) {
  return renderHook(() => useStandardObdController(client, createVehicleDescription()));
}

const idleHandlers = {
  onResponderChange: () => {},
  onRead: () => {},
  onReadEverything: () => {},
  onReadFreezeFrame: () => {},
  onReadMonitors: () => {},
  onReadVehicleInformation: () => {},
  onClear: () => {},
};

describe("standard OBD-II", () => {
  afterEach(() => {
    setCurrentLanguage("en");
    window.localStorage.removeItem(LANGUAGE_STORAGE_KEY);
  });

  it("reads every supported PID six to a request after walking the support maps", async () => {
    const client = new ScriptedClient();
    const { result } = controller(client);
    await act(async () => {
      await result.current.readEverything();
    });
    const items = client.requests.map((request) => request.items);
    expect(items[0]).toEqual(["0x00"]);
    expect(items[1]).toEqual(["0x20"]);
    // Nine supported PIDs: one request of six, one of three.
    expect(items[2]).toEqual(["0x04", "0x05", "0x0C", "0x0D", "0x0F", "0x11"]);
    expect(items[3]).toEqual(["0x1F", "0x21", "0x2F"]);
    expect(client.requests).toHaveLength(4);
    expect(result.current.values.map((value) => value.pid)).toEqual([
      "0x04", "0x05", "0x0C", "0x0D", "0x0F", "0x11", "0x1F", "0x21", "0x2F",
    ]);
    expect(client.requests[0].responder).toBe(0);
  });

  it("reads the freeze frame PID by PID, the code that froze it first, skipping a refused PID", async () => {
    const client = new ScriptedClient();
    const { result } = controller(client);
    await act(async () => {
      await result.current.readFreezeFrame();
    });
    const items = client.requests.map((request) => request.items);
    expect(items).toEqual([["0x00"], ["0x02"], ["0x04"], ["0x0C"], ["0x0D"]]);
    expect(client.requests.every((request) => request.kind === "FREEZE_FRAME")).toBe(true);
    expect(result.current.frameValues.map((value) => [value.pid, value.value])).toEqual([
      ["0x02", "P0300"],
      ["0x04", "1"],
      ["0x0C", "1"],
    ]);
    // The refusal of 0x0D is the last snapshot, not a failure of the frame.
    expect(result.current.snapshot.negativeResponse).toBe("0x31 request out of range");
    expect(result.current.snapshot.state).toBe("SUCCEEDED");
  });

  it("reads the monitors six to a request and the read-only vehicle information one type at a time", async () => {
    const client = new ScriptedClient();
    const { result } = controller(client);
    await act(async () => {
      await result.current.readMonitors();
    });
    expect(client.requests.map((request) => request.items)).toEqual([["0x00"], ["0x01", "0x21"]]);
    expect(result.current.monitors.map((monitor) => monitor.mid)).toEqual(["0x01", "0x21"]);

    client.requests = [];
    await act(async () => {
      await result.current.readVehicleInformation();
    });
    expect(client.requests.map((request) => request.items)).toEqual([["0x00"], ["0x02"], ["0x04"], ["0x0A"]]);
    expect(result.current.information).toEqual([
      { label: "VIN", value: "SAJTEST0000000001" },
      { label: "Calibration ID", value: "AX23-14C204-AC" },
      { label: "ECU name", value: "ECM -EngineControl" },
    ]);

    act(() => result.current.clear());
    expect(result.current.monitors).toEqual([]);
    expect(result.current.information).toEqual([]);
  });

  it("shows the values, the codes with our wording, and marks the bench", async () => {
    const snapshot: StandardObdSnapshot = {
      ...createStandardObdSnapshot(),
      state: "SUCCEEDED",
      kind: "STORED_DTCS",
      responder: "0x7E0 → 0x7E8",
      operation: "Stored fault codes",
      routeValidation: "SYNTHETIC",
      dtcKind: "stored",
      dtcs: [
        {
          code: "P0300",
          failureType: "",
          status: "",
          description: "Random / multiple cylinder misfire detected",
          descriptionScope: "generic",
          failureTypeText: null,
          failureTypeTexts: {},
          descriptionDataTexts: {},
          descriptionTexts: { ukr: "Пропуски запалювання в кількох циліндрах" },
          help: ["Possible causes:", "Ignition, fuel or compression"],
          helpTexts: {},
          helpNote: null,
        },
      ],
      rawResponseHex: "43 01 03 00",
      requestHex: "03",
      reportAvailable: true,
    };
    render(
      <StandardObdPanel
        snapshot={snapshot}
        values={[
          { pid: "0x0C", name: "engine speed", unit: "rpm", value: "750", number: 750, rawHex: "0B B8", kind: "number" },
          { pid: "0x6D", name: "fuel pressure control system", unit: "", value: null, number: null, rawHex: "00 01", kind: "raw" },
        ]}
        frameValues={[
          { pid: "0x02", name: "fault code that froze the frame", unit: "", value: "P0300", number: null, rawHex: "03 00", kind: "text" },
          { pid: "0x05", name: "engine coolant temperature", unit: "°C", value: "83", number: 83, rawHex: "7B", kind: "number" },
        ]}
        monitors={[]}
        information={[{ label: "VIN", value: "SAJTEST0000000001" }]}
        busy={false}
        adapterReady
        responder={0}
        {...idleHandlers}
        bench
      />,
    );
    await waitFor(() => expect(screen.getByText("SYNTHETIC")).toBeInTheDocument());
    expect(screen.getByText("engine speed")).toBeInTheDocument();
    expect(screen.getByText("750")).toBeInTheDocument();
    expect(screen.getByText("rpm")).toBeInTheDocument();
    // A PID whose layout is not carried shows its bytes and says so.
    expect(screen.getByText("00 01")).toBeInTheDocument();
    expect(screen.getByText("raw: the layout is not carried")).toBeInTheDocument();
    // The frame, headed by the code that froze it.
    expect(screen.getByRole("heading", { name: "Freeze frame 0 — at code P0300" })).toBeInTheDocument();
    expect(screen.getByText("engine coolant temperature")).toBeInTheDocument();
    expect(screen.getByText("SAJTEST0000000001")).toBeInTheDocument();
    // The code twice — in the list, and as the frame's own value — with the standard's wording underneath.
    expect(screen.getAllByText("P0300")).toHaveLength(2);
    expect(screen.getByText("Random / multiple cylinder misfire detected")).toBeInTheDocument();
    // What the loaded data says about the code, folded away under it.
    expect(screen.getByText("What the data says about this code")).toBeInTheDocument();
    expect(screen.getByText("Ignition, fuel or compression")).toBeInTheDocument();
    expect(screen.getByText("43 01 03 00")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Clear the tables" })).toBeEnabled();
  });

  it("words the standard's parameters and refusals in the interface language", () => {
    setCurrentLanguage("uk");
    render(
      <StandardObdPanel
        snapshot={{
          ...createStandardObdSnapshot(),
          state: "SUCCEEDED",
          kind: "CURRENT_DATA",
          responder: "0x7E0 → 0x7E8",
          negativeResponse: "0x31 request out of range",
          rawResponseHex: "7F 01 31",
          requestHex: "01 C0",
          reportAvailable: true,
        }}
        values={[
          { pid: "0x0C", name: "engine speed", unit: "rpm", value: "750", number: 750, rawHex: "0B B8", kind: "number" },
          { pid: "0x03", name: "fuel system 1", unit: "", value: "closed loop, oxygen sensor feedback", number: null, rawHex: "02 00", kind: "text" },
        ]}
        frameValues={[
          { pid: "0x02", name: "fault code that froze the frame", unit: "", value: "none", number: null, rawHex: "00 00", kind: "text" },
        ]}
        monitors={[]}
        information={[]}
        busy={false}
        adapterReady
        responder={0}
        {...idleHandlers}
      />,
    );
    expect(screen.getByText("оберти двигуна")).toBeInTheDocument();
    expect(screen.getByText("замкнений контур, за кисневим датчиком")).toBeInTheDocument();
    expect(screen.getByText("жоден код не зберіг стоп-кадр")).toBeInTheDocument();
    // The refusal: the code as bytes, the standard's reason in our words.
    expect(screen.getByText("0x31")).toBeInTheDocument();
    expect(screen.getByText(/запит поза діапазоном/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Стоп-кадр 0" })).toBeInTheDocument();
  });

  it("says when a list is empty and when the adapter is not there", () => {
    render(
      <StandardObdPanel
        snapshot={{
          ...createStandardObdSnapshot(),
          state: "SUCCEEDED",
          kind: "PENDING_DTCS",
          responder: "0x7E0 → 0x7E8",
          operation: "Pending fault codes",
          dtcKind: "pending",
          negativeResponse: null,
          rawResponseHex: "47 00",
          reportAvailable: true,
        }}
        values={[]}
        frameValues={[]}
        monitors={[]}
        information={[]}
        busy={false}
        adapterReady={false}
        responder={0}
        {...idleHandlers}
      />,
    );
    expect(screen.getByText("No fault codes of this kind.")).toBeInTheDocument();
    expect(screen.getByText("Connect and verify the adapter, or the bench, first.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Read everything supported" })).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Clear the tables" })).not.toBeInTheDocument();
  });
});
