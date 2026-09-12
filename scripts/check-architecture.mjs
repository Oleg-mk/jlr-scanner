import { readFile, readdir } from "node:fs/promises";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = new URL("../", import.meta.url);

const manifestRules = [
  ["crates/core-types/Cargo.toml", ["tauri", "serialport", "tokio", "reqwest", "rusqlite"]],
  ["crates/app-contracts/Cargo.toml", ["tauri", "serialport", "tokio", "reqwest"]],
  ["crates/knowledge/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "tokio",
    "reqwest",
    "rusqlite",
    "mongoose-jlr",
    "transport-api",
    "transport-serial",
    "transport-replay",
    "diagnostic-simulator",
    "diagnostic-environment",
    "diagnostic-execution",
    "diagnostics-core",
    "obd-j1979",
    "isotp",
    "uds",
    "windows",
    "winreg",
  ]],
  ["crates/diagnostic-environment/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "tokio",
    "reqwest",
    "rusqlite",
    "mongoose-jlr",
    "transport-api",
    "transport-serial",
    "transport-replay",
    "diagnostic-simulator",
    "diagnostic-execution",
    "diagnostics-core",
    "obd-j1979",
    "isotp",
    "uds",
    "windows",
    "winreg",
  ]],
  ["crates/obd-j1979/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "mongoose-jlr",
    "knowledge",
    "diagnostic-environment",
    "diagnostic-execution",
    "diagnostics-core",
    "transport-api",
    "transport-replay",
    "transport-serial",
    "diagnostic-simulator",
    "isotp",
    "uds",
    "windows",
  ]],
  ["crates/diagnostic-execution/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "mongoose-jlr",
    "transport-serial",
    "diagnostics-core",
    "uds",
    "windows",
    "winreg",
  ]],
  ["crates/report-intake/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "tokio",
    "reqwest",
    "rusqlite",
    "mongoose-jlr",
    "transport-serial",
    "transport-api",
    "transport-replay",
    "diagnostics-core",
    "diagnostic-execution",
    "uds-execution",
    "uds",
    "isotp",
    "obd-j1979",
    "windows",
  ]],
  ["crates/diagnostic-session/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "tokio",
    "reqwest",
    "rusqlite",
    "mongoose-jlr",
    "transport-serial",
    "diagnostics-core",
    "obd-j1979",
    "windows",
    "winreg",
  ]],
  ["crates/uds-execution/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "mongoose-jlr",
    "transport-serial",
    "diagnostics-core",
    "obd-j1979",
    "windows",
    "winreg",
  ]],
  ["crates/mongoose-jlr/Cargo.toml", ["tauri", "react", "frontend", "serialport", "transport-serial", "windows", "winreg"]],
  ["crates/jlr-profiles/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "mongoose-jlr",
    "transport-api",
    "transport-serial",
    "transport-replay",
    "diagnostic-simulator",
    "diagnostic-execution",
    "diagnostics-core",
    "obd-j1979",
    "isotp",
    "uds",
    "windows",
    "winreg",
  ]],
  ["crates/transport-serial/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "isotp", "knowledge"]],
  ["crates/transport-api/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "isotp", "knowledge"]],
  ["crates/transport-replay/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "isotp", "knowledge", "windows"]],
  ["crates/isotp/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "knowledge", "windows"]],
  ["crates/uds/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "isotp", "knowledge", "windows"]],
  ["crates/diagnostic-simulator/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "knowledge", "windows"]],
  // ADR-0020: the vehicle on the bench answers from the library and never sees the adapter protocol or the shell.
  ["crates/bench-vehicle/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "transport-serial", "serialport", "windows", "winreg"]],
  ["crates/diagnostics-core/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "knowledge", "windows"]],
  ["crates/sdd-ingest/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "tokio",
    "reqwest",
    "rusqlite",
    "mongoose-jlr",
    "transport-api",
    "transport-serial",
    "transport-replay",
    "diagnostic-simulator",
    "diagnostic-environment",
    "diagnostic-execution",
    "diagnostics-core",
    "obd-j1979",
    "jlr-profiles",
    "isotp",
    "uds",
    "windows",
    "winreg",
  ]],
];

async function sourceFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await sourceFiles(path)));
    else if ([".ts", ".tsx", ".js", ".jsx"].includes(extname(entry.name))) files.push(path);
  }
  return files;
}

async function rustFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await rustFiles(path)));
    else if (extname(entry.name) === ".rs") files.push(path);
  }
  return files;
}

const failures = [];

for (const [path, forbidden] of manifestRules) {
  const source = (await readFile(new URL(path, root), "utf8")).toLowerCase();
  for (const dependency of forbidden) {
    if (source.includes(dependency)) {
      failures.push(`${path} contains forbidden dependency: ${dependency}`);
    }
  }
}

