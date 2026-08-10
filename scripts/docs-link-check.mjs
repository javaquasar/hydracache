import { readFile, readdir, stat } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const docsRoot = path.join(root, "docs-site", "src");
const checkedRoots = [path.join(root, "README.md"), path.join(root, "docs-site", "README.md")];
const markdownFiles = [];

async function collectMarkdown(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      await collectMarkdown(full);
    } else if (entry.isFile() && entry.name.endsWith(".md")) {
      markdownFiles.push(full);
    }
  }
}

await collectMarkdown(docsRoot);
for (const file of checkedRoots) {
  markdownFiles.push(file);
}

const failures = [];

function isExternal(target) {
  return /^(https?:|mailto:|#)/.test(target);
}

function stripAnchor(target) {
  return target.split("#")[0];
}

async function exists(file) {
  try {
    await stat(file);
    return true;
  } catch {
    return false;
  }
}

for (const file of markdownFiles) {
  const text = await readFile(file, "utf8");
  const links = [...text.matchAll(/\[[^\]]+\]\(([^)]+)\)/g)].map((match) => match[1]);
  const images = [...text.matchAll(/<img\s+[^>]*src="([^"]+)"/g)].map((match) => match[1]);

  for (const raw of [...links, ...images]) {
    const target = stripAnchor(raw.trim());
    if (!target || isExternal(target)) {
      continue;
    }

    const resolved = path.resolve(path.dirname(file), target);
    if (!(await exists(resolved))) {
      failures.push(`${path.relative(root, file)} -> ${raw}`);
    }
  }
}

if (failures.length > 0) {
  console.error("Broken local documentation links:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Checked ${markdownFiles.length} markdown files for local links.`);
