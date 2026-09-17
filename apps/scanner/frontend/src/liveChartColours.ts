/**
 * The colours of the live chart's lines, shared with the table that is its
 * legend. Ten a person tells apart on the plate; the eleventh repeats.
 */
export const CHART_COLOURS = [
  "#0b63ce",
  "#c8362f",
  "#1d7a38",
  "#a66b00",
  "#6d3fb3",
  "#0e8a8a",
  "#b34a8f",
  "#5c6b1f",
  "#8a4b0e",
  "#3a4d6b",
];

export function chartColour(index: number): string {
  return CHART_COLOURS[index % CHART_COLOURS.length];
}
