#!/usr/bin/env node
// Set one version number in every file that carries it, then say what to
// commit and tag. The number is the tester-facing build: the last digit is
// the one testers quote ("build 1", "build 2"); it grows only when a build
// is handed out. See docs/OWNER_GUIDE.uk.md, section 5.
//
//   node scripts/bump-version.mjs 0.9.2
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const version = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(version ?? "")) {
  console.error("usage: node scripts/bump-version.mjs <major.minor.patch>");
  process.exit(2);
}

/** Replace exactly one match, or refuse: a file that changed shape must not be half-edited. */
function edit(file, pattern, replacement) {
  const path = join(root, file);
  const text = readFileSync(path, "utf8");
  const matches = text.match(new RegExp(pattern.source, pattern.flags + "g")) ?? [];
  if (matches.length !== 1) {
    console.error(`${file}: expected one version field, found ${matches.length}`);
    process.exit(1);
  }
  writeFileSync(path, text.replace(pattern, replacement));
  console.log(`${file}: ${matches[0].trim()} -> ${version}`);
}

edit("Cargo.toml", /(\[workspace\.package\]\r?\nversion = ")[^"]+(")/, `$1${version}$2`);
edit("package.json", /("version": ")[^"]+(")/, `$1${version}$2`);
edit("apps/scanner/frontend/package.json", /("version": ")[^"]+(")/, `$1${version}$2`);
edit("apps/scanner/src-tauri/tauri.conf.json", /("version": ")[^"]+(")/, `$1${version}$2`);

// Cargo.lock: every workspace member (a package block without a `source`
// line) carries the workspace version, so the lock file follows the bump
// and CI does not have to rewrite it.
{
  const path = join(root, "Cargo.lock");
  const blocks = readFileSync(path, "utf8").split(/(?=\[\[package\]\])/);
  let changed = 0;
  const next = blocks.map((block) => {
    if (!block.startsWith("[[package]]") || /^source = /m.test(block)) return block;
    const edited = block.replace(/^version = "[^"]+"/m, (line) => {
      if (line !== `version = "${version}"`) changed += 1;
      return `version = "${version}"`;
    });
    return edited;
  });
  writeFileSync(path, next.join(""));
  console.log(`Cargo.lock: ${changed} workspace package(s) set to ${version}`);
}

console.log(`
next:
  git commit -am "release: ${version}"
  git tag v${version}
  git push origin main --tags
`);
