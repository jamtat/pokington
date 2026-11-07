import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { collectArtifacts, packageVersion, targets, verifyAssets, verifyExecutable } from "./release.mjs";

const helper = fileURLToPath(new URL("./release.mjs", import.meta.url));
const setupMusl = fileURLToPath(new URL("./setup-musl.sh", import.meta.url));
const sha = "a".repeat(40);

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "pokington-release-test-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  writeFileSync(join(root, "Cargo.toml"), '[package]\nname = "pokington"\nversion = "0.0.1"\n\n[dependencies]\n');
  writeFileSync(join(root, "Cargo.lock"), 'version = 4\n\n[[package]]\nname = "pokington"\nversion = "0.0.1"\n');
  const input = join(root, "incoming");
  mkdirSync(input);
  for (const target of targets) {
    const name = `pokington-0.0.1-${target}.tar.gz`;
    const content = Buffer.from(`fixture bytes for ${target}`);
    writeFileSync(join(input, name), content);
    const hash = createHash("sha256").update(content).digest("hex");
    writeFileSync(join(input, `${name}.sha256`), `${hash}  ${name}\n`);
  }
  const assets = join(root, "assets");
  return { root, input, assets };
}

test("version tags must match the manifest and generated lockfile", (t) => {
  const { root } = fixture(t);
  assert.equal(packageVersion(root, "v0.0.1"), "0.0.1");
  for (const tag of ["v0.0.2", "0.0.1", "v0.0.1-rc.1", "v0.0.1;echo bad"]) {
    assert.throws(() => packageVersion(root, tag), /Tag must/);
  }
  writeFileSync(join(root, "Cargo.lock"), '[[package]]\nname = "pokington"\nversion = "0.0.2"\n');
  assert.throws(() => packageVersion(root), /versions must match/);
  writeFileSync(join(root, "Cargo.toml"), '[package]\nversion = "00.0.1"\n');
  assert.throws(() => packageVersion(root), /literal X.Y.Z/);
});

test("collection verifies all targets and detects changed or missing assets", (t) => {
  const { input, assets } = fixture(t);
  collectArtifacts(input, assets, "0.0.1");
  assert.equal(verifyAssets(assets, "0.0.1").length, 5);
  const archive = readdirSync(assets).find((name) => name.endsWith(".tar.gz"));
  writeFileSync(join(assets, archive), "corruption");
  assert.throws(() => verifyAssets(assets, "0.0.1"), /checksums/);
  rmSync(join(assets, archive));
  assert.throws(() => verifyAssets(assets, "0.0.1"), /missing assets/);
});

test("collection rejects incorrect sidecars, unknown files and symlinks", (t) => {
  const { input, assets } = fixture(t);
  const archive = readdirSync(input).find((name) => name.endsWith(".tar.gz"));
  const sidecar = join(input, `${archive}.sha256`);
  const original = readFileSync(sidecar);
  writeFileSync(sidecar, `bad hash  ${archive}\n`);
  assert.throws(() => collectArtifacts(input, assets, "0.0.1"), /Checksum mismatch/);
  writeFileSync(sidecar, original);
  writeFileSync(join(input, "unexpected"), "extra");
  assert.throws(() => collectArtifacts(input, assets, "0.0.1"), /Unexpected/);
  rmSync(join(input, "unexpected"));
  rmSync(join(input, archive));
  symlinkSync(sidecar, join(input, archive));
  assert.throws(() => collectArtifacts(input, assets, "0.0.1"), /regular file/);
});

