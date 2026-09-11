import { invoke } from "@tauri-apps/api/core";
import { hasTauriRuntime } from "./files";
import { currentLanguage, productLanguage, type Language } from "./i18n";

/**
 * The parameter names in the interface's language (ADR-0025).
 *
 * Every reading arrives named in SDD's English. The table of our own wording
 * is compiled into the application, not carried by the loaded library, and is
 * fetched once per language and kept. A name the table does not carry is
 * shown in its English — one name at a time, never guessed at, never filled
 * from the other language.
 *
 * The English is the identity: it is what the library holds, what a tester
 * quotes, and what every report keeps whatever the interface says.
 */
export interface ParameterNameClient {
  load(language: "ukr" | "rus"): Promise<Record<string, string>>;
}

class TauriParameterNameClient implements ParameterNameClient {
  load(language: "ukr" | "rus") {
    return invoke<Record<string, string>>("get_parameter_names", { language });
  }
}

/** In a browser there is no table, so every name stays English. */
class BrowserParameterNameClient implements ParameterNameClient {
  load() {
    return Promise.resolve({});
  }
}

export const defaultParameterNameClient: ParameterNameClient = hasTauriRuntime()
  ? new TauriParameterNameClient()
  : new BrowserParameterNameClient();

let loaded: Language | null = null;
let names: Record<string, string> = {};
/** Which request is the current one: a slower earlier answer must not win. */
let pending = 0;

/**
 * Fetch the table for `language`, once. English needs none. Resolves to true
 * when the names on screen should be redrawn.
 */
export async function loadParameterNames(
  language: Language,
  client: ParameterNameClient = defaultParameterNameClient,
): Promise<boolean> {
  if (loaded === language) return false;
  const token = (pending += 1);
  const code = productLanguage(language);
  let table: Record<string, string> = {};
  if (code !== null) {
    try {
      table = await client.load(code);
    } catch {
      // A table that will not load leaves the names in English, which is
      // what a missing row does too. Nothing is invented to cover it.
      table = {};
    }
  }
  // Languages can be switched faster than a table arrives. An answer that is
  // no longer the one asked for is dropped rather than shown.
  if (token !== pending) return false;
  loaded = language;
  names = table;
  return true;
}

/** Our word for a parameter, or SDD's English when we have none. */
export function parameterName(english: string): string {
  if (loaded !== currentLanguage()) return english;
  return names[english] ?? english;
}

/** For tests: forget what was loaded. */
export function resetParameterNames() {
  loaded = null;
  names = {};
  pending = 0;
}
