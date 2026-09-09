import { useCallback, useEffect, useState } from "react";
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

  return { snapshot, saving, error, preview, refresh, save, showPreview, hidePreview };
}
