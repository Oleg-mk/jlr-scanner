import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { VehicleCard } from "./components/VehicleCard";
import type { VehicleCatalogueSnapshot, VehicleDescription } from "./library";

/**
 * SDD splits some engines further — the naturally aspirated V8 of an X250 by
 * displacement — and puts each half at its own diagnostic address, so a car
 * that cannot say which half it is leaves those modules unresolved. The field
 * appears only for a programme whose loaded data splits something, so it does
 * not clutter the description of a car where nothing is split.
 */
const vehicle: VehicleDescription = {
  vehicleProgram: "",
  modelYear: null,
  powertrain: null,
  variant: null,
  market: null,
  yearBreakpoint: null,
};

const catalogue: VehicleCatalogueSnapshot = {
  programmes: [
    {
      program: "SPLIT",
      markers: [{ marker: "MY10", modelYearFrom: 2010, modelYearTo: 2013 }],
      powertrains: ["V8NA"],
      variants: ["4.2L", "5L"],
    },
    {
      program: "PLAIN",
      markers: [{ marker: "MY10", modelYearFrom: 2010, modelYearTo: 2013 }],
      powertrains: ["V6"],
      variants: [],
    },
  ],
};

function card(described: VehicleDescription, onVehicleChange = () => {}) {
  return render(
    <VehicleCard
      vehicle={described}
      catalogue={catalogue}
      survey={null}
      busy={false}
      libraryReady
      vin=""
      vinDecode={null}
      decodingVin={false}
      onVinChange={() => {}}
      onDecodeVin={() => {}}
      onVehicleChange={onVehicleChange}
      onSurvey={() => {}}
    />,
  );
}

describe("the engine variant", () => {
  it("is offered for a programme whose data splits an engine", async () => {
    card({ ...vehicle, vehicleProgram: "SPLIT" });
    const select = await screen.findByRole("combobox", { name: /Engine variant/i });
    expect([...select.querySelectorAll("option")].map((option) => option.textContent)).toEqual([
      "Not stated",
      "4.2L",
      "5L",
    ]);
  });

  it("is not offered where the data splits nothing", () => {
    card({ ...vehicle, vehicleProgram: "PLAIN" });
    expect(screen.queryByRole("combobox", { name: /Engine variant/i })).toBeNull();
  });

  it("reports the chosen variant, and clears it when the programme changes", async () => {
    const changes: VehicleDescription[] = [];
    const { rerender } = card({ ...vehicle, vehicleProgram: "SPLIT" }, (next) =>
      changes.push(next),
    );
    fireEvent.change(await screen.findByRole("combobox", { name: /Engine variant/i }), {
      target: { value: "5L" },
    });
    await waitFor(() => expect(changes).toHaveLength(1));
    expect(changes[0].variant).toBe("5L");

    rerender(
      <VehicleCard
        vehicle={{ ...vehicle, vehicleProgram: "SPLIT", variant: "5L" }}
        catalogue={catalogue}
        survey={null}
        busy={false}
        libraryReady
        vin=""
        vinDecode={null}
        decodingVin={false}
        onVinChange={() => {}}
        onDecodeVin={() => {}}
        onVehicleChange={(next) => changes.push(next)}
        onSurvey={() => {}}
      />,
    );
    fireEvent.change(screen.getByRole("combobox", { name: /Programme/i }), {
      target: { value: "PLAIN" },
    });
    await waitFor(() => expect(changes).toHaveLength(2));
    expect(changes[1].variant).toBeNull();
  });
});
