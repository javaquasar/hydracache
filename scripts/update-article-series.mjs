#!/usr/bin/env node

import { access, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");
const defaultManifest = "docs/articles/hydracache-runtime-series.json";

function printUsage() {
  console.log(`Usage:
  node scripts/update-article-series.mjs --article <path>
  node scripts/update-article-series.mjs --all
  node scripts/update-article-series.mjs --article <path> --set-url <published-url>

Updates the generated series/resources block in article drafts.

Options:
  --manifest <path>  Series manifest. Defaults to ${defaultManifest}
  --article <path>   Article file to update.
  --all              Update every existing article file listed in the manifest.
  --set-url <url>    Set the published URL for --article in the manifest before updating.
  --dry-run          Print generated output paths without writing files.
  --help             Show this help.
`);
}

function parseArgs(argv) {
  const options = {
    manifest: defaultManifest,
    all: false,
    dryRun: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--help" || arg === "-h") {
      options.help = true;
    } else if (arg === "--all") {
      options.all = true;
    } else if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--manifest") {
      options.manifest = requiredValue(argv, index, arg);
      index += 1;
    } else if (arg === "--article") {
      options.article = requiredValue(argv, index, arg);
      index += 1;
    } else if (arg === "--set-url") {
      options.setUrl = requiredValue(argv, index, arg);
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
  }

  if (!options.help && !options.all && !options.article) {
    throw new Error("Use --article <path> or --all.");
  }
  if (options.all && options.article) {
    throw new Error("Use either --article or --all, not both.");
  }
  if (options.setUrl && !options.article) {
    throw new Error("--set-url requires --article.");
  }

  return options;
}

function requiredValue(argv, index, arg) {
  const value = argv[index + 1];
  if (!value || value.startsWith("--")) {
    throw new Error(`${arg} requires a value`);
  }
  return value;
}

async function loadManifest(manifestPath) {
  const absolutePath = path.resolve(repoRoot, manifestPath);
  const manifest = JSON.parse(await readFile(absolutePath, "utf8"));
  validateManifest(manifest, absolutePath);
  return {
    absolutePath,
    dir: path.dirname(absolutePath),
    manifest
  };
}

function validateManifest(manifest, manifestPath) {
  if (!manifest.id || !manifest.title || !manifest.description) {
    throw new Error(`Series manifest is missing id, title, or description: ${manifestPath}`);
  }
  if (!Array.isArray(manifest.articles) || manifest.articles.length === 0) {
    throw new Error(`Series manifest must contain at least one article: ${manifestPath}`);
  }
  if (!Array.isArray(manifest.resources)) {
    throw new Error(`Series manifest resources must be an array: ${manifestPath}`);
  }
}

function findArticleEntry(manifest, manifestDir, articlePath) {
  const absoluteArticlePath = path.resolve(repoRoot, articlePath);
  const normalizedArticlePath = normalizePath(absoluteArticlePath);

  const entry = manifest.articles.find((article) => {
    const absoluteEntryPath = path.resolve(manifestDir, article.file);
    return normalizePath(absoluteEntryPath) === normalizedArticlePath;
  });

  if (!entry) {
    throw new Error(`Article is not listed in the series manifest: ${articlePath}`);
  }

  return {
    entry,
    absoluteArticlePath
  };
}

function normalizePath(value) {
  return path.resolve(value).toLowerCase();
}

async function fileExists(filePath) {
  try {
    await access(filePath);
    return true;
  } catch {
    return false;
  }
}

function buildSeriesBlock(manifest, currentArticle) {
  const lines = [
    `<!-- article-series:start ${manifest.id} -->`,
    `## ${manifest.title}`,
    "",
    manifest.description,
    "",
    `You are reading: Part ${currentArticle.part}.`,
    ""
  ];

  for (const article of manifest.articles) {
    lines.push(articleLine(article, currentArticle));
  }

  for (const resource of manifest.resources) {
    lines.push("");
    lines.push(`${resource.label}:`);
    lines.push("");
    lines.push(resource.url);
  }

  lines.push(`<!-- article-series:end -->`);
  return `${lines.join("\n")}\n`;
}

