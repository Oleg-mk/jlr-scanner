import { describe, expect, it } from "vitest";
import { codeText, helpLines, productLanguage } from "./i18n";

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

/**
 * A help screen is SDD's own text (ADR-0034), read in the language chosen
 * for such text: Russian when the library carries it, English otherwise.
 * The screen is read whole or not at all — a line the Russian pack lacks
 * arrives as its English inside the Russian list, so the lists stay the same
 * length; if they ever did not, the English screen is shown rather than a
 * screen half in each language.
 */
describe("help screen text by chosen language", () => {
  const english = ["Possible causes:", "Injector failure.", "Actions required:"];
  const texts = {
    rus: ["Возможные причины:", "Неисправность форсунки.", "Необходимые действия:"],
  };

  it("reads the Russian when it is chosen and the library has it", () => {
    expect(helpLines(texts, english, "rus")).toEqual(texts.rus);
  });

  it("reads the English when English is chosen, or when the library has no Russian", () => {
    expect(helpLines(texts, english, "eng")).toEqual(english);
    expect(helpLines({}, english, "rus")).toEqual(english);
    expect(helpLines(undefined, english, "rus")).toEqual(english);
  });

  it("refuses a screen that does not line up", () => {
    expect(helpLines({ rus: ["Возможные причины:"] }, english, "rus")).toEqual(english);
  });

  it("keeps a line the Russian pack lacks in place, in English", () => {
    const mixed = ["Возможные причины:", "Injector failure.", "Необходимые действия:"];
    expect(helpLines({ rus: mixed }, english, "rus")).toEqual(mixed);
  });
});
