import type { LiveReadValue } from "./liveRead";

/**
 * Warning and alarm limits for a live parameter, set by the person at the
 * bench, kept on this machine only.
 *
 * SDD records no normal range for anything, and this product will not invent
 * one: the limits here are the operator's, the way every workshop tool from
 * FORScan to Autel does it. A tile changes colour when a reading crosses one.
 * One starter suggestion ships, marked as ours, for the one number the owner
 * named himself (2026-09-16): a 12-volt system does not sit above 15 V.
 */
export interface LiveLimits {
  warnLow: number | null;
  warnHigh: number | null;
  alarmLow: number | null;
  alarmHigh: number | null;
}

export type LimitTone = "none" | "ok" | "warn" | "alarm";

const STORAGE_KEY = "prowlone.liveLimits";

export const EMPTY_LIMITS: LiveLimits = {
  warnLow: null,
  warnHigh: null,
  alarmLow: null,
  alarmHigh: null,
};

/** The limits belong to a parameter of a module, not to a name alone. */
export function limitKey(value: Pick<LiveReadValue, "ecuFamily" | "identifier" | "name">): string {
  return `${value.ecuFamily}|${value.identifier}|${value.name}`;
}

export function readLimits(): Record<string, LiveLimits> {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (raw === null) return {};
    const parsed: unknown = JSON.parse(raw);
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, LiveLimits>)
      : {};
  } catch {
    // A browser that refuses storage is not a reason to lose the run.
    return {};
  }
}

export function writeLimits(limits: Record<string, LiveLimits>) {
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(limits));
  } catch {
    // Nothing to do: the limits live for this session only.
  }
}

/** Whether any limit is set at all. */
export function hasLimits(limits: LiveLimits | undefined): boolean {
  return (
    limits !== undefined &&
    (limits.warnLow !== null ||
      limits.warnHigh !== null ||
      limits.alarmLow !== null ||
      limits.alarmHigh !== null)
  );
}

/**
 * How a reading stands against the limits: an alarm limit crossed is
 * `alarm`, a warning limit crossed is `warn`, inside them `ok`, and `none`
 * where nothing is set or there is no number to judge.
 */
export function toneFor(number: number | null, limits: LiveLimits | undefined): LimitTone {
  if (number === null || !hasLimits(limits) || limits === undefined) return "none";
  if (
    (limits.alarmLow !== null && number < limits.alarmLow) ||
    (limits.alarmHigh !== null && number > limits.alarmHigh)
  ) {
    return "alarm";
  }
  if (
    (limits.warnLow !== null && number < limits.warnLow) ||
    (limits.warnHigh !== null && number > limits.warnHigh)
  ) {
    return "warn";
  }
  return "ok";
}

/**
 * Suggested starting limits — ours, not JLR's — offered until the person
 * sets their own. Deliberately one entry: the owner named 15 V as the most a
 * car's electrical system is expected to show, and nothing else here is
 * anyone's measurement yet.
 */
const STARTERS: Array<{
  applies: (value: Pick<LiveReadValue, "name" | "unit">) => boolean;
  limits: LiveLimits;
}> = [
  {
    applies: (value) =>
      value.unit === "V" && /battery voltage|control module voltage/i.test(value.name),
    limits: { warnLow: 11.5, warnHigh: 15, alarmLow: null, alarmHigh: null },
  },
];

export function starterFor(value: Pick<LiveReadValue, "name" | "unit">): LiveLimits | null {
  const starter = STARTERS.find((entry) => entry.applies(value));
  return starter === undefined ? null : starter.limits;
}

/** The number behind a reading, where the reading is one. */
export function numberOf(value: LiveReadValue): number | null {
  if (value.value !== null) {
    const number = Number(value.value);
    if (Number.isFinite(number)) return number;
  }
  return value.raw;
}
