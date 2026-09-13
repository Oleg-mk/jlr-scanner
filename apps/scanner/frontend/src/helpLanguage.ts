import { useEffect, useState } from "react";
import type { Language } from "./i18n";

/**
 * The language of the text SDD wrote — the fault-code help, the self-test
 * descriptions — as distinct from the interface's language (ADR-0034). SDD
 * ships that text in English and in Russian and in no Ukrainian, so the
 * choice here is between those two and is remembered on this machine. A
 * Ukrainian interface reads Russian unless told otherwise.
 */
export type HelpLanguage = "eng" | "rus";

const STORAGE_KEY = "prowlone.help-language";
const listeners = new Set<() => void>();
let chosen: HelpLanguage | null = null;

export function defaultHelpLanguage(language: Language): HelpLanguage {
  return language === "en" ? "eng" : "rus";
}

function readStored(): HelpLanguage | null {
  try {
    const stored = window.localStorage.getItem(STORAGE_KEY);
    return stored === "eng" || stored === "rus" ? stored : null;
  } catch {
    return null;
  }
}

/** The language the SDD text is read in right now, for this interface language. */
export function helpLanguageFor(language: Language): HelpLanguage {
  return chosen ?? readStored() ?? defaultHelpLanguage(language);
}

export function setHelpLanguage(next: HelpLanguage) {
  chosen = next;
  try {
    window.localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // A browser that refuses storage still gets the choice for this session.
  }
  for (const listener of listeners) listener();
}

/** Forget the choice — for tests, so one test's click does not reach the next. */
export function resetHelpLanguage() {
  chosen = null;
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Nothing to forget.
  }
  for (const listener of listeners) listener();
}

/**
 * The current choice and a setter, for components that must re-render when
 * it changes — every panel showing SDD's text shares one choice.
 */
export function useHelpLanguage(
  language: Language,
): [HelpLanguage, (next: HelpLanguage) => void] {
  const [, bump] = useState(0);
  useEffect(() => {
    const listener = () => bump((n) => n + 1);
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  }, []);
  return [helpLanguageFor(language), setHelpLanguage];
}
