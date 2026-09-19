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
  defaultParameterNameClient,
  loadParameterNames,
  type ParameterNameClient,
} from "./parameterNames";
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
import { MileagePanel } from "./components/MileagePanel";
import { PassportPanel } from "./components/PassportPanel";
import { BatteryCard } from "./components/BatteryCard";
import { BatteryPrecondition } from "./components/BatteryPrecondition";
import { batteryPreconditionText } from "./batteryFormat";
import { latestVoltage } from "./voltage";
import { BatteryPanel } from "./components/BatteryPanel";
import { CcfPanel } from "./components/CcfPanel";
import { StandardObdPanel } from "./components/StandardObdPanel";
import { LibraryPanel } from "./components/LibraryPanel";
import { hasTauriRuntime } from "./files";
import { FaultSummary } from "./components/FaultSummary";
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
import { useMileageController } from "./useMileageController";
import { defaultMileageClient, type MileageClient } from "./mileage";
import { usePassportController } from "./usePassportController";
import { defaultPassportClient, type PassportClient } from "./passport";
import { useBatteryController } from "./useBatteryController";
import { useCcfController } from "./useCcfController";
import { defaultBatteryClient, type BatteryClient } from "./battery";
import { defaultCcfClient, type CcfClient } from "./ccf";
import { defaultLiveReadClient, type LiveReadClient } from "./liveRead";
import { useStandardObdController } from "./useStandardObdController";
import { defaultStandardObdClient, type StandardObdClient } from "./standardObd";
import { useModuleReadController } from "./useModuleReadController";
import { useNetworkCheckController } from "./useNetworkCheckController";
import { useSessionReportController } from "./useSessionReportController";
import { ConfirmDialog } from "./components/ConfirmDialog";
import { defaultServiceClient, type ServiceClient } from "./serviceMode";
import { useServiceModeController } from "./useServiceModeController";
import "./styles.css";

