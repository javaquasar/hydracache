import { readFile, readdir } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

function parseVersion(file) {
  const match = /^(\d+)\.(\d+)\.(\d+)\.md$/.exec(file);
  return match ? match.slice(1).map(Number) : null;
}

function atLeast071(version) {
  const [major, minor] = version;
  return major > 0 || minor >= 71;
}

export async function checkReleaseDocs(root) {
  const canonicalRoot = path.join(root, "docs", "releases");
  const siteRoot = path.join(root, "docs-site", "src");
  const wrappersRoot = path.join(siteRoot, "releases");
  const summary = await readFile(path.join(siteRoot, "SUMMARY.md"), "utf8");
  const archive = await readFile(path.join(wrappersRoot, "index.md"), "utf8");
  const failures = [];

  for (const file of await readdir(canonicalRoot)) {
    const version = parseVersion(file);
    if (!version || !atLeast071(version)) {
      continue;
    }

    const wrapperPath = path.join(wrappersRoot, file);
    const expectedInclude = `{{#include ../../../docs/releases/${file}}}`;
    let wrapper;
    try {
      wrapper = (await readFile(wrapperPath, "utf8")).trim();
    } catch {
      failures.push(`${file}: missing docs-site release wrapper`);
      continue;
    }

    if (wrapper !== expectedInclude) {
      failures.push(`${file}: wrapper must contain only ${expectedInclude}`);
    }
    if (!summary.includes(`releases/${file}`)) {
      failures.push(`${file}: missing from docs-site/src/SUMMARY.md`);
    }
    if (!archive.includes(`(${file})`)) {
      failures.push(`${file}: missing from docs-site/src/releases/index.md`);
    }

    const canonical = await readFile(path.join(canonicalRoot, file), "utf8");
    const relativeLinks = [
      ...canonical.matchAll(/\[[^\]]+\]\((?!https?:|mailto:|#)([^)]+)\)/g),
    ];
    for (const match of relativeLinks) {
      failures.push(`${file}: included release notes must use an absolute link: ${match[1]}`);
    }
  }

  return failures;
}

const isMain = process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (isMain) {
  const failures = await checkReleaseDocs(process.cwd());
  if (failures.length > 0) {
    console.error("Release documentation synchronization failed:");
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exit(1);
  }

  console.log("Release documentation wrappers, navigation and canonical sources are synchronized.");
}
