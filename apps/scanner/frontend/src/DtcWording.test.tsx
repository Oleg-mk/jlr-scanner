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
 * A help screen (ADR-0026) follows the same rule as a code's wording, with
 * one addition: the screen is read whole or not at all. A line this project
 * has no words for arrives as its English inside the translated list, so the
 * lists stay the same length; if they ever did not, the English screen is
 * shown rather than a screen half in each language.
 */
describe("help screen wording by language", () => {
  const english = ["Possible causes", "Injector failure.", "Actions required:"];
  const texts = {
    ukr: ["Можливі причини", "Несправність форсунки.", "Рекомендовані дії:"],
    rus: ["Возможные причины", "Неисправность форсунки.", "Рекомендуемые действия:"],
  };

  it("reads the screen in the interface's language", () => {
    expect(helpLines(texts, english, "uk")).toEqual(texts.ukr);
    expect(helpLines(texts, english, "ru")).toEqual(texts.rus);
  });

  it("reads it in English when the interface is English, or when we have none", () => {
    expect(helpLines(texts, english, "en")).toEqual(english);
    expect(helpLines({}, english, "uk")).toEqual(english);
    expect(helpLines(undefined, english, "uk")).toEqual(english);
  });

  it("refuses a screen that does not line up", () => {
    expect(helpLines({ ukr: ["Можливі причини"] }, english, "uk")).toEqual(english);
  });

  it("keeps an untranslated line in place rather than dropping it", () => {
    const mixed = ["Можливі причини", "Injector failure.", "Рекомендовані дії:"];
    expect(helpLines({ ukr: mixed }, english, "uk")).toEqual(mixed);
  });
});
