import type { DecodedAttribute } from "./library";

/**
 * The body type a car is drawn as. The owner's artwork is one picture per
 * model (`public/vehicles/<programme>[-variant].webp`, see `PROGRAMME_ART`);
 * a programme without a picture may still have a body-type image in
 * `public/vehicles/<body>.png`, and without either the rail draws its mark.
 */
export type BodyType =
  | "sedan"
  | "coupe"
  | "cabrio"
  | "wagon"
  | "large-suv"
  | "compact-suv"
  | "defender";

/** The typical body of each SDD programme; the VIN decode refines it. */
const PROGRAMME_BODY: Record<string, BodyType> = {
  // Jaguar
  X100: "coupe",
  X103: "cabrio",
  X200: "sedan",
  X150: "coupe",
  X152: "coupe",
  X202: "sedan",
  X204: "sedan",
  X206: "sedan",
  X250: "sedan",
  X260: "sedan",
  X350: "sedan",
  X356: "sedan",
  X358: "sedan",
  X351: "sedan",
  X400: "sedan",
  X404: "wagon",
  X760: "sedan",
  X761: "large-suv",
  // Land Rover
  L316: "defender",
  L319: "large-suv",
  L320: "large-suv",
  L322: "large-suv",
  L359: "compact-suv",
  L405: "large-suv",
  L494: "large-suv",
  L538: "compact-suv",
  L538C: "cabrio",
  L538JV: "compact-suv",
  L550: "compact-suv",
  L551: "compact-suv",
  L560: "large-suv",
  L460: "large-suv",
  L461: "large-suv",
  L462: "large-suv",
  L663: "defender",
  // The browser demo's synthetic car, so the owner's artwork can be previewed there.
  SYNTHA: "sedan",
};

/**
 * The body type for a programme, refined by what the VIN decode says about
 * the body when it says anything: an estate is a wagon whatever the
 * programme, a convertible a cabrio, a coupé a coupe.
 */
export function bodyFor(programme: string, decoded: DecodedAttribute[] = []): BodyType | null {
  const fromVin = decoded
    .map((attribute) => attribute.value.toLowerCase())
    .reduce<BodyType | null>((found, value) => {
      if (found) return found;
      if (/sportbrake|estate|wagon|універсал|универсал/.test(value)) return "wagon";
      if (/convertible|cabrio|roadster|кабріолет|кабриолет/.test(value)) return "cabrio";
      if (/coup[eé]|купе/.test(value)) return "coupe";
      return null;
    }, null);
  return fromVin ?? PROGRAMME_BODY[programme.trim().toUpperCase()] ?? null;
}

/** Where the image for a body type is served from, inside the application. */
export function bodyImageUrl(body: BodyType) {
  return `/vehicles/${body}.png`;
}

/**
 * One of the owner's pictures: the file under `public/vehicles/`, and when
 * it applies — from a model year on, or for one body only. The first entry
 * of a programme is its default.
 */
interface ProgrammeArt {
  file: string;
  from?: number;
  body?: BodyType;
}

/** The owner's pictures, 2026-09-06: one per model, by SDD programme; 34 files. */
const PROGRAMME_ART: Record<string, ProgrammeArt[]> = {
  // Jaguar
  X100: [{ file: "X100" }],
  X200: [{ file: "X202" }],
  X202: [{ file: "X202" }],
  X204: [{ file: "X202" }],
  X206: [{ file: "X202" }],
  X250: [{ file: "X250" }, { file: "X250-from2012", from: 2012 }],
  X260: [{ file: "X260" }, { file: "X260-from2021", from: 2021 }, { file: "X260-wagon", body: "wagon" }],
  X350: [{ file: "X350" }],
  X356: [{ file: "X350" }],
  X358: [{ file: "X358" }],
  X351: [{ file: "X351" }, { file: "X351-from2016", from: 2016 }],
  X150: [{ file: "X150-coupe" }, { file: "X150-cabrio", body: "cabrio" }],
  X152: [{ file: "X152-coupe" }, { file: "X152-cabrio", body: "cabrio" }],
  X400: [{ file: "X400" }],
  X404: [{ file: "X400" }],
  X760: [{ file: "X760" }],
  X761: [{ file: "X761" }],
  // Land Rover
  L316: [{ file: "L316" }],
  L663: [{ file: "L663" }],
  L322: [{ file: "L322" }],
  L405: [{ file: "L405" }],
  L460: [{ file: "L460" }],
  L320: [{ file: "L320" }],
  L494: [{ file: "L494" }],
  L461: [{ file: "L461" }],
  L538: [{ file: "L538" }],
  L538C: [{ file: "L538" }],
  L538JV: [{ file: "L538" }],
  L551: [{ file: "L551" }],
  L560: [{ file: "L560" }],
  L319: [{ file: "L319" }, { file: "L319-from2010", from: 2010 }],
  L462: [{ file: "L462" }],
  L359: [{ file: "L359" }],
  L550: [{ file: "L550" }],
  // The browser demo's synthetic car wears the XF's picture.
  SYNTHA: [{ file: "X250" }],
};

/**
 * The owner's picture for a car: the programme's variant for the body when
 * one exists (a convertible, an estate), else the latest variant whose
 * model year has been reached, else the programme's default; `null` when
 * the programme has no picture.
 */
export function vehicleImageUrl(
  programme: string,
  modelYear: number | null,
  body: BodyType | null,
): string | null {
  const art = PROGRAMME_ART[programme.trim().toUpperCase()];
  if (!art || art.length === 0) return null;
  const forBody = body === null ? undefined : art.find((entry) => entry.body === body);
  const byYear = art
    .filter((entry) => entry.body === undefined)
    .filter((entry) => entry.from === undefined || (modelYear !== null && modelYear >= entry.from))
    .reduce<ProgrammeArt | undefined>(
      (best, entry) => (best === undefined || (entry.from ?? 0) >= (best.from ?? 0) ? entry : best),
      undefined,
    );
  const chosen = forBody ?? byYear ?? art[0];
  return `/vehicles/${chosen.file}.webp`;
}