const mockGh = String.raw`#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const args = process.argv.slice(2);
fs.appendFileSync(process.env.MOCK_CALLS, JSON.stringify(args) + "\n");
const statePath = process.env.MOCK_STATE;
let state = fs.existsSync(statePath) ? JSON.parse(fs.readFileSync(statePath, "utf8")) : null;
const option = (name) => args[args.indexOf(name) + 1];
const fail = (message) => { console.error(message); process.exit(1); };
if (args[0] === "api") {
  if (!args.includes("--paginate") || !args.includes("--slurp") || !args.at(-1).endsWith("/releases?per_page=100")) {
    fail("The mock tag endpoint does not return drafts; use paginated list-releases");
  }
  if (process.env.MOCK_API_ERROR) fail("gh: Forbidden (HTTP 403)");
  console.log(JSON.stringify(state ? (process.env.MOCK_DUPLICATE ? [[state], [state]] : [[], [state]]) : [[]]));
} else if (args[0] === "release" && ["create", "upload"].includes(args[1])) {
  if (process.env.MOCK_UPLOAD_ERROR) fail("upload interrupted");
  if (args[1] === "create") {
    state = { draft: args.includes("--draft"), tag_name: args[2], target_commitish: option("--target"),
      body: option("--notes"), assets: [], data: {}, html_url: "https://example.test/draft" };
  }
  const end = args.findIndex((arg, index) => index > 2 && arg.startsWith("--"));
  for (const file of args.slice(3, end)) state.data[path.basename(file)] = fs.readFileSync(file).toString("base64");
  state.assets = Object.keys(state.data).map((name) => ({ name }));
  fs.writeFileSync(statePath, JSON.stringify(state));
} else if (args[0] === "release" && args[1] === "download") {
  for (const [name, bytes] of Object.entries(state.data)) {
    fs.writeFileSync(path.join(option("--dir"), name), Buffer.from(bytes, "base64"));
  }
  if (process.env.MOCK_CORRUPT_DOWNLOAD) fs.writeFileSync(path.join(option("--dir"), "SHA256SUMS"), "corrupt");
} else fail("Unexpected mock command");
`;

function draftFixture(t) {
  const data = fixture(t);
  collectArtifacts(data.input, data.assets, "0.0.1");
  const bin = join(data.root, "bin");
  mkdirSync(bin);
  writeFileSync(join(bin, "gh"), mockGh);
  writeFileSync(join(bin, "git"), '#!/bin/sh\ncase "$1" in\nrev-parse) printf "%s\\n" "$MOCK_SHA" ;;\nls-remote) printf "%s\\trefs/tags/v0.0.1\\n" "${MOCK_REMOTE_SHA:-$MOCK_SHA}" ;;\n*) exit 1 ;;\nesac\n');
  chmodSync(join(bin, "gh"), 0o755);
  chmodSync(join(bin, "git"), 0o755);
  const state = join(data.root, "state.json");
  const calls = join(data.root, "calls.jsonl");
  const env = { ...process.env, PATH: `${bin}:${process.env.PATH}`, GITHUB_REPOSITORY: "test/pokington",
    MOCK_SHA: sha, MOCK_STATE: state, MOCK_CALLS: calls };
  const run = (extra = {}) => spawnSync(process.execPath, [helper, "draft", "v0.0.1", data.assets],
    { cwd: data.root, env: { ...env, ...extra }, encoding: "utf8" });
  const readState = () => JSON.parse(readFileSync(state, "utf8"));
  const readCalls = () => existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n").map(JSON.parse) : [];
  return { ...data, state, run, readState, readCalls };
}

test("draft creation and retry preserve manual notes and remain unpublished", (t) => {
  const data = draftFixture(t);
  const first = data.run();
  assert.equal(first.status, 0, first.stderr);
  const state = data.readState();
  assert.equal(state.draft, true);
  assert.equal(state.assets.length, 5);
  state.body += "\nHand-edited notes.";
  delete state.data.SHA256SUMS;
  state.assets = state.assets.filter((asset) => asset.name !== "SHA256SUMS");
  writeFileSync(data.state, JSON.stringify(state));
  const retry = data.run();
  assert.equal(retry.status, 0, retry.stderr);
  assert.match(data.readState().body, /Hand-edited notes/);
  assert.equal(data.readState().assets.length, 5);
  assert.equal(data.readState().draft, true);
  assert.equal(data.readCalls().filter((args) => args[1] === "create").length, 1);
});

test("published or unrelated drafts are rejected before any mutation", (t) => {
  const data = draftFixture(t);
  for (const [draft, body] of [[false, `<!-- pokington-source: ${sha} -->`], [true, "unrelated notes"]]) {
    writeFileSync(data.state, JSON.stringify({ draft, tag_name: "v0.0.1", target_commitish: sha, body, assets: [] }));
    const result = data.run();
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /published release|exact source/);
  }
  assert.ok(data.readCalls().every((args) => args[0] === "api"));
});

test("remote tag mismatch and API errors never create a release", (t) => {
  const data = draftFixture(t);
  assert.match(data.run({ MOCK_REMOTE_SHA: "b".repeat(40) }).stderr, /Remote tag/);
  assert.match(data.run({ MOCK_API_ERROR: "yes" }).stderr, /Cannot inspect release/);
  assert.ok(data.readCalls().every((args) => args[0] === "api"));
  assert.equal(existsSync(data.state), false);
});

test("duplicate matching releases fail before uploads", (t) => {
  const data = draftFixture(t);
  assert.equal(data.run().status, 0);
  const before = data.readCalls().filter((args) => args[0] === "release").length;
  const result = data.run({ MOCK_DUPLICATE: "yes" });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Multiple releases/);
  assert.equal(data.readCalls().filter((args) => args[0] === "release").length, before);
});

