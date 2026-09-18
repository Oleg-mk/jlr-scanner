import { afterEach, describe, expect, it } from "vitest";
import { languages, setCurrentLanguage, t, type Language } from "./i18n";

/**
 * The header keeps one geometry in every language (2026-09-18).
 *
 * Measured on the owner's window: switching English to Russian moved the
 * left edge of the header row by 205 points, because each capsule was as
 * wide as its own text and every language writes a different length. The
 * capsules now reserve the width of their longest language, so the same
 * row is drawn whatever the language — but only while every label still
 * fits inside what was reserved. The first label to outgrow it starts the
 * movement again, silently.
 *
 * A unit test cannot measure a rendered capsule, so it counts characters
 * against the budget the reservation was measured from:
 *
 *   badges  213 points reserved, longest measured "Библиотека: встроенная"
 *           at 207 points and 22 characters;
 *   buttons 173 points reserved, longest measured "Сервисный режим"
 *           at 166 points and 15 characters.
 *
 * The limits below are those counts with one character of room. A label
 * that needs more is not wrong — it means the reservation in `styles.css`
 * has to be measured and widened in the same change.
 */

const BADGE_LIMIT = 23;
const BUTTON_LIMIT = 16;

/** Every state the adapter and library badges can show. */
const BADGES = [
  "Adapter ready",
  "Board unverified",
  "Adapter found",
  "Connecting…",
  "Adapter error",
  "No adapter",
  "Bench",
  "Library",
  "Library: built-in",
  "Library: failed",
];

/** The buttons that sit on the same row. */
const BUTTONS = ["New session", "Service mode"];

function measure(keys: string[], limit: number) {
  const over: string[] = [];
  for (const { id } of languages) {
    setCurrentLanguage(id as Language);
    for (const key of keys) {
      const text = t(key);
      if (text.length > limit) over.push(`${id}: "${text}" (${text.length})`);
    }
  }
  return over;
}

describe("the header keeps one width in every language", () => {
  afterEach(() => {
    setCurrentLanguage("en");
  });

  it("has no badge longer than the width reserved for it", () => {
    expect(measure(BADGES, BADGE_LIMIT)).toEqual([]);
  });

  it("has no button longer than the width reserved for it", () => {
    expect(measure(BUTTONS, BUTTON_LIMIT)).toEqual([]);
  });

  it("translates every one of them, so none falls back to English", () => {
    const untranslated: string[] = [];
    for (const { id } of languages) {
      if (id === "en") continue;
      setCurrentLanguage(id as Language);
      for (const key of [...BADGES, ...BUTTONS]) {
        if (t(key) === key) untranslated.push(`${id}: ${key}`);
      }
    }
    expect(untranslated).toEqual([]);
  });
});
