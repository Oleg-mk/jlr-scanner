import { decimalText } from "./i18n";

/**
 * A reading's decimals are its converter's own: the decoder writes 21.0 and
 * 21.5 at a half-degree step, 2.500 V on a 5/1024 V channel. The smallest
 * and largest of a run are written with the same decimals, so the three
 * numbers of a row read as one instrument (2026-09-17).
 */
export function decimalsOf(reading: string | null): number {
  const match = /\.(\d+)$/.exec(reading ?? "");
  return match === null ? 0 : match[1].length;
}

/** A number written with the given decimals, in the language's notation. */
export function withDecimals(number: number, decimals: number): string {
  return decimalText(number.toFixed(decimals));
}
