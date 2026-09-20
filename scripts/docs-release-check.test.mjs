import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";

import { checkReleaseDocs } from "./docs-release-check.mjs";

async function fixture({ wrapper = true, link = "https://example.invalid/evidence" } = {}) {
  const root = await mkdtemp(path.join(tmpdir(), "hydracache-docs-release-"));
  const canonicalRoot = path.join(root, "docs", "releases");
  const siteRoot = path.join(root, "docs-site", "src");
  const releasesRoot = path.join(siteRoot, "releases");
  await mkdir(canonicalRoot, { recursive: true });
  await mkdir(releasesRoot, { recursive: true });
  await writeFile(
    path.join(canonicalRoot, "0.71.0.md"),
    `# Release 0.71.0\n\n[Evidence](${link})\n`,
  );
  await writeFile(path.join(siteRoot, "SUMMARY.md"), "- [0.71](releases/0.71.0.md)\n");
  await writeFile(path.join(releasesRoot, "index.md"), "- [0.71](0.71.0.md)\n");
  if (wrapper) {
    await writeFile(
      path.join(releasesRoot, "0.71.0.md"),
      "{{#include ../../../docs/releases/0.71.0.md}}\n",
    );
  }
  return root;
}

test("accepts an exact include wrapper with absolute links", async (context) => {
  const root = await fixture();
  context.after(() => rm(root, { recursive: true, force: true }));
  assert.deepEqual(await checkReleaseDocs(root), []);
});

test("rejects a missing site wrapper", async (context) => {
  const root = await fixture({ wrapper: false });
  context.after(() => rm(root, { recursive: true, force: true }));
  assert.match((await checkReleaseDocs(root)).join("\n"), /missing docs-site release wrapper/);
});

test("rejects relative links that change meaning after inclusion", async (context) => {
  const root = await fixture({ link: "../testing/evidence.md" });
  context.after(() => rm(root, { recursive: true, force: true }));
  assert.match((await checkReleaseDocs(root)).join("\n"), /must use an absolute link/);
});
