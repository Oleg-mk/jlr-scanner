import { describe, expect, it } from "vitest";
import { bodyFor, bodyImageUrl } from "./vehicleBody";

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
