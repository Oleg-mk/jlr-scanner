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
    {
      // A Discovery 4 as its data describes it: the diesels split by
      // displacement, the V8 not split at all.
      program: "PAIRED",
      markers: [{ marker: "MY10", modelYearFrom: 2010, modelYearTo: 2013 }],
      powertrains: ["V6DIESEL", "V8NA"],
      variants: ["2.7L", "3L"],
      variantsByPowertrain: [
        { powertrain: "V6DIESEL", variants: ["2.7L", "3L"] },
        { powertrain: "V8NA", variants: [] },
      ],
    },
  ],
};

function card(
  described: VehicleDescription,
  onVehicleChange: (next: VehicleDescription) => void = () => {},
) {
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

  it("offers only the variants of the chosen engine, and none under an engine that is not split", async () => {
    const { rerender } = card({ ...vehicle, vehicleProgram: "PAIRED" });
    // No engine chosen yet: everything the programme splits.
    let select = await screen.findByRole("combobox", { name: /Engine variant/i });
    expect([...select.querySelectorAll("option")].map((option) => option.textContent)).toEqual([
      "Not stated",
      "2.7L",
      "3L",
    ]);
    rerender(
      <VehicleCard
        vehicle={{ ...vehicle, vehicleProgram: "PAIRED", powertrain: "V6DIESEL" }}
        catalogue={catalogue}
        survey={null}
        busy={false}
        libraryReady
        vin=""
        vinDecode={null}
        decodingVin={false}
        onVinChange={() => {}}
        onDecodeVin={() => {}}
        onVehicleChange={() => {}}
        onSurvey={() => {}}
      />,
    );
    select = screen.getByRole("combobox", { name: /Engine variant/i });
    expect([...select.querySelectorAll("option")].map((option) => option.textContent)).toEqual([
      "Not stated",
      "2.7L",
      "3L",
    ]);
    rerender(
      <VehicleCard
        vehicle={{ ...vehicle, vehicleProgram: "PAIRED", powertrain: "V8NA" }}
        catalogue={catalogue}
        survey={null}
        busy={false}
        libraryReady
        vin=""
        vinDecode={null}
        decodingVin={false}
        onVinChange={() => {}}
        onDecodeVin={() => {}}
        onVehicleChange={() => {}}
        onSurvey={() => {}}
      />,
    );
    expect(screen.queryByRole("combobox", { name: /Engine variant/i })).toBeNull();
  });

  it("drops the variant when the engine changes", async () => {
    const changes: VehicleDescription[] = [];
    card({ ...vehicle, vehicleProgram: "PAIRED", powertrain: "V6DIESEL", variant: "3L" }, (next) =>
      changes.push(next),
    );
    fireEvent.change(await screen.findByRole("combobox", { name: "Engine" }), {
      target: { value: "V8NA" },
    });
    await waitFor(() => expect(changes).toHaveLength(1));
    expect(changes[0].powertrain).toBe("V8NA");
    expect(changes[0].variant).toBeNull();
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
