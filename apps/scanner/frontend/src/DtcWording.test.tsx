import { describe, expect, it } from "vitest";
import { codeText, productLanguage } from "./i18n";

/**
 * A fault code's wording follows the interface language when we have our own
 * text for that code, and falls back to the loaded data's English when we do
 * not. The English is never lost: when a translation is shown, the original
 * comes back beside it, and it is the only thing a report carries.
 */
describe("fault-code wording by language", () => {
  const ours = { ukr: "Надто бідна суміш (ряд 1)", rus: "Слишком бедная смесь (ряд 1)" };
  const english = "System too lean (bank 1)";

  it("knows Ukrainian, which SDD's own text does not", () => {
    expect(productLanguage("uk")).toBe("ukr");
    expect(productLanguage("ru")).toBe("rus");
    expect(productLanguage("en")).toBeNull();
  });

  it("shows our wording in Ukrainian and keeps the English beside it", () => {
    expect(codeText(ours, english, "uk")).toEqual({ shown: ours.ukr, original: english });
    expect(codeText(ours, english, "ru")).toEqual({ shown: ours.rus, original: english });
  });

  it("shows the loaded data's English in English, with nothing repeated", () => {
    expect(codeText(ours, english, "en")).toEqual({ shown: english, original: null });
  });

  it("falls back to English for a code we have not translated", () => {
    expect(codeText({}, english, "uk")).toEqual({ shown: english, original: null });
    expect(codeText(undefined, english, "uk")).toEqual({ shown: english, original: null });
  });

  it("says nothing rather than inventing when the library has no wording either", () => {
    expect(codeText({}, null, "uk")).toEqual({ shown: null, original: null });
  });
});
