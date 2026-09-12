import { invoke } from "@tauri-apps/api/core";

/**
 * Saving a file and choosing a folder.
 *
 * Inside the desktop application both go through native dialogs opened by
 * the shell, which then writes the file itself and reports where; a
 * webview's own "download" link is not something a tester can rely on. In
 * the browser preview the file is offered as a download and no folder can
 * be chosen.
 */

export function hasTauriRuntime() {
  return (
    typeof window !== "undefined" &&
    "__TAURI_INTERNALS__" in (window as Window & { __TAURI_INTERNALS__?: unknown })
  );
}

/**
 * Save text under a suggested name; resolves to the path written, or null
 * when the user cancelled. The format picks the dialog's filter and, in the
 * browser preview, the type of the offered download; `json` unless the
 * caller says `csv` (ADR-0022 §5, amended).
 */
export async function saveTextFile(
  suggestedName: string,
  contents: string,
  format: "json" | "csv" = "json",
): Promise<string | null> {
  if (hasTauriRuntime()) {
    return invoke<string | null>("save_text_file", { suggestedName, contents, format });
  }
  const type = format === "csv" ? "text/csv" : "application/json";
  const url = URL.createObjectURL(new Blob([contents], { type }));
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = suggestedName;
  anchor.click();
  URL.revokeObjectURL(url);
  return suggestedName;
}

/** Let the user choose a folder; resolves to its path, or null when cancelled or unavailable. */
export async function pickDirectory(): Promise<string | null> {
  if (!hasTauriRuntime()) return null;
  return invoke<string | null>("pick_directory");
}