interface AppProps {
  client?: AdapterClient;
  diagnosticClient?: DiagnosticClient;
  libraryClient?: LibraryClient;
  standardObdClient?: StandardObdClient;
  liveReadClient?: LiveReadClient;
  mileageClient?: MileageClient;
  passportClient?: PassportClient;
  ccfClient?: CcfClient;
  batteryClient?: BatteryClient;
  captureClient?: CaptureClient;
  moduleReadClient?: ModuleReadClient;
  sessionReportClient?: SessionReportClient;
  /** The service mode and its operations (ADR-0036). */
  serviceClient?: ServiceClient;
  parameterNameClient?: ParameterNameClient;
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
  mileageClient = defaultMileageClient,
  passportClient = defaultPassportClient,
  ccfClient = defaultCcfClient,
  batteryClient = defaultBatteryClient,
  captureClient = defaultCaptureClient,
  moduleReadClient = defaultModuleReadClient,
  sessionReportClient = defaultSessionReportClient,
  serviceClient = defaultServiceClient,
  parameterNameClient = defaultParameterNameClient,
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
  // The parameter names (ADR-0025) are compiled into the application, not
  // carried by the loaded library. Fetch them once per language; until they
  // arrive, and for English, every name stays as SDD wrote it.
  const [, setNamesLoaded] = useState(0);
  useEffect(() => {
    let live = true;
    void loadParameterNames(language, parameterNameClient).then((changed) => {
      if (live && changed) setNamesLoaded((count) => count + 1);
    });
    return () => {
      live = false;
    };
  }, [language, parameterNameClient]);
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
  const mileage = useMileageController(mileageClient, library.vehicle);
  const passport = usePassportController(passportClient, library.vehicle);
  const ccf = useCcfController(ccfClient, library.vehicle);
  const battery = useBatteryController(batteryClient, library.vehicle);
  const session = useSessionReportController(sessionReportClient);
  // The service mode (ADR-0036): the session's own switch, behind one
  // consent; the shell keeps the truth of it in the session snapshot.
  const service = useServiceModeController(
    serviceClient,
    library.vehicle,
    session.snapshot.serviceMode,
    session.refresh,
  );
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
        let confirmed = false;
        try {
          confirmed = await startNewSession({
            title: t("Start a new session?"),
            message: t(
              "A session is either on the bench or on a car, never both. The report, the survey and the reads of this session are dropped unless saved.",
            ),
            confirmLabel: t("Start"),
            cancelLabel: t("Cancel"),
          });
        } catch (error) {
          // The shell could not ask. Saying so beats doing nothing.
          controller.reportRefusal(
            t("This session already holds records, and the question about starting a new one could not be asked."),
            error instanceof Error ? error.message : String(error),
          );
          return;
        }
        if (!confirmed) {
          controller.reportRefusal(
            t(
              "Nothing was connected: a session is either on the bench or on a car, and this one already holds records. Start a new session to change over.",
            ),
            null,
          );
          return;
        }
        await connect();
        if (onNewSession) onNewSession();
        else window.location.reload();
        return;
      }
      await connect();
    },
    [controller, onNewSession, sessionHasRecords, sessionMode],
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
  // The name in the corner is the instrument's own lamp (2026-09-13). Before
  // there is a car on the other end of the lead it beats — Jaguar's own
  // rhythm, one, a short pause, two, then the longer rest. Once the adapter
  // is verified and the car is described, the beating stops and it settles
  // to a quiet green: reading, and only reading. The third state is red and
  // steady, for when something may be sent rather than asked: the service
  // mode of ADR-0036, while it is on.
  // The rail's voltage, from whichever read last brought one (ADR-0030,
  // 2026-09-19): the key on the plate shows it, and the precondition line
  // stands on it.
  const voltage = latestVoltage({
    battery: battery.snapshot,
    obdValues: standardObd.values,
    obdAtMs: standardObd.valuesAtMs,
    obdRouteValidation: standardObd.snapshot.routeValidation,
    live: liveRead.snapshot,
    liveStartedAtMs: liveRead.startedAtMs,
  });
  const batteryNotice = batteryPreconditionText(
    voltage,
    battery.snapshot.sddLowVoltageMaxMv,
    Date.now(),
  );
  const lamp: "waiting" | "live" | "sending" = service.on
    ? "sending"
    : adapterReady && library.vehicle.vehicleProgram.trim() !== ""
      ? "live"
      : "waiting";
  // The card is offered when there is something to read it with and a car
  // to read it from; the reason is said rather than the button hidden.
  const batteryDisabledReason = !adapterReady
    ? t("Connect the adapter first.")
    : library.vehicle.vehicleProgram.trim() === ""
      ? t("Choose the vehicle first.")
      : null;
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

  // The bench says itself on this badge - its own colour, the scenario in a
  // circle - since the owner withdrew the bands (2026-09-18).
  const benchScenario = controller.snapshot.benchScenario ?? BENCH_SCENARIO_DEFAULT;
  const adapterPill = bench ? (
    <StatusBadge
      tone="bench"
      mark={{ value: benchScenario, label: t("Scenario {scenario}", { scenario: benchScenario }) }}
    >
      {t("Bench")}
    </StatusBadge>
  ) : controller.snapshot.state === "CONNECTED" && adapterReady ? (
      <StatusBadge tone="positive">{t("Adapter ready")}</StatusBadge>
    ) : controller.snapshot.state === "CONNECTED" ? (
      <StatusBadge tone="pending">{t("Board unverified")}</StatusBadge>
    ) : controller.snapshot.state === "ADAPTER_DETECTED" ? (
      <StatusBadge tone="pending">{t("Adapter found")}</StatusBadge>
    ) : controller.snapshot.state === "CONNECTING" ? (
      // Without this the badge fell through to the grey "no adapter" while a
      // connection was being made, so nothing at the top changed when the
      // person acted (the owner, 2026-09-18).
      <StatusBadge tone="pending">{t("Connecting…")}</StatusBadge>
    ) : controller.snapshot.state === "ERROR" ? (
      <StatusBadge tone="negative">{t("Adapter error")}</StatusBadge>
    ) : (
      <StatusBadge tone="neutral">{t("No adapter")}</StatusBadge>
    );
  /*
   * The badges name the thing and let the dot carry the state. They used to
   * carry whole sentences, and a sentence is a different length in every
   * language: switching to Russian moved the left edge of this row by 205
   * points, 141 of them from the library badge alone (measured 2026-09-18).
   * What the sentence said is said in full by the panel underneath, which is
   * where a person goes for the detail anyway.
   */
  const libraryPill =
    library.library.state === "LOADED" || library.library.state === "PARTIALLY_LOADED" ? (
      <StatusBadge tone="positive">{t("Library")}</StatusBadge>
    ) : library.library.state === "FAILED" ? (
      <StatusBadge tone="negative">{t("Library: failed")}</StatusBadge>
    ) : (
      <StatusBadge tone="neutral">{t("Library: built-in")}</StatusBadge>
    );

  return (
    <LanguageContext.Provider value={language}>
    <div className={bench ? "app-shell app-shell--bench" : "app-shell"} lang={language}>
      <header className="app-header">
        <div className="brand">
          <div className="brand-name">
            <h1 data-lamp={lamp}>ProwlOne</h1>
            <p className="brand-tagline">{t("Multi-platform vehicle diagnostics")}</p>
          </div>
          {demoPreview ? <span className="demo-badge">{t("Simulator UI preview")}</span> : null}
        </div>
        <div className="header-status">
          {/* Everything but the language switch wraps inside this group, so
              the switch keeps its place at the right edge whatever the
              language does to the labels (the owner, 2026-09-18). */}
          <div className="header-controls">
          <BatteryCard
            snapshot={battery.snapshot}
            busy={battery.busy}
            running={battery.running}
            onRead={() => void battery.start()}
            onStop={() => void battery.stop()}
            disabledReason={batteryDisabledReason}
            volts={voltage}
          />
          {adapterPill}
          {libraryPill}
          <button
            className="button button--secondary"
            type="button"
            onClick={() => void beginNewSession()}
          >
            {t("New session")}
          </button>
          {/* The service mode (ADR-0036): one switch for the session, behind
              one consent; while it is on the switch itself is red, and so is
              the lamp - no bands (the owner, 2026-09-18). */}
          <button
            className={`button ${service.on ? "button--service" : "button--secondary"}`}
            type="button"
            aria-pressed={service.on}
            disabled={service.switching}
            onClick={() => {
              if (service.on) void service.turnOff();
              else service.askToTurnOn();
            }}
          >
            {t("Service mode")}
          </button>
          {/* Three marks milled into one plate, and a disc of glass that
              slides to the one in use — the operating system's own list has
              no place on an instrument (2026-09-13). */}
          </div>
          <div
            className="language-switch"
            role="group"
            aria-label={t("Language")}
            data-language={language}
          >
            <span className="language-switch__puck" aria-hidden="true" />
            {languages.map((entry) => (
              <button
                key={entry.id}
                type="button"
                className="language-switch__mark"
                aria-pressed={language === entry.id}
                aria-label={entry.label}
                title={entry.label}
                onClick={() => chooseLanguage(entry.id)}
              >
                {entry.short}
              </button>
            ))}
          </div>
        </div>
      </header>
      {/* The session's precondition, when the last battery read is in
          SDD's low band (ADR-0030, 2026-09-19). One sentence, no block. */}
      <BatteryPrecondition reading={voltage} sddLowVoltageMaxMv={battery.snapshot.sddLowVoltageMaxMv} />
      {service.consentOpen ? (
        <ConfirmDialog
          title={t("Service mode")}
          confirmLabel={t("Turn the service mode on")}
          cancelLabel={t("Cancel")}
          busy={service.switching}
          onConfirm={() => void service.acceptConsent()}
          onCancel={service.declineConsent}
        >
          <p>
            {t(
              "Service mode lets this application change what a module holds: clear its fault codes, run its routines, drive its outputs, change its adaptations.",
            )}
          </p>
          <p>
            {t(
              "Each operation will ask you again, one at a time, and is written into the session report with what it changed. This application undoes nothing by itself.",
            )}
          </p>
          <p>
            {t(
              "The car stands still, the ignition is on and the engine is off unless a procedure says otherwise.",
            )}
          </p>
          <p>
            <strong>{t("What follows is your decision for the car in front of you.")}</strong>
          </p>
        </ConfirmDialog>
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
                      pending={controller.pending}
                      surveying={library.surveying}
                      selectedPort={controller.selectedPort}
                      onSelectPort={controller.setSelectedPort}
                      onDetect={() => void controller.refresh()}
                      onConnect={() => {
                        void connectAdapter().catch((error: unknown) => {
                          controller.reportRefusal(
                            t("The adapter could not be connected."),
                            error instanceof Error ? error.message : String(error),
                          );
                        });
                      }}
                      onConnectBench={(scenario) => {
                        void connectBench(scenario).catch((error: unknown) => {
                          controller.reportRefusal(
                            t("The bench could not be connected."),
                            error instanceof Error ? error.message : String(error),
                          );
                        });
                      }}
                      onDisconnect={() => void controller.disconnect()}
                    />
                    <LibraryPanel
                      snapshot={library.library}
                      directory={library.directory}
                      busy={library.busy}
                      restoring={library.restoring}
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
                      busy={library.surveying}
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
                      mileageRunning={mileage.running}
                      mileageBusy={mileage.busy}
                      onReadMileage={() => void mileage.start()}
                      onStopMileage={() => void mileage.stop()}
                      passportRunning={passport.running}
                      passportBusy={passport.busy}
                      onReadPassports={() => void passport.start()}
                      onStopPassports={() => void passport.stop()}
                      ccfRunning={ccf.running}
                      ccfBusy={ccf.busy}
                      onReadCcf={() => void ccf.start()}
                      onStopCcf={() => void ccf.stop()}
                    />
                    {/* What the check found, in one place: the codes used to
                        live only inside whichever module was clicked, so a
                        check that found two dozen looked like one that found
                        none (the owner, 2026-09-18). */}
                    <FaultSummary
                      results={results}
                      modules={surveyModules}
                      onSelect={selectModule}
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
                      serviceMode={service.on}
                      batteryNotice={batteryNotice}
                      clear={service.clear}
                      clearing={service.clearing}
                      onClearCodes={(ecuFamily) => void service.clearCodes(ecuFamily)}
                    />
                  </div>
                ) : null}
                {section.id === "network" ? (
                  <BatteryPanel
                    snapshot={battery.snapshot}
                    running={battery.running}
                    adapterReady={adapterReady}
                    surveyed={library.survey !== null}
                    bench={bench}
                  />
                ) : null}
                {section.id === "network" ? (
                  <MileagePanel
                    snapshot={mileage.snapshot}
                    running={mileage.running}
                    adapterReady={adapterReady}
                    surveyed={library.survey !== null}
                    bench={bench}
                  />
                ) : null}
                {section.id === "network" ? (
                  <PassportPanel
                    snapshot={passport.snapshot}
                    running={passport.running}
                    adapterReady={adapterReady}
                    surveyed={library.survey !== null}
                    bench={bench}
                  />
                ) : null}
                {section.id === "network" ? (
                  <CcfPanel
                    snapshot={ccf.snapshot}
                    running={ccf.running}
                    adapterReady={adapterReady}
                    surveyed={library.survey !== null}
                    bench={bench}
                  />
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
                    onChoose={liveRead.choose}
                    onClearSet={liveRead.clearSet}
                    onStart={() => void liveRead.start()}
                    onStop={() => void liveRead.stop()}
                    onSaveCsv={() => void liveRead.saveCsv()}
                    saved={liveRead.saved}
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
                    document={session.document}
                    mask={session.mask}
                    onMaskChange={(masked) => void session.setMasked(masked)}
                    onShowDocument={() => void session.showDocument()}
                    onHideDocument={session.hideDocument}
                    onPrint={() => void session.print()}
                    onSaveDocument={() => void session.saveDocument()}
                    savedPath={session.savedPath}
                    onReveal={() => void session.reveal()}
                  />
                ) : null}
              </section>
            );
          })}
        </div>
      </main>
      <footer className="app-footer">
        <div className="app-footer__row">
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
        </div>
        {/* Cut into the foot of the plate, as such a line is on any
            instrument: who made it, under what licence, and whose marks are
            named on it (2026-09-13). */}
        <div className="app-footer__row app-footer__legal">
          <span>{t("© 2026 Oleg-mk · AGPL-3.0 · written with Claude (Anthropic)")}</span>
          <span>{t("macOS and Windows, native on Apple silicon, Intel and x86 — connect it and read; the adapter needs no driver of its own.")}</span>
          <span>
            {t(
              "Independent software. Jaguar, Land Rover, Range Rover and SDD are trademarks of Jaguar Land Rover Limited; this application is not affiliated with or endorsed by it.",
            )}
          </span>
        </div>
      </footer>
    </div>
    </LanguageContext.Provider>
  );
}