function articleLine(article, currentArticle) {
  const label = `Part ${article.part}: ${article.title}`;
  if (article.part === currentArticle.part) {
    return `- ${label}`;
  }
  if (article.url) {
    return `- [${label}](${article.url})`;
  }
  if (article.status === "planned") {
    return `- ${label} (planned)`;
  }
  return `- ${label}`;
}

function replaceOrInsertSeriesBlock(markdown, manifest, currentArticle) {
  const block = buildSeriesBlock(manifest, currentArticle);
  const startMarker = `<!-- article-series:start ${manifest.id} -->`;
  const endMarker = "<!-- article-series:end -->";
  const startIndex = markdown.indexOf(startMarker);

  if (startIndex !== -1) {
    const endIndex = markdown.indexOf(endMarker, startIndex);
    if (endIndex === -1) {
      throw new Error(`Found ${startMarker} without ${endMarker}.`);
    }
    return `${markdown.slice(0, startIndex)}${block}${markdown.slice(endIndex + endMarker.length).replace(/^\r?\n/, "")}`;
  }

  const insertionIndex = findSeriesInsertionIndex(markdown);
  return `${markdown.slice(0, insertionIndex)}${block}\n${markdown.slice(insertionIndex).replace(/^\r?\n/, "")}`;
}

function findSeriesInsertionIndex(markdown) {
  const normalized = markdown.replace(/\r\n/g, "\n");
  const lines = normalized.split("\n");
  let index = lines.findIndex((line) => /^#\s+/.test(line));
  if (index === -1) {
    return 0;
  }

  index += 1;
  while (index < lines.length && lines[index].trim() === "") {
    index += 1;
  }
  if (index < lines.length && /^!\[[^\]]*]\([^)]+\)\s*$/.test(lines[index])) {
    index += 1;
  }
  while (index < lines.length && lines[index].trim() === "") {
    index += 1;
  }

  const prefix = lines.slice(0, index).join("\n");
  return prefix.length === 0 ? 0 : prefix.length + 1;
}

async function updateArticle(manifestContext, articleEntry, options) {
  const markdown = await readFile(articleEntry.absoluteArticlePath, "utf8");
  const updated = replaceOrInsertSeriesBlock(
    markdown.replace(/\r\n/g, "\n"),
    manifestContext.manifest,
    articleEntry.entry
  );

  if (options.dryRun) {
    console.log(`Would update ${path.relative(repoRoot, articleEntry.absoluteArticlePath)}`);
    return;
  }

  await writeFile(articleEntry.absoluteArticlePath, updated, "utf8");
  console.log(`Updated ${path.relative(repoRoot, articleEntry.absoluteArticlePath)}`);
}

async function updateAllArticles(manifestContext, options) {
  for (const entry of manifestContext.manifest.articles) {
    const absoluteArticlePath = path.resolve(manifestContext.dir, entry.file);
    if (!(await fileExists(absoluteArticlePath))) {
      continue;
    }
    await updateArticle(
      manifestContext,
      {
        entry,
        absoluteArticlePath
      },
      options
    );
  }
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    printUsage();
    return;
  }

  const manifestContext = await loadManifest(options.manifest);

  if (options.article) {
    const articleEntry = findArticleEntry(manifestContext.manifest, manifestContext.dir, options.article);
    if (options.setUrl) {
      articleEntry.entry.url = options.setUrl;
      if (!options.dryRun) {
        await writeFile(manifestContext.absolutePath, `${JSON.stringify(manifestContext.manifest, null, 2)}\n`, "utf8");
        console.log(`Updated ${path.relative(repoRoot, manifestContext.absolutePath)}`);
      } else {
        console.log(`Would set URL for part ${articleEntry.entry.part}: ${options.setUrl}`);
      }
    }

    await updateArticle(manifestContext, articleEntry, options);
    return;
  }

  await updateAllArticles(manifestContext, options);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
