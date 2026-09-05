import {
  cpSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { dirname, join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

export const RELEASE = "0.72.0";
const SCRIPT_ROOT = dirname(dirname(fileURLToPath(import.meta.url)));
const DEFAULT_REPO_ROOT = resolve(SCRIPT_ROOT, "..");
const CONTRACTS = [
  "docs/architecture/management-center-v2.md",
  "docs/testing/management-center/0.72/baselines.toml",
  "docs/testing/management-center/0.72/bounds.toml",
  "docs/testing/management-center/0.72/claims.toml",
  "docs/testing/management-center/0.72/fault-matrix.toml",
  "docs/testing/management-center/0.72/healthchecks.toml",
  "docs/testing/management-center/0.72/source-map.toml",
];

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function normalizeCommit(value) {
  const commit = value.trim().toLowerCase();
  if (!/^[0-9a-f]{40}$/.test(commit)) {
    throw new Error("candidate source commit must be an exact 40-character Git SHA");
  }
  return commit;
}

function git(repoRoot, args) {
  return execFileSync("git", ["-C", repoRoot, ...args], { encoding: "utf8" }).trim();
}

export function currentSourceCommit(repoRoot = DEFAULT_REPO_ROOT) {
  return normalizeCommit(git(repoRoot, ["rev-parse", "HEAD"]));
}

export function assertCleanSource(repoRoot = DEFAULT_REPO_ROOT) {
  const dirty = git(repoRoot, ["status", "--porcelain", "--untracked-files=normal"]);
  if (dirty) {
    throw new Error("refusing to package a dirty source tree");
  }
}

function sourceEntries(repoRoot) {
  const entries = [];
  const dist = join(repoRoot, "console/dist");
  const embeddedRoot = join(repoRoot, "crates/hydracache-server/console");
  const assets = walkFiles(dist);
  const embeddedAssets = walkFiles(embeddedRoot);
  if (JSON.stringify(assets) !== JSON.stringify(embeddedAssets)) {
    throw new Error("embedded console asset set differs from Vite dist");
  }
  for (const asset of assets) {
    const source = join(dist, asset);
    const embedded = join(repoRoot, "crates/hydracache-server/console", asset);
    const sourceBytes = readFileSync(source);
    const embeddedBytes = readFileSync(embedded);
    if (!sourceBytes.equals(embeddedBytes)) {
      throw new Error(`embedded console asset drifted: ${asset}`);
    }
    entries.push([source, `console/${asset}`], [embedded, `embedded/${asset}`]);
  }
  for (const contract of CONTRACTS) entries.push([join(repoRoot, contract), `contracts/${contract.split("/").at(-1)}`]);
  entries.push([
    join(repoRoot, "target/management-center-0.72-sbom.cdx.json"),
    "sbom/management-center-0.72-sbom.cdx.json",
  ]);
  return entries;
}

function describeFiles(outDir, names) {
  return [...names].sort().map((path) => {
    const bytes = readFileSync(join(outDir, path));
    return { path, bytes: statSync(join(outDir, path)).size, sha256: sha256(bytes) };
  });
}

function artifactSetDigest(files) {
  return sha256(Buffer.from(files.map((file) => `${file.sha256}  ${file.path}\n`).join(""), "utf8"));
}

export function buildBundle({ repoRoot = DEFAULT_REPO_ROOT, outDir, sourceCommit }) {
  const destination = resolve(outDir ?? join(repoRoot, "target/management-center-0.72-bundle"));
  const commit = normalizeCommit(sourceCommit);
  rmSync(destination, { recursive: true, force: true });
  mkdirSync(destination, { recursive: true });
  const names = [];
  for (const [source, name] of sourceEntries(repoRoot)) {
    const target = join(destination, name);
    mkdirSync(dirname(target), { recursive: true });
    cpSync(source, target);
    names.push(name.replaceAll("\\", "/"));
  }
  const files = describeFiles(destination, names);
  const manifest = {
    schema_version: 1,
    release: RELEASE,
    source_commit: commit,
    reproducible: true,
    artifact_set_sha256: artifactSetDigest(files),
    files,
  };
  writeFileSync(join(destination, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  return { destination, manifest };
}

export function verifyBundle({ outDir, expectedSourceCommit }) {
  const destination = resolve(outDir);
  const manifest = JSON.parse(readFileSync(join(destination, "manifest.json"), "utf8"));
  if (manifest.schema_version !== 1 || manifest.release !== RELEASE || manifest.reproducible !== true) {
    throw new Error("unsupported or incomplete Management Center bundle manifest");
  }
  const expected = normalizeCommit(expectedSourceCommit);
  if (manifest.source_commit !== expected) {
    throw new Error(`bundle source commit mismatch: ${manifest.source_commit} != ${expected}`);
  }
  const names = manifest.files.map((file) => file.path);
  if (new Set(names).size !== names.length || names.some((name) => name.includes("..") || resolve(destination, name).startsWith(destination) === false)) {
    throw new Error("bundle manifest contains duplicate or escaping paths");
  }
  const actual = describeFiles(destination, names);
  for (let index = 0; index < actual.length; index += 1) {
    if (JSON.stringify(actual[index]) !== JSON.stringify(manifest.files[index])) {
      throw new Error(`bundle artifact mismatch: ${manifest.files[index].path}`);
    }
  }
  if (artifactSetDigest(actual) !== manifest.artifact_set_sha256) {
    throw new Error("bundle artifact-set digest mismatch");
  }
  const consoleAssets = names
    .filter((name) => name.startsWith("console/"))
    .map((name) => name.slice("console/".length));
  for (const asset of consoleAssets) {
    const source = readFileSync(join(destination, "console", asset));
    const embedded = readFileSync(join(destination, "embedded", asset));
    if (!source.equals(embedded)) throw new Error(`bundle embedded asset mismatch: ${asset}`);
  }
  return manifest;
}

function walkFiles(root) {
  const found = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) found.push(relative(root, absolute).replaceAll("\\", "/"));
    }
  };
  visit(root);
  return found.sort();
}

function cli() {
  const mode = process.argv[2] ?? "package";
  const repoRoot = DEFAULT_REPO_ROOT;
  const outDir = process.env.HYDRACACHE_MANAGEMENT_BUNDLE_DIR
    ? resolve(process.env.HYDRACACHE_MANAGEMENT_BUNDLE_DIR)
    : join(repoRoot, "target/management-center-0.72-bundle");
  const expected = process.env.HYDRACACHE_CANDIDATE_SHA ?? currentSourceCommit(repoRoot);
  if (mode === "package") {
    if (process.env.HYDRACACHE_ALLOW_DIRTY_BUNDLE !== "1") assertCleanSource(repoRoot);
    const result = buildBundle({ repoRoot, outDir, sourceCommit: expected });
    verifyBundle({ outDir: result.destination, expectedSourceCommit: expected });
    console.log(`packaged and verified ${relative(repoRoot, result.destination)} at ${result.manifest.source_commit}`);
    return;
  }
  if (mode === "verify") {
    const manifest = verifyBundle({ outDir, expectedSourceCommit: expected });
    console.log(`verified ${relative(repoRoot, outDir)} at ${manifest.source_commit}`);
    return;
  }
  if (mode === "test-temp") {
    const destination = mkdtempSync(join(tmpdir(), "hydracache-mc72-"));
    try {
      buildBundle({ repoRoot, outDir: destination, sourceCommit: expected });
      verifyBundle({ outDir: destination, expectedSourceCommit: expected });
    } finally {
      rmSync(destination, { recursive: true, force: true });
    }
    return;
  }
  throw new Error(`unknown mode: ${mode}`);
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) cli();
