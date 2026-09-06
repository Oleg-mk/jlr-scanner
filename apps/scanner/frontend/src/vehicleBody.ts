import type { DecodedAttribute } from "./library";

/**
 * The body type a car is drawn as. One image per body type lives in
 * `public/vehicles/<body>.png` — the owner's own artwork — and the rail
 * shows it once the vehicle is known; a body type without a file falls
 * back to the drawn silhouette.
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
