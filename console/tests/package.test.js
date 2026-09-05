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
