import { useCallback, useEffect, useState } from "react";
import {
  LanguageContext,
  languages,
  readStoredLanguage,
  setCurrentLanguage,
  t,
  type Language,
} from "./i18n";
import type { AdapterClient } from "./adapter";
import {
  BENCH_SCENARIO_DEFAULT,
  browserDemoEnabled,
  defaultAdapterClient,
  isBench,
} from "./adapter";
import type { CaptureClient } from "./capture";
import { defaultCaptureClient } from "./capture";
import { AdapterPanel } from "./components/AdapterPanel";
import { CapturePanel } from "./components/CapturePanel";
import { DiagnosticPanel } from "./components/DiagnosticPanel";
import { LiveReadPanel } from "./components/LiveReadPanel";
import { StandardObdPanel } from "./components/StandardObdPanel";
import { LibraryPanel } from "./components/LibraryPanel";
import { hasTauriRuntime } from "./files";
import { ModuleDetails } from "./components/ModuleDetails";
import { NetworkMap } from "./components/NetworkMap";
import { ReportPanel } from "./components/ReportPanel";
import { SessionPanel } from "./components/SessionPanel";
import type { RailVehicle } from "./components/SessionPanel";
import { StatusBadge } from "./components/StatusBadge";
import { VehicleCard } from "./components/VehicleCard";
import type { DiagnosticClient } from "./diagnostic";
import { defaultDiagnosticClient } from "./diagnostic";
import type { LibraryClient } from "./library";
import { defaultLibraryClient, markerLabel } from "./library";
import type { ModuleReadClient, ModuleReadSnapshot } from "./moduleRead";
import { defaultModuleReadClient } from "./moduleRead";
import { checkableModules, lanes } from "./networkMap";
import { startNewSession } from "./newSession";
import type { SessionReportClient } from "./sessionReport";
import type { SessionStep } from "./sessionReport";
import { defaultSessionReportClient, sessionSteps } from "./sessionReport";
import { useAdapterController } from "./useAdapterController";
import { bodyFor, vehicleImageUrl } from "./vehicleBody";
import { useCaptureController } from "./useCaptureController";
import { useDiagnosticController } from "./useDiagnosticController";
import { useLibraryController } from "./useLibraryController";
import { useLiveReadController } from "./useLiveReadController";
import { defaultLiveReadClient, type LiveReadClient } from "./liveRead";
import { useStandardObdController } from "./useStandardObdController";
import { defaultStandardObdClient, type StandardObdClient } from "./standardObd";
import { useModuleReadController } from "./useModuleReadController";
import { useNetworkCheckController } from "./useNetworkCheckController";
import { useSessionReportController } from "./useSessionReportController";
import "./styles.css";

interface AppProps {
  client?: AdapterClient;
  diagnosticClient?: DiagnosticClient;
  libraryClient?: LibraryClient;
  standardObdClient?: StandardObdClient;
  liveReadClient?: LiveReadClient;
  captureClient?: CaptureClient;
  moduleReadClient?: ModuleReadClient;
  sessionReportClient?: SessionReportClient;
  pollIntervalMs?: number;
  /** What restarts the interface once the shell has dropped the session; reloads the page by default. */
  onNewSession?: () => void;
}

const stepLabel = { done: "done", next: "next", todo: "to do" } as const;

/**
 * The interface follows the session: each section below does one or two of
 * its steps, in order, and the rail on the left is the same list.
 */
const sections: Array<{ id: string; title: string; steps: SessionStep["id"][] }> = [
  { id: "prepare", title: "Preparation", steps: ["adapter", "library"] },
  { id: "vehicle", title: "Vehicle", steps: ["vehicle"] },
  { id: "network", title: "Module network", steps: ["survey", "read"] },
  { id: "listen", title: "Listening and standard OBD-II", steps: ["capture"] },
  { id: "report", title: "Session report", steps: ["save"] },
];

function sectionOf(stepId: SessionStep["id"]) {
  return sections.find((section) => section.steps.includes(stepId)) ?? sections[0];
}

function sectionState(steps: SessionStep[], ids: SessionStep["id"][]) {
  const own = steps.filter((step) => ids.includes(step.id));
  if (own.some((step) => step.state === "next")) return "next" as const;
  if (own.every((step) => step.state === "done")) return "done" as const;
  return "todo" as const;
}

