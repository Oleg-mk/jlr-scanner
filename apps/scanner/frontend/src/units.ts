import { currentLanguage, type Language } from "./i18n";

/**
 * The unit a reading is shown with. The library carries SDD's own unit
 * codes — `pct`, `degC`, `kph`, `int` — which are names in a data file, not
 * symbols a person reads. Here each code becomes its symbol in the
 * interface language, and `int`, which SDD writes where a number carries no
 * unit at all, becomes nothing. A code this table does not know is shown as
 * it is: an unknown unit is still a unit, not a guess.
 *
 * The codes are the twenty-nine the library carries (2026-09-17) plus the
 * three the battery card already read.
 */
const SYMBOLS: Record<string, Record<Language, string>> = {
  int: { en: "", uk: "", ru: "" },
  V: { en: "V", uk: "В", ru: "В" },
  A: { en: "A", uk: "А", ru: "А" },
  mA: { en: "mA", uk: "мА", ru: "мА" },
  uA: { en: "µA", uk: "мкА", ru: "мкА" },
  Ah: { en: "A·h", uk: "А·год", ru: "А·ч" },
  pct: { en: "%", uk: "%", ru: "%" },
  degC: { en: "°C", uk: "°C", ru: "°C" },
  degF: { en: "°F", uk: "°F", ru: "°F" },
  km: { en: "km", uk: "км", ru: "км" },
  m: { en: "m", uk: "м", ru: "м" },
  kph: { en: "km/h", uk: "км/год", ru: "км/ч" },
  rpm: { en: "rpm", uk: "об/хв", ru: "об/мин" },
  s: { en: "s", uk: "с", ru: "с" },
  Nm: { en: "N·m", uk: "Н·м", ru: "Н·м" },
  deg: { en: "°", uk: "°", ru: "°" },
  "deg/s": { en: "°/s", uk: "°/с", ru: "°/с" },
  "kg/h": { en: "kg/h", uk: "кг/год", ru: "кг/ч" },
  "g/s": { en: "g/s", uk: "г/с", ru: "г/с" },
  Pa: { en: "Pa", uk: "Па", ru: "Па" },
  bar: { en: "bar", uk: "бар", ru: "бар" },
  Hz: { en: "Hz", uk: "Гц", ru: "Гц" },
  // Resistance: the library's "R" stands on damper-solenoid and sensor-element resistances.
  R: { en: "Ω", uk: "Ом", ru: "Ом" },
  "m/s^2": { en: "m/s²", uk: "м/с²", ru: "м/с²" },
  W: { en: "W", uk: "Вт", ru: "Вт" },
  g: { en: "g", uk: "г", ru: "г" },
  dB: { en: "dB", uk: "дБ", ru: "дБ" },
  "l/h": { en: "l/h", uk: "л/год", ru: "л/ч" },
  "mm/s": { en: "mm/s", uk: "мм/с", ru: "мм/с" },
  "cm^3": { en: "cm³", uk: "см³", ru: "см³" },
  L: { en: "l", uk: "л", ru: "л" },
  "kg/m^3": { en: "kg/m³", uk: "кг/м³", ru: "кг/м³" },
};

/** The symbol for a unit code in the interface language; nothing for none, or for `int`. */
export function unitLabel(unit: string | null | undefined): string {
  if (unit === null || unit === undefined || unit === "") return "";
  const symbol = SYMBOLS[unit];
  return symbol === undefined ? unit : symbol[currentLanguage()];
}

/** A value with its unit beside it, or the value alone where there is none to show. */
export function withUnit(value: string, unit: string | null | undefined): string {
  const label = unitLabel(unit);
  return label === "" ? value : `${value} ${label}`;
}
