import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync, copyFileSync, lstatSync, mkdirSync, mkdtempSync,
  readFileSync, readdirSync, rmSync, writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const targets = Object.freeze([
  "aarch64-apple-darwin", "aarch64-unknown-linux-musl",
  "x86_64-apple-darwin", "x86_64-unknown-linux-musl",
]);
const versionPattern = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

function requireValue(condition, message) {
  if (!condition) throw new Error(message);
}

function command(program, args) {
  const result = spawnSync(program, args, { encoding: "utf8" });
  requireValue(!result.error && result.status === 0,
    `${program} ${args.join(" ")} failed: ${result.error?.message ?? result.stderr}`);
  return result.stdout.trim();
}

export function packageVersion(root = process.cwd(), tag) {
  const manifest = readFileSync(join(root, "Cargo.toml"), "utf8");
  const section = manifest.match(/^\[package\]\s*\n([\s\S]*?)(?=^\[|$(?![\s\S]))/m)?.[1];
  const version = section?.match(/^version\s*=\s*"([^"]+)"\s*(?:#.*)?$/m)?.[1];
  requireValue(version && versionPattern.test(version), "Expected a literal X.Y.Z package version");
  const packages = readFileSync(join(root, "Cargo.lock"), "utf8").split(/^\[\[package\]\]\s*$/m);
  const own = packages.filter((entry) => /^name = "pokington"\s*$/m.test(entry));
  requireValue(own.length === 1 && own[0].match(/^version = "([^"]+)"\s*$/m)?.[1] === version,
    "Cargo.lock and Cargo.toml package versions must match");
  if (tag !== undefined) requireValue(tag === `v${version}`, `Tag must be v${version}`);
  return version;
}

const archiveName = (version, target) => `pokington-${version}-${target}.tar.gz`;
const checksumLine = (path) => `${createHash("sha256").update(readFileSync(path)).digest("hex")}  ${basename(path)}\n`;
const expectedArchives = (version) => targets.map((target) => archiveName(version, target));

function exactFiles(directory, expected) {
  requireValue(JSON.stringify(readdirSync(directory).sort()) === JSON.stringify([...expected].sort()),
    `Unexpected or missing assets in ${directory}`);
  for (const name of expected) requireValue(lstatSync(join(directory, name)).isFile(), `Not a regular file: ${name}`);
}

export function verifyExecutable(executable, target, version) {
  requireValue(command(executable, ["--version"]) === `pokington ${version}`, "Packaged executable version mismatch");
  requireValue(command(executable, ["--help"]).includes("Usage: pokington"), "Packaged executable help failed");
  if (target.endsWith("linux-musl")) {
    const header = command("readelf", ["-h", executable]);
    const machine = target.startsWith("aarch64") ? "AArch64" : "Advanced Micro Devices X86-64";
    requireValue(header.includes(machine), "Packaged ELF architecture mismatch");
    requireValue(!/\bINTERP\b/.test(command("readelf", ["-l", executable])), "Linux binary has a dynamic interpreter");
    requireValue(!/\(NEEDED\)/.test(command("readelf", ["-d", executable])), "Linux binary has dynamic dependencies");
  } else {
    const architecture = target.startsWith("aarch64") ? "arm64" : "x86_64";
    requireValue(command("lipo", ["-archs", executable]) === architecture, "Packaged Mach-O architecture mismatch");
  }
}

