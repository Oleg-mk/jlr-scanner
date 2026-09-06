import { invoke } from "@tauri-apps/api/core";
import { hasTauriRuntime } from "./files";

/** A type alias, not an interface: `invoke` wants an object with an index signature. */
export type NewSessionStrings = {
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
};

/**
 * Ask the user, then drop the session's state in the shell — report, survey,
 * captures, reads — while the adapter and the library stay. Resolves to true
 * when the user confirmed and the shell reset; the caller then restarts the
 * interface. In the browser preview a plain confirm stands in and there is
 * no shell state to drop.
 */
export async function startNewSession(strings: NewSessionStrings): Promise<boolean> {
  if (hasTauriRuntime()) {
    return invoke<boolean>("start_new_session", strings);
  }
  return window.confirm(`${strings.title}\n\n${strings.message}`);
}