const frontendRoot = fileURLToPath(new URL("apps/scanner/frontend/src/", root));
for (const file of await sourceFiles(frontendRoot)) {
  const source = (await readFile(file, "utf8")).toLowerCase();
  for (const forbidden of [
    "crates/",
    "src-tauri",
    ".rs\"",
    "diagnostic-environment",
    "diagnostic-execution",
    "diagnostics-core",
    "knowledge",
    "obd-j1979",
    "isotp",
    "transport-api",
    "uds",
  ]) {
    if (source.includes(forbidden)) failures.push(`${file} imports Rust internals`);
  }
}

// Every file in the adapter crate and in the vehicle on the bench, not a
// list that a new file can fall outside of: bench.rs did, until 2026-09-12.
const transmitRuleFiles = [];
for (const directory of ["crates/mongoose-jlr/src/", "crates/bench-vehicle/src/"]) {
  transmitRuleFiles.push(...(await rustFiles(fileURLToPath(new URL(directory, root)))));
}
for (const path of transmitRuleFiles) {
  const source = await readFile(path, "utf8");
  for (const forbidden of [
    /pub\s+fn\s+send_can_frame\b/,
    /pub\s+fn\s+raw_can_tx\b/,
    /pub\s+fn\s+diagnostic_request\b/,
    /pub\s+fn\s+send_isotp\b/,
    /pub\s+fn\s+send_uds\b/,
    /pub\s+fn\s+send_raw\b/,
    /pub\s+fn\s+send_can\b/,
    /pub\s+fn\s+send_bytes\b/,
    /pub\s+fn\s+send_payload\b/,
    /pub\s+fn\s+execute_arbitrary\b/,
  ]) {
    if (forbidden.test(source)) failures.push(`${path} exposes forbidden vehicle TX API`);
  }
}

