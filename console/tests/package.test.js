import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import test from "node:test";

import { buildBundle, currentSourceCommit, verifyBundle } from "../scripts/package-management-center.mjs";

const repoRoot = resolve(import.meta.dirname, "../..");
const consoleRoot = resolve(repoRoot, "console");

// Reviewed DoS advisories also apply to the build/test supply chain. Keep the
// locked transitive versions above all reviewed vulnerable ranges, even when
// the production-only npm audit does not include these dev dependencies.
function lockedVersionAtLeast(
  name,
  minimum,
  lock = JSON.parse(readFileSync(join(consoleRoot, "package-lock.json"), "utf8")),
) {
  const entries = Object.entries(lock.packages).filter(([path]) =>
    path === `node_modules/${name}` || path.endsWith(`/node_modules/${name}`));
  assert.ok(entries.length > 0, `reviewed package missing: ${name}`);
  return entries.every(([, { version }]) => {
    assert.match(version ?? "", /^\d+\.\d+\.\d+$/);
    const actual = version.split(".").map(Number);
    for (let index = 0; index < minimum.length; index += 1) {
      if (actual[index] !== minimum[index]) return actual[index] > minimum[index];
    }
    return true;
  });
}

test("build lock excludes reviewed brace-expansion DoS versions", () => {
  // Floor covers GHSA-q2hr-2g5m-vwhr, GHSA-qhr7-859c-m2p7
  // and GHSA-6j4f-fj2g-mc7p.
  assert.ok(lockedVersionAtLeast("brace-expansion", [5, 0, 12]));
});

test("build lock excludes reviewed source-map-js DoS versions", () => {
  // GHSA-68fv-2mgg-jv7q: indexed source-map offset validation.
  assert.ok(lockedVersionAtLeast("source-map-js", [1, 2, 2]));
});

test("reviewed dependency floor refuses vulnerable nested copies and missing packages", () => {
  for (const [name, minimum, patched, vulnerable] of [
    ["brace-expansion", [5, 0, 12], "5.0.12", "5.0.11"],
    ["source-map-js", [1, 2, 2], "1.2.2", "1.2.1"],
  ]) {
    const fixture = {
      packages: {
        [`node_modules/${name}`]: { version: patched },
        [`node_modules/other/node_modules/${name}`]: { version: vulnerable },
      },
    };
    assert.equal(lockedVersionAtLeast(name, minimum, fixture), false);
    fixture.packages[`node_modules/other/node_modules/${name}`].version = patched;
    assert.equal(lockedVersionAtLeast(name, minimum, fixture), true);
    assert.throws(() => lockedVersionAtLeast(name, minimum, { packages: {} }));
  }
});

test("static and supply-chain gates validate the generated production bundle", () => {
  for (const script of ["check-static.mjs", "check-supply-chain.mjs"]) {
    execFileSync(process.execPath, [join(consoleRoot, "scripts", script)], {
      cwd: consoleRoot,
      stdio: "pipe",
    });
  }
  const generated = readFileSync(
    join(repoRoot, "crates/hydracache-server/src/generated_console_assets.rs"),
    "utf8",
  );
  for (const path of describeTree(join(consoleRoot, "dist")).map((entry) => entry.path)) {
    assert.match(generated, new RegExp(path.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
  }
});

test("locked Vite build is byte-identical across clean output directories", () => {
  const first = mkdtempSync(join(tmpdir(), "hydracache-vite-a-"));
  const second = mkdtempSync(join(tmpdir(), "hydracache-vite-b-"));
  try {
    for (const destination of [first, second]) {
      execFileSync(
        process.execPath,
        [join(consoleRoot, "node_modules/vite/bin/vite.js"), "build", "--outDir", destination, "--emptyOutDir"],
        { cwd: consoleRoot, stdio: "pipe" },
      );
    }
    assert.deepEqual(describeTree(first), describeTree(second));
  } finally {
    rmSync(first, { recursive: true, force: true });
    rmSync(second, { recursive: true, force: true });
  }
});

test("candidate bundle is deterministic and verifies every artifact", () => {
  const first = mkdtempSync(join(tmpdir(), "hydracache-mc72-a-"));
  const second = mkdtempSync(join(tmpdir(), "hydracache-mc72-b-"));
  try {
    const sourceCommit = currentSourceCommit(repoRoot);
    const a = buildBundle({ repoRoot, outDir: first, sourceCommit });
    const b = buildBundle({ repoRoot, outDir: second, sourceCommit });
    assert.deepEqual(a.manifest, b.manifest);
    assert.equal(verifyBundle({ outDir: first, expectedSourceCommit: sourceCommit }).source_commit, sourceCommit);
  } finally {
    rmSync(first, { recursive: true, force: true });
    rmSync(second, { recursive: true, force: true });
  }
});

test("MC72-W13-MIXED-ARTIFACT rejects one substituted file", () => {
  const destination = mkdtempSync(join(tmpdir(), "hydracache-mc72-mixed-"));
  try {
    const sourceCommit = currentSourceCommit(repoRoot);
    buildBundle({ repoRoot, outDir: destination, sourceCommit });
    const app = join(destination, "console/app.js");
    writeFileSync(app, `${readFileSync(app, "utf8")}\n// substituted artifact\n`, "utf8");
    assert.throws(
      () => verifyBundle({ outDir: destination, expectedSourceCommit: sourceCommit }),
      /bundle artifact mismatch: console\/app\.js/,
    );
  } finally {
    rmSync(destination, { recursive: true, force: true });
  }
});

function describeTree(root) {
  const paths = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = join(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) paths.push(relative(root, absolute).replaceAll("\\", "/"));
    }
  };
  visit(root);
  return paths.sort().map((path) => ({
    path,
    sha256: createHash("sha256").update(readFileSync(join(root, path))).digest("hex"),
  }));
}