export function packageRelease(target, output) {
  requireValue(targets.includes(target), `Unsupported target: ${target}`);
  const version = packageVersion();
  const binary = resolve("target", target, "release", "pokington");
  const nativeTarget = `${process.arch === "arm64" ? "aarch64" : "x86_64"}-${process.platform === "darwin" ? "apple-darwin" : "unknown-linux-musl"}`;
  requireValue(target === nativeTarget, `Execute package checks on the native target (${nativeTarget})`);
  const temporary = mkdtempSync(join(tmpdir(), "pokington-package-"));
  try {
    const stem = `pokington-${version}-${target}`;
    const directory = join(temporary, stem);
    mkdirSync(directory);
    copyFileSync(binary, join(directory, "pokington"));
    chmodSync(join(directory, "pokington"), 0o755);
    copyFileSync("README.md", join(directory, "README.md"));
    mkdirSync(output, { recursive: true });
    const archive = resolve(output, `${stem}.tar.gz`);
    command("tar", ["-czf", archive, "-C", temporary, stem]);
    const extracted = join(temporary, "extracted");
    mkdirSync(extracted);
    command("tar", ["-xzf", archive, "-C", extracted]);
    const executable = join(extracted, stem, "pokington");
    verifyExecutable(executable, target, version);
    writeFileSync(`${archive}.sha256`, checksumLine(archive));
    console.log(`Verified ${archive}`);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

export function collectArtifacts(input, output, version = packageVersion()) {
  const archives = expectedArchives(version);
  exactFiles(input, archives.flatMap((name) => [name, `${name}.sha256`]));
  for (const name of archives) {
    requireValue(readFileSync(join(input, `${name}.sha256`), "utf8") === checksumLine(join(input, name)),
      `Checksum mismatch: ${name}`);
  }
  mkdirSync(output, { recursive: true });
  requireValue(readdirSync(output).length === 0, "Collection output must be empty");
  for (const name of archives) copyFileSync(join(input, name), join(output, name));
  writeFileSync(join(output, "SHA256SUMS"), archives.map((name) => checksumLine(join(output, name))).join(""));
  verifyAssets(output, version);
}

export function verifyAssets(directory, version = packageVersion()) {
  const archives = expectedArchives(version);
  exactFiles(directory, [...archives, "SHA256SUMS"]);
  const expected = archives.map((name) => checksumLine(join(directory, name))).join("");
  requireValue(readFileSync(join(directory, "SHA256SUMS"), "utf8") === expected, "Release checksums do not match archives");
  return [...archives, "SHA256SUMS"];
}

export function prepareDraft(tag, directory) {
  const version = packageVersion(process.cwd(), tag);
  const files = verifyAssets(directory, version);
  const repository = process.env.GITHUB_REPOSITORY;
  requireValue(/^[\w.-]+\/[\w.-]+$/.test(repository ?? ""), "GITHUB_REPOSITORY must identify the release repository");
  const sha = command("git", ["rev-parse", "HEAD"]);
  requireValue(/^[a-f0-9]{40}$/.test(sha), "Expected a full source commit SHA");
  const refs = command("git", ["ls-remote", "origin", `refs/tags/${tag}`, `refs/tags/${tag}^{}`]).split("\n");
  const peeled = refs.find((line) => line.endsWith(`refs/tags/${tag}^{}`)) ?? refs.find((line) => line.endsWith(`refs/tags/${tag}`));
  requireValue(peeled?.split(/\s+/)[0] === sha, "Remote tag does not resolve to the checked-out source commit");
  const marker = `<!-- pokington-source: ${sha} -->`;
  // The tag endpoint only promises published releases; listing includes drafts
  // for the authenticated token with write access.
  const endpoint = `repos/${repository}/releases?per_page=100`;
  const readRelease = () => {
    const response = spawnSync("gh", ["api", "--paginate", "--slurp", endpoint], { encoding: "utf8" });
    requireValue(!response.error && response.status === 0,
      `Cannot inspect release: ${response.error?.message ?? response.stderr}`);
    const pages = JSON.parse(response.stdout);
    requireValue(Array.isArray(pages) && pages.every(Array.isArray), "Unexpected release listing response");
    const matches = pages.flat().filter((release) => release.tag_name === tag);
    requireValue(matches.length <= 1, "Multiple releases identify this tag");
    return matches[0] ?? null;
  };
  const checkDraft = (release) => {
    requireValue(release?.draft === true, "Refusing to change a published release");
    requireValue(release.tag_name === tag && release.target_commitish === sha && release.body?.includes(marker),
      "Existing draft does not identify this exact source commit");
    requireValue(release.assets.every((asset) => files.includes(asset.name)), "Draft contains unexpected assets");
  };
  const existing = readRelease();
  const paths = files.map((name) => resolve(directory, name));
  if (existing) {
    checkDraft(existing);
    command("gh", ["release", "upload", tag, ...paths, "--clobber", "--repo", repository]);
  } else {
    const notes = `${marker}\n\nDownload the archive for your platform and verify it with SHA256SUMS. Extract it and place pokington on your PATH. Catalogue and artwork download on first use.\n\nReview every asset and the successful workflow before publishing this draft.`;
    command("gh", ["release", "create", tag, ...paths, "--draft", "--verify-tag", "--target", sha,
      "--title", `Pokington ${tag}`, "--generate-notes", "--notes", notes, "--repo", repository]);
  }
  const release = readRelease();
  checkDraft(release);
  requireValue(JSON.stringify(release.assets.map((asset) => asset.name).sort()) === JSON.stringify([...files].sort()),
    "Draft asset inventory is incomplete");
  const downloaded = mkdtempSync(join(tmpdir(), "pokington-release-download-"));
  try {
    command("gh", ["release", "download", tag, "--dir", downloaded, "--repo", repository]);
    verifyAssets(downloaded, version);
  } finally {
    rmSync(downloaded, { recursive: true, force: true });
  }
  checkDraft(readRelease());
  console.log(`Draft ready for manual review: ${release.html_url}`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [operation, ...args] = process.argv.slice(2);
    switch (operation) {
      case "version":
        requireValue(args.length <= 1, "Usage: release.mjs version [TAG]");
        console.log(packageVersion(process.cwd(), args[0]));
        break;
      case "package":
        requireValue(args.length === 2, "Usage: release.mjs package TARGET OUTPUT");
        packageRelease(...args);
        break;
      case "collect":
        requireValue(args.length === 2, "Usage: release.mjs collect INPUT OUTPUT");
        collectArtifacts(...args);
        break;
      case "draft":
        requireValue(args.length === 2, "Usage: release.mjs draft TAG ASSETS");
        prepareDraft(...args);
        break;
      default:
        throw new Error("Usage: release.mjs version|package|collect|draft");
    }
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