test("failed uploads and corrupt downloaded assets are reported as failure", (t) => {
  const data = draftFixture(t);
  assert.notEqual(data.run({ MOCK_UPLOAD_ERROR: "yes" }).status, 0);
  assert.equal(existsSync(data.state), false);
  const corrupt = data.run({ MOCK_CORRUPT_DOWNLOAD: "yes" });
  assert.notEqual(corrupt.status, 0);
  assert.match(corrupt.stderr, /checksums/);
  assert.equal(data.readState().draft, true);
  assert.equal(data.run().status, 0);
});

test("a cached musl sysroot configures paths without calling apt or sudo", (t) => {
  const { root } = fixture(t);
  const muslRoot = join(root, "musl");
  const specs = join(muslRoot, "usr/lib/x86_64-linux-musl/musl-gcc.specs");
  mkdirSync(dirname(specs), { recursive: true });
  writeFileSync(specs, "-isystem /usr/include/x86_64-linux-musl -L/usr/lib/x86_64-linux-musl\n");
  writeFileSync(join(muslRoot, ".complete"), "");
  const bin = join(root, "bin");
  mkdirSync(bin);
  writeFileSync(join(bin, "uname"), '#!/bin/sh\nprintf "x86_64\\n"\n');
  writeFileSync(join(bin, "sudo"), '#!/bin/sh\necho "sudo must not run" >&2\nexit 99\n');
  for (const name of ["uname", "sudo"]) chmodSync(join(bin, name), 0o755);
  const envFile = join(root, "environment");
  const pathFile = join(root, "path");
  const result = spawnSync("bash", [setupMusl], { cwd: resolve(dirname(helper), "../.."),
    env: { ...process.env, MUSL_ROOT: muslRoot, GITHUB_ENV: envFile, GITHUB_PATH: pathFile, PATH: `${bin}:${process.env.PATH}` }, encoding: "utf8" });
  assert.equal(result.status, 0, result.stderr);
  assert.match(readFileSync(envFile, "utf8"), /CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=/);
  assert.equal(readFileSync(pathFile, "utf8"), `${muslRoot}/bin\n`);
  assert.ok(readFileSync(join(muslRoot, "musl-gcc.specs"), "utf8").includes(`${muslRoot}/usr/include/`));
});

test("executable checks reject wrong versions, architectures and dynamic Linux linkage", (t) => {
  const { root } = fixture(t);
  const bin = join(root, "bin");
  mkdirSync(bin);
  const executable = join(bin, "pokington");
  writeFileSync(executable, '#!/bin/sh\ncase "$1" in\n--version) cat "$ELF_FIXTURES/version" ;;\n--help) echo "Usage: pokington" ;;\nesac\n');
  writeFileSync(join(bin, "readelf"), '#!/bin/sh\ncat "$ELF_FIXTURES/$1"\n');
  for (const name of ["pokington", "readelf"]) chmodSync(join(bin, name), 0o755);
  const previousPath = process.env.PATH;
  const previousFixtures = process.env.ELF_FIXTURES;
  process.env.PATH = `${bin}:${previousPath}`;
  process.env.ELF_FIXTURES = root;
  t.after(() => {
    process.env.PATH = previousPath;
    if (previousFixtures === undefined) delete process.env.ELF_FIXTURES;
    else process.env.ELF_FIXTURES = previousFixtures;
  });
  const check = () => verifyExecutable(executable, "x86_64-unknown-linux-musl", "0.0.1");
  writeFileSync(join(root, "version"), "pokington 0.0.2\n");
  assert.throws(check, /version mismatch/);
  writeFileSync(join(root, "version"), "pokington 0.0.1\n");
  writeFileSync(join(root, "-h"), "Machine: AArch64\n");
  assert.throws(check, /architecture mismatch/);
  writeFileSync(join(root, "-h"), "Machine: Advanced Micro Devices X86-64\n");
  writeFileSync(join(root, "-l"), "  INTERP 0x000000\n");
  assert.throws(check, /dynamic interpreter/);
  writeFileSync(join(root, "-l"), "  LOAD 0x000000\n");
  writeFileSync(join(root, "-d"), "(NEEDED) Shared library: [libc.so.6]\n");
  assert.throws(check, /dynamic dependencies/);
  writeFileSync(join(root, "-d"), "There is no dynamic section in this file.\n");
  assert.doesNotThrow(check);
});

