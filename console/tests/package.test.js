import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";

import { buildBundle, currentSourceCommit, verifyBundle } from "../scripts/package-management-center.mjs";

const repoRoot = resolve(import.meta.dirname, "../..");

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
