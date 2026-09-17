import { afterEach, describe, expect, it } from "vitest";
import { setCurrentLanguage } from "./i18n";
import { decimalsOf, withDecimals } from "./liveFormat";

/**
 * The smallest and largest of a run are written with the reading's own
 * decimals, which are its converter's: 21.0 and 21.5 at a half-degree step,
 * 2.500 V on a 5/1024 V channel, whole numbers for a count (2026-09-17).
 */
describe("live formatting", () => {
  afterEach(() => setCurrentLanguage("en"));

  it("reads the decimals off the reading", () => {
    expect(decimalsOf("21.0")).toBe(1);
    expect(decimalsOf("2.500")).toBe(3);
    expect(decimalsOf("1726")).toBe(0);
    expect(decimalsOf(null)).toBe(0);
  });

  it("writes a number with the same decimals, in the language's notation", () => {
    expect(withDecimals(20.96, 1)).toBe("21.0");
    expect(withDecimals(2.5, 3)).toBe("2.500");
    expect(withDecimals(1726.4, 0)).toBe("1726");
    setCurrentLanguage("uk");
    expect(withDecimals(21.5, 1)).toBe("21,5");
    expect(withDecimals(812, 0)).toBe("812");
  });
});
