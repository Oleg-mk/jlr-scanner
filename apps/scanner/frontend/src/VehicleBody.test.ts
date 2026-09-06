import { describe, expect, it } from "vitest";
import { bodyFor, bodyImageUrl, vehicleImageUrl } from "./vehicleBody";

describe("the body a car is drawn as", () => {
  it("comes from the programme", () => {
    expect(bodyFor("X250")).toBe("sedan");
    expect(bodyFor("l319")).toBe("large-suv");
    expect(bodyFor("L538")).toBe("compact-suv");
    expect(bodyFor("L316")).toBe("defender");
    expect(bodyFor("X152")).toBe("coupe");
  });

  it("is refined by what the VIN says about the body", () => {
    expect(bodyFor("X250", [{ name: "Body style", value: "Sportbrake" }])).toBe("wagon");
    expect(bodyFor("X152", [{ name: "Body", value: "Convertible" }])).toBe("cabrio");
    expect(bodyFor("X150", [{ name: "Body", value: "Coupé" }])).toBe("coupe");
  });

  it("is unknown for a programme the map does not name", () => {
    expect(bodyFor("ZZZ999")).toBeNull();
    expect(bodyFor("")).toBeNull();
  });

  it("names the image file by the body", () => {
    expect(bodyImageUrl("large-suv")).toBe("/vehicles/large-suv.png");
  });
});

describe("the owner's picture of a model", () => {
  it("is chosen by programme, and by model year where the model changed face", () => {
    expect(vehicleImageUrl("X250", 2010, "sedan")).toBe("/vehicles/X250.webp");
    expect(vehicleImageUrl("X250", 2013, "sedan")).toBe("/vehicles/X250-from2012.webp");
    expect(vehicleImageUrl("x250", null, null)).toBe("/vehicles/X250.webp");
    expect(vehicleImageUrl("L319", 2006, "large-suv")).toBe("/vehicles/L319.webp");
    expect(vehicleImageUrl("L319", 2012, "large-suv")).toBe("/vehicles/L319-from2010.webp");
    expect(vehicleImageUrl("X204", 2006, "sedan")).toBe("/vehicles/X202.webp");
  });

  it("follows the body where the model came as more than one", () => {
    expect(vehicleImageUrl("X152", 2015, "coupe")).toBe("/vehicles/X152-coupe.webp");
    expect(vehicleImageUrl("X152", 2015, "cabrio")).toBe("/vehicles/X152-cabrio.webp");
    expect(vehicleImageUrl("X260", 2018, "wagon")).toBe("/vehicles/X260-wagon.webp");
    expect(vehicleImageUrl("X260", 2022, "sedan")).toBe("/vehicles/X260-from2021.webp");
  });

  it("is null for a programme without a picture", () => {
    expect(vehicleImageUrl("L550", 2016, "compact-suv")).toBe("/vehicles/L550.webp");
    expect(vehicleImageUrl("X404", 2005, "wagon")).toBe("/vehicles/X400.webp");
    expect(vehicleImageUrl("X103", 2003, "cabrio")).toBeNull();
    expect(vehicleImageUrl("", null, null)).toBeNull();
  });
});