function stepNumbers(steps: SessionStep[], ids: SessionStep["id"][]) {
  const numbers = steps
    .map((step, index) => (ids.includes(step.id) ? index + 1 : null))
    .filter((number): number is number => number !== null);
  if (numbers.length === 2 && numbers[1] === numbers[0] + 1) return `${numbers[0]}–${numbers[1]}`;
  return numbers.join(", ");
}

function jumpTo(sectionId: string) {
  const element = document.getElementById(`section-${sectionId}`);
  if (element && typeof element.scrollIntoView === "function") {
    element.scrollIntoView({ behavior: "smooth", block: "start" });
  }
}

export function App({
  client = defaultAdapterClient,
  diagnosticClient = defaultDiagnosticClient,
  libraryClient = defaultLibraryClient,
  standardObdClient = defaultStandardObdClient,
  liveReadClient = defaultLiveReadClient,
  captureClient = defaultCaptureClient,
  moduleReadClient = defaultModuleReadClient,
  sessionReportClient = defaultSessionReportClient,
  pollIntervalMs,
  onNewSession,
}: AppProps) {
  const [language, setLanguage] = useState<Language>(() => {
    const stored = readStoredLanguage();
    setCurrentLanguage(stored);
    return stored;
  });
  const chooseLanguage = useCallback((next: Language) => {
    setCurrentLanguage(next);
    setLanguage(next);
  }, []);
  const library = useLibraryController(libraryClient);
  const capture = useCaptureController(captureClient, library.vehicle);
  const surveyModules = library.survey?.modules ?? [];
  const moduleRead = useModuleReadController(moduleReadClient, surveyModules, library.vehicle);
  const controller = useAdapterController(client, pollIntervalMs);
  const adapterReady =
    controller.snapshot.state === "CONNECTED" &&
    controller.snapshot.boardCommunication === "VERIFIED";
  const diagnostic = useDiagnosticController(diagnosticClient, adapterReady);
  const standardObd = useStandardObdController(standardObdClient, library.vehicle);
  const liveRead = useLiveReadController(liveReadClient, library.vehicle);
  const session = useSessionReportController(sessionReportClient);
  // The bench (ADR-0020): a virtual vehicle behind a stand-in adapter. The
  // whole screen says so, and the session is bench-only or real-only.
  const bench = isBench(controller.snapshot);
  const beginNewSession = useCallback(async () => {
    const confirmed = await startNewSession({
      title: t("Start a new session?"),
      message: t(
        "The report, the survey and the reads of this session are dropped unless saved. The adapter stays connected and the library stays loaded.",
      ),
      confirmLabel: t("Start"),
      cancelLabel: t("Cancel"),
    });
    if (!confirmed) return;
    if (onNewSession) onNewSession();
    else window.location.reload();
  }, [onNewSession]);
  const sessionMode = session.snapshot.mode;
  const sessionHasRecords = session.snapshot.reportAvailable;
  const { connect: connectReal, connectBench: connectBenchAdapter } = controller;
  /**
   * A session is on the bench or on a car, never both: switching while the
   * session holds records of the other kind starts a new session first, on
   * the user's confirmation, and the shell refuses the switch regardless.
   */
  const switchMode = useCallback(
    async (wanted: "bench" | "real", connect: () => Promise<void>) => {
      if (sessionHasRecords && sessionMode !== wanted) {
        const confirmed = await startNewSession({
          title: t("Start a new session?"),
          message: t(
            "A session is either on the bench or on a car, never both. The report, the survey and the reads of this session are dropped unless saved.",
          ),
          confirmLabel: t("Start"),
          cancelLabel: t("Cancel"),
        });
        if (!confirmed) return;
        await connect();
        if (onNewSession) onNewSession();
        else window.location.reload();
        return;
      }
      await connect();
    },
    [onNewSession, sessionHasRecords, sessionMode],
  );
  const connectAdapter = useCallback(() => switchMode("real", connectReal), [connectReal, switchMode]);
  const connectBench = useCallback(
    (scenario: number) => switchMode("bench", () => connectBenchAdapter(scenario)),
    [connectBenchAdapter, switchMode],
  );

  // Every module's latest read, from single reads and from the network check
  // alike; what the map colours its nodes by.
  const [results, setResults] = useState<Record<string, ModuleReadSnapshot>>({});
  const recordResult = useCallback((snapshot: ModuleReadSnapshot) => {
    if (snapshot.ecuFamily === "") return;
    setResults((previous) => ({ ...previous, [snapshot.ecuFamily]: snapshot }));
  }, []);
  const check = useNetworkCheckController(moduleReadClient, library.vehicle, recordResult);
  const [selectedModule, setSelectedModule] = useState<string | null>(null);
  const setModuleReadTarget = moduleRead.setEcuFamily;
  const selectModule = useCallback((ecuFamily: string) => {
    setSelectedModule(ecuFamily);
  }, []);
  useEffect(() => {
    // The read panel always targets the selected module.
    setModuleReadTarget(selectedModule ?? "");
  }, [selectedModule, setModuleReadTarget]);

  useEffect(() => {
    recordResult(moduleRead.snapshot);
  }, [moduleRead.snapshot, recordResult]);
  useEffect(() => {
    // A new survey answers for a different description; old reads do not
    // carry over. A selection is kept when the module is still in the
    // survey — this effect may run after a click that made it.
    const families = new Set((library.survey?.modules ?? []).map((module) => module.ecuFamily));
    setResults({});
    setSelectedModule((current) => (current !== null && families.has(current) ? current : null));
  }, [library.survey]);

  const refreshSession = session.refresh;
  useEffect(() => {
    // Every recorded capture or read changes what the session report holds.
    void refreshSession();
  }, [refreshSession, capture.snapshot, results, diagnostic.snapshot]);

  const steps = sessionSteps({
    adapterReady,
    libraryLoaded: library.library.state === "LOADED",
    vehicleDescribed:
      library.vehicle.vehicleProgram.trim() !== "" &&
      (library.catalogue.programmes.length === 0 || library.vehicle.yearBreakpoint !== null),
    surveyed: library.survey !== null,
    captures: session.snapshot.captures,
    moduleReads: session.snapshot.moduleReads,
    reportAvailable: session.snapshot.reportAvailable,
    bench,
  });
  const demoPreview = browserDemoEnabled();
  const railVehicle: RailVehicle | null = (() => {
    const programme = library.vehicle.vehicleProgram.trim();
    if (programme === "") return null;
    const entry = library.catalogue.programmes.find((candidate) => candidate.program === programme);
    const marker = entry?.markers.find((candidate) => candidate.marker === library.vehicle.yearBreakpoint);
    const model = library.vinDecode?.valid
      ? library.vinDecode.attributes.find((attribute) => /model/i.test(attribute.name))?.value
      : undefined;
    const details = [
      marker ? markerLabel(marker) : library.vehicle.modelYear !== null ? String(library.vehicle.modelYear) : null,
      library.vehicle.powertrain,
    ].filter((detail): detail is string => detail !== null && detail !== undefined && detail !== "");
    const body = bodyFor(programme, library.vinDecode?.valid ? library.vinDecode.attributes : []);
    const image = vehicleImageUrl(programme, library.vehicle.modelYear, body);
    return { title: model ? `${model} · ${programme}` : programme, details, body, image };
  })();
  const selected = surveyModules.find((module) => module.ecuFamily === selectedModule) ?? null;
  const checkable = checkableModules(
    lanes(surveyModules).flatMap((lane) => lane.modules),
    check.includeHypothesis,
  );
  const readingModule = check.current ?? (moduleRead.busy ? selectedModule : null);

  const adapterPill = bench ? (
    <StatusBadge tone="pending">{t("Bench: virtual vehicle")}</StatusBadge>
  ) : controller.snapshot.state === "CONNECTED" && adapterReady ? (
      <StatusBadge tone="positive">{t("Adapter ready")}</StatusBadge>
    ) : controller.snapshot.state === "CONNECTED" ? (
      <StatusBadge tone="pending">{t("Adapter connected, board unverified")}</StatusBadge>
    ) : controller.snapshot.state === "ADAPTER_DETECTED" ? (
      <StatusBadge tone="pending">{t("Adapter found, not connected")}</StatusBadge>
    ) : controller.snapshot.state === "ERROR" ? (
      <StatusBadge tone="negative">{t("Adapter error")}</StatusBadge>
    ) : (
      <StatusBadge tone="neutral">{t("No adapter")}</StatusBadge>
    );
  const libraryPill =
    library.library.state === "LOADED" || library.library.state === "PARTIALLY_LOADED" ? (
      <StatusBadge tone="positive">
        {t("Library: {records} records", {
          records: library.library.records.toLocaleString(language === "uk" ? "uk-UA" : "en-GB"),
        })}
      </StatusBadge>
    ) : library.library.state === "FAILED" ? (
      <StatusBadge tone="negative">{t("Library failed to load")}</StatusBadge>
    ) : (
      <StatusBadge tone="neutral">{t("Library: built-in only")}</StatusBadge>
    );

  return (
    <LanguageContext.Provider value={language}>
    <div className={bench ? "app-shell app-shell--bench" : "app-shell"} lang={language}>
      <header className="app-header">
        <div className="brand">
          <div className="brand-name">
            <h1>ProwlOne</h1>
            <p className="brand-tagline">{t("Multi-platform vehicle diagnostics")}</p>
          </div>
          {demoPreview ? <span className="demo-badge">{t("Simulator UI preview")}</span> : null}
        </div>
        <div className="header-status">
          {adapterPill}
          {libraryPill}
          <button
            className="button button--secondary"
            type="button"
            onClick={() => void beginNewSession()}
          >
            {t("New session")}
          </button>
          <label className="language-switch">
            <span>{t("Language")}</span>
            <select
              name="language"
              value={language}
              onChange={(event) => chooseLanguage(event.target.value as Language)}
            >
              {languages.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {entry.label}
                </option>
              ))}
            </select>
          </label>
        </div>
      </header>
      {bench ? (
        <div className="bench-band" role="status">
          {t("BENCH · virtual vehicle · synthetic data")} · {t("Scenario")}{" "}
          {controller.snapshot.benchScenario ?? BENCH_SCENARIO_DEFAULT}
        </div>
      ) : null}
      {demoPreview ? (
        <div className="demo-banner" role="status">
          {t("Development preview only — no Mongoose or vehicle communication.")}
        </div>
      ) : null}
      <main className="flow">
        <SessionPanel
          steps={steps}
          onJump={(stepId) => jumpTo(sectionOf(stepId).id)}
          vehicle={railVehicle}
        />
        <div className="flow-body">
          {sections.map((section) => {
            const state = sectionState(steps, section.steps);
            const numbers = stepNumbers(steps, section.steps);
            const heading = (
              <header className="flow-section-heading">
                <span className="flow-section-steps">
                  {section.steps.length > 1
                    ? t("Steps {list}", { list: numbers })
                    : t("Step {list}", { list: numbers })}
                </span>
                <h2 id={`section-${section.id}-title`}>{t(section.title)}</h2>
                <StatusBadge
                  tone={state === "done" ? "positive" : state === "next" ? "pending" : "neutral"}
                >
                  {t(stepLabel[state])}
                </StatusBadge>
              </header>
            );
            return (
              <section
                key={section.id}
                id={`section-${section.id}`}
                className={`flow-section flow-section--${state}`}
                aria-labelledby={`section-${section.id}-title`}
              >
                {heading}
                {section.id === "prepare" ? (
                  <div className="flow-two-up">
                    <AdapterPanel
                      snapshot={controller.snapshot}
                      selectedPort={controller.selectedPort}
                      onSelectPort={controller.setSelectedPort}
                      onDetect={() => void controller.refresh()}
                      onConnect={() => void connectAdapter()}
                      onConnectBench={(scenario) => void connectBench(scenario)}
                      onDisconnect={() => void controller.disconnect()}
                    />
                    <LibraryPanel
                      snapshot={library.library}
                      directory={library.directory}
                      busy={library.busy}
                      onDirectoryChange={library.setDirectory}
                      onChooseDirectory={
                        hasTauriRuntime() ? () => void library.chooseDirectory() : undefined
                      }
                      onLoad={() => void library.load()}
                    />
                  </div>
                ) : null}
                {section.id === "vehicle" ? (
                  <div className="flow-vehicle">
                    <VehicleCard
                      vehicle={library.vehicle}
                      catalogue={library.catalogue}
                      survey={library.survey}
                      busy={library.busy}
                      libraryReady={library.library.state !== "FAILED"}
                      vin={library.vin}
                      vinDecode={library.vinDecode}
                      decodingVin={library.decodingVin}
                      onVinChange={library.setVin}
                      onDecodeVin={() => void library.decodeVin()}
                      onVehicleChange={library.setVehicle}
                      onSurvey={() => void library.runSurvey()}
                    />
                  </div>
                ) : null}
                {section.id === "network" ? (
                  <div className="cockpit" aria-label="Vehicle cockpit">
                    <NetworkMap
                      survey={library.survey}
                      results={results}
                      readingModule={readingModule}
                      selected={selectedModule}
                      adapterReady={adapterReady}
                      checking={check.progress}
                      checkable={checkable.length}
                      includeHypothesis={check.includeHypothesis}
                      onSelect={selectModule}
                      onCheckAll={() => void check.run(checkable)}
                      onCancelCheck={check.stop}
                      onIncludeHypothesisChange={check.setIncludeHypothesis}
                    />
                    <ModuleDetails
                      module={selected}
                      outcome={selected !== null ? results[selected.ecuFamily] : undefined}
                      kind={moduleRead.kind}
                      identifier={moduleRead.identifier}
                      adapterReady={adapterReady}
                      busy={moduleRead.busy || check.current === selectedModule}
                      saving={moduleRead.saving}
                      bench={bench}
                      onKindChange={moduleRead.setKind}
                      onIdentifierChange={moduleRead.setIdentifier}
                      onRead={() => void moduleRead.read()}
                      onSaveReport={() => void moduleRead.saveReport()}
                    />
                  </div>
                ) : null}
                {section.id === "network" ? (
                  <LiveReadPanel
                    snapshot={liveRead.snapshot}
                    set={liveRead.set}
                    modules={surveyModules}
                    running={liveRead.running}
                    busy={liveRead.busy}
                    adapterReady={adapterReady}
                    bench={bench}
                    onToggle={liveRead.toggle}
                    onClearSet={liveRead.clearSet}
                    onStart={() => void liveRead.start()}
                    onStop={() => void liveRead.stop()}
                  />
                ) : null}
                {section.id === "listen" ? (
                  <div className="flow-two-up">
                    <CapturePanel
                      snapshot={capture.snapshot}
                      routeId={capture.routeId}
                      seconds={capture.seconds}
                      adapterReady={adapterReady}
                      busy={capture.busy}
                      saving={capture.saving}
                      bench={bench}
                      onRouteChange={capture.setRouteId}
                      onSecondsChange={capture.setSeconds}
                      onListen={() => void capture.listen()}
                      onSave={() => void capture.save()}
                    />
                    <DiagnosticPanel
                      snapshot={diagnostic.snapshot}
                      adapterReady={adapterReady}
                      saving={diagnostic.saving}
                      bench={bench}
                      onRead={() => void diagnostic.read()}
                      onSaveReport={() => void diagnostic.saveReport()}
                    />
                  </div>
                ) : null}
                {section.id === "listen" ? (
                  <StandardObdPanel
                    snapshot={standardObd.snapshot}
                    values={standardObd.values}
                    frameValues={standardObd.frameValues}
                    monitors={standardObd.monitors}
                    information={standardObd.information}
                    busy={standardObd.busy}
                    adapterReady={adapterReady}
                    responder={standardObd.responder}
                    bench={bench}
                    onResponderChange={standardObd.setResponder}
                    onRead={(kind, items) => void standardObd.read(kind, items)}
                    onReadEverything={() => void standardObd.readEverything()}
                    onReadFreezeFrame={() => void standardObd.readFreezeFrame()}
                    onReadMonitors={() => void standardObd.readMonitors()}
                    onReadVehicleInformation={() => void standardObd.readVehicleInformation()}
                    onClear={standardObd.clear}
                  />
                ) : null}
                {section.id === "report" ? (
                  <ReportPanel
                    snapshot={session.snapshot}
                    error={session.error}
                    saving={session.saving}
                    bench={bench}
                    preview={session.preview}
                    onSave={() => void session.save()}
                    onPreview={() => void session.showPreview()}
                    onHidePreview={session.hidePreview}
                  />
                ) : null}
              </section>
            );
          })}
        </div>
      </main>
      {bench ? (
        <div className="bench-band">
          {t("BENCH · virtual vehicle · synthetic data")} · {t("Scenario")}{" "}
          {controller.snapshot.benchScenario ?? BENCH_SCENARIO_DEFAULT}
        </div>
      ) : null}
      <footer className="app-footer">
        <span>
          {bench
            ? t("Bench: virtual vehicle from the library; every value is synthetic.")
            : controller.snapshot.state === "CONNECTED"
            ? t("Adapter connected and board communication verified.")
            : controller.snapshot.state === "ADAPTER_DETECTED"
              ? t("Adapter detected.")
              : controller.snapshot.state === "ERROR"
                ? controller.snapshot.error?.message
                : t("Adapter not detected.")}
        </span>
        <span>{t("Live vehicle status: not yet externally validated.")}</span>
        <span className="app-build" title={t("Build: quote it when you report")}>
          {`${__APP_VERSION__} · ${__BUILD_SHA__}`}
        </span>
      </footer>
    </div>
    </LanguageContext.Provider>
  );
}
