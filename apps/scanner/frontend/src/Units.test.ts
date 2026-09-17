import { afterEach, describe, expect, it } from "vitest";
import { setCurrentLanguage } from "./i18n";
import { unitLabel, withUnit } from "./units";

/**
 * The library carries SDD's unit codes; the screen shows symbols in the
 * interface language. `int` is SDD's word for "no unit" and shows nothing,
 * and a code the table does not know is shown as it is (2026-09-17).
 */
describe("units", () => {
  afterEach(() => setCurrentLanguage("en"));

  it("turns SDD's codes into symbols", () => {
    expect(unitLabel("degC")).toBe("°C");
    expect(unitLabel("pct")).toBe("%");
    expect(unitLabel("kph")).toBe("km/h");
    expect(unitLabel("uA")).toBe("µA");
    expect(unitLabel("R")).toBe("Ω");
    expect(unitLabel("m/s^2")).toBe("m/s²");
  });

  it("writes the symbol in the interface language", () => {
    setCurrentLanguage("uk");
    expect(withUnit("13,6", "V")).toBe("13,6 В");
    expect(withUnit("60", "kph")).toBe("60 км/год");
    expect(withUnit("812", "rpm")).toBe("812 об/хв");
    setCurrentLanguage("ru");
    expect(withUnit("60", "kph")).toBe("60 км/ч");
    expect(withUnit("812", "rpm")).toBe("812 об/мин");
    expect(withUnit("21", "degC")).toBe("21 °C");
  });

  it("shows nothing for int, for no unit, and a code it does not know as it is", () => {
    expect(unitLabel("int")).toBe("");
    expect(withUnit("7", "int")).toBe("7");
    expect(withUnit("7", null)).toBe("7");
    expect(unitLabel("furlong")).toBe("furlong");
    expect(withUnit("3", "furlong")).toBe("3 furlong");
  });
});
