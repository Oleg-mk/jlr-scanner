import { useCallback, useEffect, useState } from "react";
import type { ModuleSurveyEntry, VehicleDescription } from "./library";
import {
  createModuleReadSnapshot,
  saveModuleReadReportFile,
  type ModuleReadClient,
  type ModuleReadKind,
  type ModuleReadSnapshot,
} from "./moduleRead";

function failedRead(ecuFamily: string, error: unknown): ModuleReadSnapshot {
  return {
    ...createModuleReadSnapshot(),
    state: "FAILED",
    ecuFamily,
    error: {
      category: "INTERNAL_FAILURE",
      message: "Module read failed",
      technicalDetails: error instanceof Error ? error.message : String(error),
      stage: "FRONTEND_COMMAND",
    },
  };
}

/** Modules a read may be attempted on: routed, or routed on a hypothesis. */
export function readableModules(modules: ModuleSurveyEntry[]): ModuleSurveyEntry[] {
  return modules.filter(
    (module) =>
      ["REACHABLE", "HYPOTHESIS"].includes(module.identifierRead.status) ||
      ["REACHABLE", "HYPOTHESIS"].includes(module.dtcRead.status),
  );
}

export function useModuleReadController(
  client: ModuleReadClient,
  modules: ModuleSurveyEntry[],
  vehicle: VehicleDescription,
) {
  const [snapshot, setSnapshot] = useState<ModuleReadSnapshot>(createModuleReadSnapshot);
  const [ecuFamily, setEcuFamily] = useState("");
  const [kind, setKind] = useState<ModuleReadKind>("FAULT_CODES");
  const [identifier, setIdentifier] = useState("");
  const [busy, setBusy] = useState(false);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let active = true;
    void client
      .getState()
      .then((next) => {
        if (active) setSnapshot(next);
      })
      .catch((error: unknown) => {
        if (active) setSnapshot(failedRead("", error));
      });
    return () => {
      active = false;
    };
  }, [client]);

  const candidates = readableModules(modules);
  const selected = candidates.find((module) => module.ecuFamily === ecuFamily) ?? null;

  const read = useCallback(async () => {
    if (selected === null) return;
    setBusy(true);
    try {
      setSnapshot(
        await client.read({
          ecuFamily: selected.ecuFamily,
          kind,
          identifier: kind === "IDENTIFIER" ? identifier || null : null,
          context: vehicle,
        }),
      );
    } catch (error) {
      setSnapshot(failedRead(selected.ecuFamily, error));
    } finally {
      setBusy(false);
    }
  }, [client, identifier, kind, selected, vehicle]);

  const saveReport = useCallback(async () => {
    setSaving(true);
    try {
      await saveModuleReadReportFile(await client.getReportJson());
    } catch (error) {
      setSnapshot(failedRead(ecuFamily, error));
    } finally {
      setSaving(false);
    }
  }, [client, ecuFamily]);

  return {
    snapshot,
    candidates,
    selected,
    ecuFamily,
    setEcuFamily,
    kind,
    setKind,
    identifier,
    setIdentifier,
    busy,
    saving,
    read,
    saveReport,
  };
}
