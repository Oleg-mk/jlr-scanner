import { useCallback, useEffect, useState } from "react";
import { t } from "./i18n";
import {
  buildReport,
  revealInFolder,
  saveReportHtml,
  type PrintableReport,
  type SessionBundle,
} from "./printableReport";
import {
  createSessionReportSnapshot,
  saveSessionReportFile,
  type SessionReportClient,
  type SessionReportSnapshot,
} from "./sessionReport";

export function useSessionReportController(client: SessionReportClient) {
  const [snapshot, setSnapshot] = useState<SessionReportSnapshot>(createSessionReportSnapshot);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The bench (ADR-0020): the report is read for the screen, never for a file.
  const [preview, setPreview] = useState<string | null>(null);
  // The readable report (ADR-0031), once built, and whether the VIN and the
  // adapter's serial are masked in it.
  const [document, setDocument] = useState<PrintableReport | null>(null);
  const [mask, setMask] = useState(false);
  const [savedPath, setSavedPath] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      setSnapshot(await client.getState());
    } catch {
      // A missing state leaves the last known counts; the save path reports its own error.
    }
  }, [client]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const save = useCallback(async () => {
    setSaving(true);
    setError(null);
    try {
      await saveSessionReportFile(await client.getReportJson());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    } finally {
      setSaving(false);
    }
  }, [client]);

  const showPreview = useCallback(async () => {
    setError(null);
    try {
      setPreview(await client.getReportJson());
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [client]);

  const hidePreview = useCallback(() => setPreview(null), []);

  /**
   * The readable report (ADR-0031): the same session as a document. Built
   * on demand from the bundle, in the interface's language, with the VIN
   * and the adapter's serial masked when the tester asks.
   */
  const buildDocument = useCallback(
    async (masked: boolean): Promise<PrintableReport | null> => {
      setError(null);
      try {
        const bundle = JSON.parse(await client.getReportJson()) as SessionBundle;
        const built = buildReport(bundle, t, masked);
        setDocument(built);
        return built;
      } catch (caught) {
        setError(caught instanceof Error ? caught.message : String(caught));
        return null;
      }
    },
    [client],
  );

  const showDocument = useCallback(async () => {
    await buildDocument(mask);
  }, [buildDocument, mask]);

  const hideDocument = useCallback(() => setDocument(null), []);

  const setMasked = useCallback(
    async (next: boolean) => {
      setMask(next);
      if (document !== null) await buildDocument(next);
    },
    [buildDocument, document],
  );

  /** Print through the platform's own dialog, where a PDF writer is a printer. */
  const print = useCallback(async () => {
    if (document === null && (await buildDocument(mask)) === null) return;
    // The document is in the page; the print stylesheet hides everything else.
    window.print();
  }, [buildDocument, document, mask]);

  /** Save the document as one web page, styles and all. */
  const saveDocument = useCallback(async () => {
    if (document === null && (await buildDocument(mask)) === null) return;
    const node = window.document.getElementById("printable-report");
    if (node === null) return;
    const page = [
      "<!doctype html>",
      '<html><head><meta charset="utf-8">',
      `<title>${t("Diagnostic session report")}</title>`,
      "</head><body>",
      node.outerHTML,
      "</body></html>",
    ].join("");
    try {
      setSavedPath(await saveReportHtml(page, Date.now()));
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [buildDocument, document, mask]);

  const reveal = useCallback(async () => {
    if (savedPath === null) return;
    try {
      await revealInFolder(savedPath);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : String(caught));
    }
  }, [savedPath]);

  return {
    snapshot,
    saving,
    error,
    preview,
    refresh,
    save,
    showPreview,
    hidePreview,
    document,
    mask,
    setMasked,
    showDocument,
    hideDocument,
    print,
    saveDocument,
    savedPath,
    reveal,
  };
}
