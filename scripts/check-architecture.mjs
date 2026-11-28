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
    "ds2",
    "kwp2000",
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
    "ds2",
    "kwp2000",
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
    "ds2",
    "kwp2000",
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
    "ds2",
    "kwp2000",
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
    "ds2",
    "kwp2000",
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
  // ADR-0029 slice B: the K-line execution crate compiles a plan into bytes
  // and reads bytes back. No transport, no adapter, no UDS, no shell.
  ["crates/kline-execution/Cargo.toml", [
    "tauri",
    "react",
    "frontend",
    "serialport",
    "mongoose-jlr",
    "transport-api",
    "transport-serial",
    "transport-replay",
    "diagnostics-core",
    "obd-j1979",
    "isotp",
    "uds",
    "uds-execution",
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
    "ds2",
    "kwp2000",
    "windows",
    "winreg",
  ]],
  ["crates/transport-serial/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "ds2", "kwp2000", "isotp", "knowledge"]],
  ["crates/transport-api/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "ds2", "kwp2000", "isotp", "knowledge"]],
  ["crates/transport-replay/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "ds2", "kwp2000", "isotp", "knowledge", "windows"]],
  ["crates/isotp/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "uds", "ds2", "kwp2000", "knowledge", "windows"]],
  ["crates/uds/Cargo.toml", ["tauri", "react", "frontend", "mongoose-jlr", "diagnostics-core", "isotp", "knowledge", "windows"]],
  // ADR-0029: the K-line protocol crates know a frame and nothing else — no
  // transport, no knowledge, no adapter, no other protocol.
  ["crates/ds2/Cargo.toml", ["tauri", "react", "frontend", "serialport", "tokio", "reqwest", "rusqlite", "mongoose-jlr", "transport-api", "transport-serial", "transport-replay", "diagnostic-simulator", "diagnostic-environment", "diagnostic-execution", "diagnostics-core", "knowledge", "sdd-ingest", "obd-j1979", "isotp", "uds", "kwp2000", "windows", "winreg"]],
  ["crates/kwp2000/Cargo.toml", ["tauri", "react", "frontend", "serialport", "tokio", "reqwest", "rusqlite", "mongoose-jlr", "transport-api", "transport-serial", "transport-replay", "diagnostic-simulator", "diagnostic-environment", "diagnostic-execution", "diagnostics-core", "knowledge", "sdd-ingest", "obd-j1979", "isotp", "uds", "ds2", "windows", "winreg"]],
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
    "ds2",
    "kwp2000",
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

// ADR-0043, decision 9: the filter table's one command is the pass entry.
// `cTableAddEntry` is built in one function of passive.rs, with its subtype,
// rule, mask and pattern fixed; nothing in the adapter crate or the bench
// names the table's other commands, its BLOCK rule or its periodic-message
// type, which would be a transmit the firmware repeats on its own. Test code
// (after `#[cfg(test)]`) may write malformed entries to the bench.
const productionPart = (source) => source.split("#[cfg(test)]")[0];
let tableEntryBuilders = 0;
for (const path of transmitRuleFiles) {
  const source = productionPart(await readFile(path, "utf8"))
    .split(String.fromCharCode(10))
    .filter((line) => !line.trim().startsWith("//"))
    .join(String.fromCharCode(10));
  const builders = source.match(/command\([^;]*?\bTABLE_ADD_ENTRY\b(?!_)/g) ?? [];
  if (builders.length > 0 && !path.replace(/\\/g, "/").endsWith("crates/mongoose-jlr/src/passive.rs")) {
    failures.push(`${path} builds a cTableAddEntry outside passive.rs (ADR-0043)`);
  }
  tableEntryBuilders += builders.length;
  for (const forbidden of [
    /TABLE_(REMOVE|MODIFY|CLEAR)/,
    /cTable(Remove|Modify|Clear)/,
    /PERIODIC_(TABLE|ENTRY|MESSAGE|SUBTYPE)/,
    /fn\s+\w*periodic\w*\s*\(/i,
    /BLOCK_(RULE|TABLE|SUBTYPE|ENTRY)/,
    /fn\s+\w*block\w*_entry\w*\s*\(/i,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${path} names a filter-table command, rule or type outside ADR-0043`);
    }
  }
}
if (tableEntryBuilders !== 1) {
  failures.push(`cTableAddEntry must be built exactly once, in passive.rs (found ${tableEntryBuilders}; ADR-0043)`);
}
const passiveSource = await readFile(new URL("crates/mongoose-jlr/src/passive.rs", root), "utf8");
for (const required of [
  /const PASS_TABLE_SUBTYPE: u32 = 0;/,
  /const PASS_RULE: u8 = 1;/,
  /const PASS_EVERYTHING_MASK: \[u8; 4\] = \[0; 4\];/,
  /const PASS_EVERYTHING_PATTERN: \[u8; 4\] = \[0; 4\];/,
  /pub\(crate\) fn pass_everything_entry_request\(\s*sequence: u16,\s*channel_id: u16,\s*format: CanIdFormat,?\s*\) -> Vec<u8>/,
]) {
  if (!required.test(passiveSource)) {
    failures.push(`the pass entry is not the fixed record of ADR-0043 (${required})`);
  }
}

// ADR-0030, amended 2026-11-15, decision 3: the one value command is the
// read of the battery voltage at the connector. `cGetValue` is built in one
// function of passive.rs with the value id a constant 3; nothing in the
// adapter crate or the bench names `cSetValue` (0x000B) or the programming
// voltage, and no other value id is built.
let valueReadBuilders = 0;
for (const path of transmitRuleFiles) {
  const source = productionPart(await readFile(path, "utf8"))
    .split(String.fromCharCode(10))
    .filter((line) => !line.trim().startsWith("//"))
    .join(String.fromCharCode(10));
  const builders = source.match(/\bGET_VALUE\b(?!_)\s*,/g) ?? [];
  if (builders.length > 0 && !path.replace(/\\/g, "/").endsWith("crates/mongoose-jlr/src/passive.rs")) {
    failures.push(`${path} builds a cGetValue outside passive.rs (ADR-0030)`);
  }
  valueReadBuilders += builders.length;
  for (const forbidden of [
    /SET_VALUE/,
    /cSetValue/,
    /\b0x000[bB]\b/,
    /PROG(RAMMING)?_VOLTAGE/i,
    /VALUE_ID\s*:\s*u32\s*=\s*(?!\s|3;)/,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${path} names a value command or value outside ADR-0030 (${forbidden})`);
    }
  }
}
if (valueReadBuilders !== 1) {
  failures.push(`cGetValue must be built exactly once, in passive.rs (found ${valueReadBuilders}; ADR-0030)`);
}
for (const required of [
  /const CONNECTOR_VOLTAGE_VALUE_ID: u32 = 3;/,
  /pub\(crate\) fn connector_voltage_request\(sequence: u16\) -> Vec<u8>/,
]) {
  if (!required.test(passiveSource)) {
    failures.push(`the value read is not the fixed record of ADR-0030 (${required})`);
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
// ADR-0036 (stage 2, step 1): the one service operation reaches the adapter
// only as a prepared service, through its own function, and that function is
// the only place a diagnostic session is opened. The read function's body
// names no session control; the crate names no service of a later step.
if (!/pub\s+fn\s+execute_prepared_uds_service\s*\([\s\S]*?service:\s*&PreparedUdsService/.test(mongooseUdsSource)) {
  failures.push("Mongoose live UDS service execution does not require PreparedUdsService");
}
const udsReadStart = mongooseUdsSource.indexOf("pub fn execute_prepared_uds_read(");
const udsServiceStart = mongooseUdsSource.indexOf("pub fn execute_prepared_uds_service(");
const udsReadBody =
  udsReadStart >= 0 && udsServiceStart > udsReadStart
    ? mongooseUdsSource.slice(udsReadStart, udsServiceStart)
    : "";
if (udsReadBody === "" || /diagnostic_session_control\s*\(|tester_present\s*\(/.test(udsReadBody)) {
  failures.push("Mongoose live UDS read opens a diagnostic session, or the read and the service are not laid out as ADR-0036 says");
}
for (const forbidden of [
  /physical_request_id\s*\+\s*8/,
  /wrapping_add\(8\)/,
  /\bSecurityAccess\b/,
  /\bRoutineControl\b/,
  /\bWriteDataByIdentifier\b/,
  /\bInputOutputControl\w*\b/,
  /\bEcuReset\b/,
  /\bRequestDownload\b/,
  /\bTransferData\b/,
]) {
  if (forbidden.test(mongooseUdsSource)) {
    failures.push("Mongoose live UDS path exposes forbidden behaviour");
  }
}

// ADR-0029 slice B: the live K-line path takes only a prepared K-line
// transaction, speaks no service beyond the four reads, and never writes to
// the adapter — 0x0014 is that firmware's SetData, and nothing sends it.
const mongooseKlineSource = await readFile(
  new URL("crates/mongoose-jlr/src/kline_live.rs", root),
  "utf8",
);
if (!/pub\s+fn\s+execute_prepared_kline_read\s*\([\s\S]*?transaction:\s*&PreparedKlineTransaction/.test(mongooseKlineSource)) {
  failures.push("Mongoose live K-line execution does not require PreparedKlineTransaction");
}
// ADR-0036: the K-line clear reaches the adapter only as a prepared service.
if (!/pub\s+fn\s+execute_prepared_kline_service\s*\([\s\S]*?service:\s*&PreparedKlineService/.test(mongooseKlineSource)) {
  failures.push("Mongoose live K-line service execution does not require PreparedKlineService");
}
// Comments may name the write command in order to forbid it; code may not.
const withoutComments = (source) =>
  source
    .split(String.fromCharCode(10))
    .filter((line) => !line.trim().startsWith("//"))
    .join(" ");
const klineSources = [
  withoutComments(mongooseKlineSource),
  withoutComments(await readFile(new URL("crates/mongoose-jlr/src/kline.rs", root), "utf8")),
];
for (const source of klineSources) {
  for (const forbidden of [
    /0x0014/,
    /SetData/,
    /\bsecurity_access/i,
    /start_diagnostic_session/i,
    /routine_control/i,
    /write_memory/i,
  ]) {
    if (forbidden.test(source)) {
      failures.push("Mongoose K-line path names a write or a service outside ADR-0029 and ADR-0036");
    }
  }
}

const klineExecutionSource = await readFile(
  new URL("crates/kline-execution/src/lib.rs", root),
  "utf8",
);
for (const forbidden of [
  /pub\s+fn\s+execute_raw\b/,
  /pub\s+fn\s+send_\w*\(/,
  /pub\s+fn\s+transmit\w*\(/,
  /\bsecurity_access\b/i,
  /\bstart_diagnostic_session\b/i,
  /\broutine_control\b/i,
  /\bwrite_memory\b/i,
]) {
  if (forbidden.test(klineExecutionSource)) {
    failures.push("kline-execution exposes forbidden raw or non-read-only behaviour");
  }
}
// ADR-0036 step 1: the service intents are the two clears and no third.
const serviceEnumStart = klineExecutionSource.indexOf("pub enum ServiceKlineIntent {");
const serviceEnumEnd =
  serviceEnumStart >= 0
    ? klineExecutionSource.indexOf(String.fromCharCode(10) + "}", serviceEnumStart)
    : -1;
const klineServiceBlock =
  serviceEnumStart >= 0 && serviceEnumEnd > serviceEnumStart
    ? klineExecutionSource.slice(serviceEnumStart, serviceEnumEnd)
    : "";
const klineServices = (klineServiceBlock.match(/^\s{4}(Ds2|Kwp)\w+\s*\{/gm) ?? []).length;
if (klineServices !== 2) {
  failures.push(`kline-execution declares ${klineServices} service intents, ADR-0036 step 1 names 2`);
}
// The four reads of ADR-0029 and no fifth.
const enumStart = klineExecutionSource.indexOf("pub enum ReadOnlyKlineIntent {");
const enumEnd =
  enumStart >= 0
    ? klineExecutionSource.indexOf(String.fromCharCode(10) + "}", enumStart)
    : -1;
const klineIntentBlock =
  enumStart >= 0 && enumEnd > enumStart
    ? klineExecutionSource.slice(enumStart, enumEnd)
    : "";
const klineIntents = (klineIntentBlock.match(/^\s{4}(Ds2|Kwp)\w+\s*\{/gm) ?? []).length;
if (klineIntents !== 4) {
  failures.push(`kline-execution declares ${klineIntents} read intents, expected 4`);
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
  "crates/ds2/src/",
  "crates/kwp2000/src/",
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

// Knowledge never carries protocol, transport or programming code. The
// patterns test the code, not the prose: a comment saying that VBF names
// or flash lists are not read is the rule kept, not broken. Until
// 2026-11-15 every pattern here was wrapped in a backspace character
// instead of a word boundary (a tool's escaping, since the first public
// snapshot), so the rule matched nothing and checked nothing.
const sddIngestRoot = fileURLToPath(new URL("crates/sdd-ingest/src/", root));
for (const file of await rustFiles(sddIngestRoot)) {
  const source = (await readFile(file, "utf8"))
    .split(String.fromCharCode(10))
    .filter((line) => !line.trim().startsWith("//"))
    .join(String.fromCharCode(10));
  for (const forbidden of [
    /\bCanFrame\b/,
    /\bMongooseJlr\b/,
    /\bcOpenChannel\b/,
    /\bsend_can_frame\b/,
    /\bsend_isotp\b/,
    /\bsend_uds\b/,
    /\bSecurityAccess\b/,
    /\bRequestDownload\b/,
    /\bTransferData\b/,
    /\bvbf\b/i,
    /\bflash\b/i,
  ]) {
    if (forbidden.test(source)) {
      failures.push(`${file} contains protocol, transport, or firmware-programming logic`);
    }
  }
}

const udsSource = await readFile(new URL("crates/uds/src/lib.rs", root), "utf8");
for (const forbidden of [
  /pub\s+fn\s+security/i,
  /pub\s+fn\s+write/i,
  /pub\s+fn\s+request_download/i,
  /pub\s+fn\s+transfer/i,
  /pub\s+fn\s+ecu_reset/i,
]) {
  if (forbidden.test(udsSource)) failures.push("UDS exposes an out-of-scope active/programming service");
}
// ADR-0036 step 2: the one routine constructor - the sub-function and the
// routine, no option record - and no other routine constructor.
const udsRoutineConstructors = (udsSource.match(/^\s*pub\s+fn\s+routine\w*\s*\(/gm) ?? []).length;
if (
  udsRoutineConstructors !== 1 ||
  !/^\s*pub\s+fn\s+routine_control\(sub_function: u8, routine: u16\) -> Self/m.test(udsSource)
) {
  failures.push("UDS must have exactly the one routine constructor ADR-0036 step 2 names: routine_control(sub_function, routine)");
}

// ADR-0029 decision 7, amended by ADR-0036 step 1: the K-line protocol
// crates speak their read-only services and, since stage 2, the one clear
// each - DS2 clear-fault (0x05), KWP2000 ClearDiagnosticInformation (0x14) -
// and nothing else that writes. No public frame encoder; KWP2000 has no
// session control beyond the link handshake, no tester-present, no security
// access, no routine, no write, no download or upload.
const ds2Source = await readFile(new URL("crates/ds2/src/lib.rs", root), "utf8");
for (const forbidden of [
  /^\s*pub\s+fn\s+(?!clear_fault_memory\()(clear|erase|reset|write|program|code|secur|session|frame|encode|raw|send|transmit)\w*\(/im,
  /COMMAND_(?!CLEAR_FAULT_MEMORY\b)(CLEAR|ERASE|RESET|WRITE|CODE|PROGRAM)/,
]) {
  if (forbidden.test(ds2Source)) failures.push("DS2 exposes a service or an encoder outside ADR-0029 and ADR-0036");
}
const kwpSource = await readFile(new URL("crates/kwp2000/src/lib.rs", root), "utf8");
for (const forbidden of [
  /^\s*pub\s+fn\s+(?!clear_diagnostic_information\()(clear|erase|reset|write|program|secur|routine|session|tester_present|stop_communication|input_output|download|upload|transfer|memory|frame|encode|raw|send|transmit)\w*\(/im,
  /SID_(?!CLEAR_DIAGNOSTIC_INFORMATION\b)(CLEAR|SECURITY|WRITE|ROUTINE|ECU_RESET|START_DIAGNOSTIC_SESSION|TESTER_PRESENT|INPUT_OUTPUT|REQUEST_DOWNLOAD|REQUEST_UPLOAD|TRANSFER_DATA|STOP_COMMUNICATION|ACCESS_TIMING)/,
  /\bSecurityAccess\b/,
  /\bWriteDataBy/,
  /\bStartDiagnosticSession\b/,
]) {
  if (forbidden.test(kwpSource)) failures.push("KWP2000 exposes a service or an encoder outside ADR-0029 and ADR-0036");
}
// The K-line request types must have exactly the constructors ADR-0029 and
// ADR-0036 name: three for DS2, four for KWP2000.
const ds2Constructors = (ds2Source.match(/^\s{4}pub fn \w+\(node_address: u8/gm) ?? []).length;
if (ds2Constructors !== 3) failures.push(`DS2 has ${ds2Constructors} request constructors, ADR-0029 and ADR-0036 name 3`);
const kwpConstructors = (kwpSource.match(/^\s{4}pub fn \w+\(target: u8/gm) ?? []).length;
if (kwpConstructors !== 4) failures.push(`KWP2000 has ${kwpConstructors} request constructors, ADR-0029 and ADR-0036 name 4`);

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
// ADR-0036 step 1: one service intent, the clear, prepared as a service the
// read path cannot take.
if (!/pub\s+struct\s+PreparedUdsService\b/.test(udsExecutionSource)) {
  failures.push("uds-execution does not prepare a service as its own type (ADR-0036)");
}
const udsServiceEnumStart = udsExecutionSource.indexOf("pub enum ServiceUdsIntent {");
const udsServiceEnumEnd =
  udsServiceEnumStart >= 0
    ? udsExecutionSource.indexOf(String.fromCharCode(10) + "}", udsServiceEnumStart)
    : -1;
const udsServiceBlock =
  udsServiceEnumStart >= 0 && udsServiceEnumEnd > udsServiceEnumStart
    ? udsExecutionSource.slice(udsServiceEnumStart, udsServiceEnumEnd)
    : "";
const udsServices = (udsServiceBlock.match(/^\s{4}[A-Z]\w+\s*\{/gm) ?? []).length;
if (udsServices !== 2) {
  failures.push(`uds-execution declares ${udsServices} service intents, ADR-0036 step 2 names 2`);
}
// ADR-0036 step 2, widened on 2026-11-28: the routines this product sends
// are a closed set - the reviewed table, one line per entry of the ADR's
// table (the self test and twenty-four resets) - and the closed routine
// type is built from a line of that table and a declaration's name, by
// its one constructor, never from a number the interface chose.
const STAGE_2_ROUTINE_LINES = 25;
if (!/pub const SELF_TEST_ROUTINE: u16 = 0x0202;/.test(udsExecutionSource)) {
  failures.push("uds-execution must name the self test, 0x0202, as ADR-0036 step 2 does");
}
const routineTableStart = udsExecutionSource.indexOf("pub const STAGE_2_ROUTINES: &[ReviewedRoutine] = &[");
const routineTableEnd =
  routineTableStart >= 0 ? udsExecutionSource.indexOf(String.fromCharCode(10) + "];", routineTableStart) : -1;
const routineTable =
  routineTableStart >= 0 && routineTableEnd > routineTableStart
    ? udsExecutionSource.slice(routineTableStart, routineTableEnd)
    : "";
const routineLines = (routineTable.match(/ReviewedRoutine \{/g) ?? []).length;
if (routineLines !== STAGE_2_ROUTINE_LINES) {
  failures.push(
    `uds-execution's reviewed table holds ${routineLines} routines, ADR-0036 (2026-11-28) names ${STAGE_2_ROUTINE_LINES}`,
  );
}
const routineStructStart = udsExecutionSource.indexOf("pub struct StageTwoRoutine {");
const routineStructEnd =
  routineStructStart >= 0 ? udsExecutionSource.indexOf(String.fromCharCode(10) + "}", routineStructStart) : -1;
const routineStruct =
  routineStructStart >= 0 && routineStructEnd > routineStructStart
    ? udsExecutionSource.slice(routineStructStart, routineStructEnd)
    : "";
if (routineStruct === "" || /^\s{4}pub /m.test(routineStruct)) {
  failures.push("uds-execution's closed routine type must keep its fields private: it is built from the reviewed table alone");
}
const reviewedConstructors = (
  udsExecutionSource.match(/^\s{4}pub fn reviewed\(identifier: u16, declared_name: &str\) -> Option<Self>/gm) ?? []
).length;
if (reviewedConstructors !== 1) {
  failures.push("uds-execution's closed routine type must have the one constructor ADR-0036 (2026-11-28) names: reviewed(identifier, declared_name)");
}
if (!/routine: StageTwoRoutine,/.test(udsServiceBlock)) {
  failures.push("uds-execution's routine intent must take the closed routine type, not a number");
}
// The shell may look a declaration up in the table and may not build the
// type any other way: no struct literal of it anywhere outside uds-execution.
for (const [name, path] of [
  ["prowlone-shell", "apps/scanner/src-tauri/src/routine_run_service.rs"],
  ["prowlone-shell", "apps/scanner/src-tauri/src/lib.rs"],
  ["diagnostic-session", "crates/diagnostic-session/src/lib.rs"],
]) {
  const source = await readFile(new URL(path, root), "utf8");
  if (/StageTwoRoutine\s*\{/.test(source)) {
    failures.push(`${name} builds the closed routine type by hand; only StageTwoRoutine::reviewed may`);
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

// The public tree says that the library is derived from SDD's own data and
// never how that data was obtained: no local corpus folder names, no account
// of the owner's own tooling (2026-10-12, when both were found in the docs).
async function documentFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];
  for (const entry of entries) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) files.push(...(await documentFiles(path)));
    else if (extname(entry.name) === ".md") files.push(path);
  }
  return files;
}
const publicDocuments = [
  ...(await documentFiles(fileURLToPath(new URL("docs/", root)))),
  fileURLToPath(new URL("README.md", root)),
  fileURLToPath(new URL("CLAUDE.md", root)),
];
for (const path of publicDocuments) {
  const text = await readFile(path, "utf8");
  for (const forbidden of [
    /SDD\d*_(DECRYPTED|EXTRACTED)/i,
    /owner'?s (own )?scripts|his own scripts/i,
    /сво(їми|ими) скриптами|твої скрипти/i,
  ]) {
    if (forbidden.test(text)) {
      failures.push(`${path} says how the SDD corpus was obtained (${forbidden})`);
    }
  }
}

if (failures.length > 0) {
  console.error(failures.join("\n"));
  process.exitCode = 1;
} else {
  console.log("Architecture boundaries: OK");
}