const mongooseDeviceSource = await readFile(
  new URL("crates/mongoose-jlr/src/device.rs", root),
  "utf8",
);
if (!/pub\s+fn\s+execute_prepared_calibration_identification\s*\([\s\S]*?transaction:\s*&PreparedDiagnosticTransaction/.test(mongooseDeviceSource)) {
  failures.push("Mongoose live execution does not require PreparedDiagnosticTransaction");
}
for (const forbidden of [/physical_request_id\s*\+\s*8/, /wrapping_add\(8\)/]) {
  if (forbidden.test(mongooseDeviceSource)) {
    failures.push("Mongoose diagnostic path derives response identifiers arithmetically");
  }
}

// ADR-0015: the live UDS path takes only a prepared UDS transaction and never
// derives a response identifier or exposes a session or programming service.
const mongooseUdsSource = await readFile(
  new URL("crates/mongoose-jlr/src/uds_live.rs", root),
  "utf8",
);
if (!/pub\s+fn\s+execute_prepared_uds_read\s*\([\s\S]*?transaction:\s*&PreparedUdsTransaction/.test(mongooseUdsSource)) {
  failures.push("Mongoose live UDS execution does not require PreparedUdsTransaction");
}
for (const forbidden of [
  /physical_request_id\s*\+\s*8/,
  /wrapping_add\(8\)/,
  /diagnostic_session_control\s*\(/,
  /tester_present\s*\(/,
  /\bSecurityAccess\b/,
  /\bRoutineControl\b/,
  /\bWriteDataByIdentifier\b/,
  /\bEcuReset\b/,
]) {
  if (forbidden.test(mongooseUdsSource)) {
    failures.push("Mongoose live UDS path exposes forbidden behaviour");
  }
}

const frontendSources = await Promise.all(
  (await sourceFiles(frontendRoot)).map((file) => readFile(file, "utf8")),
);
const combinedFrontend = frontendSources.join("\n");
for (const forbidden of [
  /send_raw/i,
  /send_can/i,
  /send_bytes/i,
  /send_payload/i,
  /execute_arbitrary/i,
  /name=["']can[_-]?id/i,
  /name=["']service/i,
  /name=["']did/i,
]) {
  if (forbidden.test(combinedFrontend)) {
    failures.push("frontend exposes raw diagnostic command entry");
  }
}

for (const directory of [
  "crates/transport-api/src/",
  "crates/transport-replay/src/",
  "crates/diagnostic-simulator/src/",
  "crates/isotp/src/",
  "crates/uds/src/",
  "crates/diagnostics-core/src/",
  "crates/knowledge/src/",
  "crates/diagnostic-environment/src/",
  "crates/diagnostic-execution/src/",
  "crates/uds-execution/src/",
  "crates/obd-j1979/src/",
]) {
  const absolute = fileURLToPath(new URL(directory, root));
  for (const file of await rustFiles(absolute)) {
    const source = await readFile(file, "utf8");
    if (/x250/i.test(source)) failures.push(`${file} contains vehicle-program hardcoding`);
  }
}

const knowledgeRoot = fileURLToPath(new URL("crates/knowledge/src/", root));
for (const file of await rustFiles(knowledgeRoot)) {
  const source = await readFile(file, "utf8");
  for (const forbidden of [
    /\bCanFrame\b/,
    /\bMongooseJlr\b/,
    /\bcOpenChannel\b/,
    /\bcSetPin\b/,
    /\bsend_can_frame\b/,
    /\bsend_isotp\b/,
    /\bsend_uds\b/,
    /\bSecurityAccess\b/,
    /\bRequestDownload\b/,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${file} contains protocol, transport, or programming logic`);
    }
  }
}

const sddIngestRoot = fileURLToPath(new URL("crates/sdd-ingest/src/", root));
for (const file of await rustFiles(sddIngestRoot)) {
  const source = await readFile(file, "utf8");
  for (const forbidden of [
    /CanFrame/,
    /MongooseJlr/,
    /cOpenChannel/,
    /send_can_frame/,
    /send_isotp/,
    /send_uds/,
    /SecurityAccess/,
    /RequestDownload/,
    /TransferData/,
    /vbf/i,
    /flash/i,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${file} contains protocol, transport, or firmware-programming logic`);
    }
  }
}

const udsSource = await readFile(new URL("crates/uds/src/lib.rs", root), "utf8");
for (const forbidden of [
  /pub\s+fn\s+security/i,
  /pub\s+fn\s+routine/i,
  /pub\s+fn\s+write/i,
  /pub\s+fn\s+request_download/i,
  /pub\s+fn\s+transfer/i,
  /pub\s+fn\s+ecu_reset/i,
]) {
  if (forbidden.test(udsSource)) failures.push("UDS exposes an out-of-scope active/programming service");
}

for (const path of [
  "crates/transport-api/src/can.rs",
  "crates/diagnostics-core/src/lib.rs",
]) {
  const source = await readFile(new URL(path, root), "utf8");
  for (const forbidden of [
    /pub\s+fn\s+send_can_frame\b/,
    /pub\s+fn\s+transmit\b/,
    /pub\s+fn\s+write_frame\b/,
  ]) {
    if (forbidden.test(source)) failures.push(`${path} exposes forbidden CAN TX API`);
  }
}

const executionSource = await readFile(
  new URL("crates/diagnostic-execution/src/lib.rs", root),
  "utf8",
);
for (const forbidden of [
  /pub\s+fn\s+execute_raw\b/,
  /pub\s+fn\s+send_frame\b/,
  /pub\s+fn\s+send_diagnostic\b/,
  /pub\s+fn\s+send_can\b/,
  /pub\s+fn\s+execute_live\b/,
  /\bWriteDataByIdentifier\b/,
  /\bSecurityAccess\b/,
  /\bRoutineControl\b/,
  /\bEcuReset\b/,
  /physical_request_id\s*\+\s*8/,
  /wrapping_add\(8\)/,
]) {
  if (forbidden.test(executionSource)) {
    failures.push("diagnostic-execution exposes forbidden raw/live/non-read-only behavior");
  }
}

// ADR-0012: the UDS bridge carries every guard the J1979 bridge carries, plus
// a check that no session-control or programming constructor appears.
const udsExecutionSource = await readFile(
  new URL("crates/uds-execution/src/lib.rs", root),
  "utf8",
);
for (const forbidden of [
  /pub\s+fn\s+execute_raw\b/,
  /pub\s+fn\s+send_frame\b/,
  /pub\s+fn\s+send_diagnostic\b/,
  /pub\s+fn\s+send_can\b/,
  /pub\s+fn\s+execute_live\b/,
  /\bWriteDataByIdentifier\b/,
  /\bSecurityAccess\b/,
  /\bRoutineControl\b/,
  /\bEcuReset\b/,
  /\bRequestDownload\b/,
  /\bTransferData\b/,
  /diagnostic_session_control\s*\(/,
  /tester_present\s*\(/,
  /physical_request_id\s*\+\s*8/,
  /wrapping_add\(8\)/,
]) {
  if (forbidden.test(udsExecutionSource)) {
    failures.push("uds-execution exposes forbidden raw/live/session/non-read-only behavior");
  }
}

const diagnosticsCoreSource = await readFile(
  new URL("crates/diagnostics-core/src/lib.rs", root),
  "utf8",
);
for (const forbidden of [/KnowledgeStore/, /KnowledgeQuery/, /DiagnosticEnvironment/]) {
  if (forbidden.test(diagnosticsCoreSource)) {
    failures.push("diagnostics-core queries F5/F6 knowledge");
  }
}

const tauriSourceRoot = fileURLToPath(new URL("apps/scanner/src-tauri/src/", root));
const tauriEntries = await readdir(tauriSourceRoot, { withFileTypes: true });
for (const entry of tauriEntries) {
  if (!entry.isFile() || extname(entry.name) !== ".rs") continue;
  const path = join(tauriSourceRoot, entry.name);
  const source = await readFile(path, "utf8");
  for (const forbidden of [
    /MongoosePacket/,
    /FrameDecoder/,
    /open_receive_route/,
    /receive_frame/,
    /set_pin_request/,
    /open_channel_request/,
    /DT_LISTEN_ONLY/,
    /cSetPin/,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${path} contains protocol or CAN logic outside production crates`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Architecture boundaries: OK");
}
