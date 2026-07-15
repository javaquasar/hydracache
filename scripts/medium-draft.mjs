#!/usr/bin/env node

import { createRequire } from "node:module";
import { mkdir, readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "..");

const defaultArticle = "docs/articles/001-why-rust-needs-cache-semantics.md";
const defaultProfile = ".playwright/medium-profile";

function printUsage() {
  console.log(`Usage:
  node scripts/medium-draft.mjs [--article <path>] [--profile <path>] [--url <url>]

Creates a Medium draft from a Markdown article and stops before publishing.

Options:
  --article <path>   Markdown article path. Defaults to ${defaultArticle}
  --profile <path>   Persistent browser profile. Defaults to ${defaultProfile}
  --url <url>         Editor URL. Defaults to https://medium.com/new-story
  --dry-run          Parse the article and print the title/body preview only.
  --help             Show this help.

First-time local setup, if Playwright is not already installed:
  npm --prefix console install
  npx --prefix console playwright install chromium
`);
}

function parseArgs(argv) {
  const options = {
    article: defaultArticle,
    profile: defaultProfile,
    url: "https://medium.com/new-story",
    dryRun: false
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--help" || arg === "-h") {
      options.help = true;
    } else if (arg === "--dry-run") {
      options.dryRun = true;
    } else if (arg === "--article") {
      options.article = requiredValue(argv, index, arg);
      index += 1;
    } else if (arg === "--profile") {
      options.profile = requiredValue(argv, index, arg);
      index += 1;
    } else if (arg === "--url") {
      options.url = requiredValue(argv, index, arg);
      index += 1;
    } else {
      throw new Error(`Unknown argument: ${arg}`);
    }
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

async function loadPlaywright() {
  try {
    return await import("playwright");
  } catch {
    // Fall through to repo-local package locations.
  }

  const packageRoots = [
    path.join(repoRoot, "console", "package.json"),
    path.join(repoRoot, "demo", "package.json")
  ];

  for (const packageJson of packageRoots) {
    try {
      const requireFromPackage = createRequire(packageJson);
      return requireFromPackage("playwright");
    } catch {
      // Try the next known package root.
    }
  }

  throw new Error(
    "Playwright is not installed. Run: npm --prefix console install && npx --prefix console playwright install chromium"
  );
}

async function readArticle(articlePath) {
  const absolutePath = path.resolve(repoRoot, articlePath);
  const markdown = await readFile(absolutePath, "utf8");
  const { title, bodyMarkdown } = splitMarkdownArticle(markdown);
  const baseDir = path.dirname(absolutePath);

  return {
    absolutePath,
    title,
    bodyMarkdown,
    bodyText: stripMarkdownComments(bodyMarkdown).trim(),
    bodyHtml: await markdownToHtml(bodyMarkdown, baseDir)
  };
}

function stripMarkdownComments(markdown) {
  return markdown
    .replace(/^\s*<!--.*-->\s*$/gm, "")
    .replace(/\n{3,}/g, "\n\n");
}

function splitMarkdownArticle(markdown) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const firstHeadingIndex = lines.findIndex((line) => /^#\s+/.test(line));

  if (firstHeadingIndex === -1) {
    const fallbackTitle = path.basename(defaultArticle, ".md");
    return {
      title: fallbackTitle,
      bodyMarkdown: markdown
    };
  }

  const rawTitle = lines[firstHeadingIndex].replace(/^#\s+/, "").trim();
  const title = rawTitle.replace(/^\d{3}\s+-\s+/, "");
  const bodyMarkdown = [
    ...lines.slice(0, firstHeadingIndex),
    ...lines.slice(firstHeadingIndex + 1)
  ].join("\n");

  return { title, bodyMarkdown };
}

async function markdownToHtml(markdown, baseDir) {
  const lines = markdown.replace(/\r\n/g, "\n").split("\n");
  const html = [];
  let paragraph = [];
  let list = null;
  let inCode = false;
  let codeLines = [];
  let codeLang = "";

  const flushParagraph = () => {
    if (paragraph.length === 0) {
      return;
    }
    html.push(`<p>${formatInline(paragraph.join(" "))}</p>`);
    paragraph = [];
  };

  const flushList = () => {
    if (!list) {
      return;
    }
    html.push(`<${list.type}>${list.items.map((item) => `<li>${formatInline(item)}</li>`).join("")}</${list.type}>`);
    list = null;
  };

  for (const line of lines) {
    const fence = line.match(/^```([A-Za-z0-9_-]*)\s*$/);
    if (fence && !inCode) {
      flushParagraph();
      flushList();
      inCode = true;
      codeLang = fence[1] || "";
      codeLines = [];
      continue;
    }

    if (fence && inCode) {
      const langClass = codeLang ? ` class="language-${escapeAttribute(codeLang)}"` : "";
      html.push(`<pre><code${langClass}>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
      inCode = false;
      codeLang = "";
      codeLines = [];
      continue;
    }

    if (inCode) {
      codeLines.push(line);
      continue;
    }

    if (/^\s*$/.test(line)) {
      flushParagraph();
      flushList();
      continue;
    }

    if (/^<!--.*-->$/.test(line.trim())) {
      flushParagraph();
      flushList();
      continue;
    }

    const heading = line.match(/^(#{2,6})\s+(.+)$/);
    if (heading) {
      flushParagraph();
      flushList();
      const level = heading[1].length;
      html.push(`<h${level}>${formatInline(heading[2].trim())}</h${level}>`);
      continue;
    }

    const image = line.match(/^!\[([^\]]*)\]\(([^)\s]+)\)\s*$/);
    if (image) {
      flushParagraph();
      flushList();
      html.push(await imageToHtml(image[2], image[1], baseDir));
      continue;
    }

    const unorderedItem = line.match(/^\s*-\s+(.+)$/);
    if (unorderedItem) {
      flushParagraph();
      if (!list || list.type !== "ul") {
        flushList();
        list = { type: "ul", items: [] };
      }
      list.items.push(unorderedItem[1].trim());
      continue;
    }

    const orderedItem = line.match(/^\s*\d+\.\s+(.+)$/);
    if (orderedItem) {
      flushParagraph();
      if (!list || list.type !== "ol") {
        flushList();
        list = { type: "ol", items: [] };
      }
      list.items.push(orderedItem[1].trim());
      continue;
    }

    flushList();
    paragraph.push(line.trim());
  }

  if (inCode) {
    const langClass = codeLang ? ` class="language-${escapeAttribute(codeLang)}"` : "";
    html.push(`<pre><code${langClass}>${escapeHtml(codeLines.join("\n"))}</code></pre>`);
  }
  flushParagraph();
  flushList();

  return html.join("\n");
}

async function imageToHtml(reference, alt, baseDir) {
  const src = await imageSource(reference, baseDir);
  const altText = escapeAttribute(alt);
  return `<figure><img src="${src}" alt="${altText}"></figure>`;
}

async function imageSource(reference, baseDir) {
  if (/^https?:\/\//.test(reference)) {
    return escapeAttribute(reference);
  }

  const imagePath = path.resolve(baseDir, reference);
  const bytes = await readFile(imagePath);
  const mimeType = imageMimeType(imagePath);
  return `data:${mimeType};base64,${bytes.toString("base64")}`;
}

function imageMimeType(imagePath) {
  const extension = path.extname(imagePath).toLowerCase();
  if (extension === ".jpg" || extension === ".jpeg") {
    return "image/jpeg";
  }
  if (extension === ".png") {
    return "image/png";
  }
  if (extension === ".webp") {
    return "image/webp";
  }
  if (extension === ".gif") {
    return "image/gif";
  }

  throw new Error(`Unsupported article image type: ${imagePath}`);
}

function formatInline(value) {
  const placeholders = [];
  let escaped = escapeHtml(value);

  escaped = escaped.replace(/`([^`]+)`/g, (_match, code) => {
    const token = `@@CODE_${placeholders.length}@@`;
    placeholders.push(`<code>${code}</code>`);
    return token;
  });

  escaped = escaped.replace(/\[([^\]]+)\]\((https?:\/\/[^)\s]+)\)/g, (_match, label, url) => {
    const token = `@@LINK_${placeholders.length}@@`;
    placeholders.push(`<a href="${url}">${label}</a>`);
    return token;
  });
  escaped = escaped.replace(/(^|[\s(])(https?:\/\/[^\s<)]+)/g, (_match, prefix, url) => {
    const trailing = url.match(/[.,;:!?]+$/)?.[0] ?? "";
    const cleanUrl = trailing ? url.slice(0, -trailing.length) : url;
    return `${prefix}<a href="${cleanUrl}">${cleanUrl}</a>${trailing}`;
  });
  escaped = escaped.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  escaped = escaped.replace(/\*([^*]+)\*/g, "<em>$1</em>");

  for (const [index, replacement] of placeholders.entries()) {
    escaped = escaped.replace(`@@CODE_${index}@@`, replacement);
    escaped = escaped.replace(`@@LINK_${index}@@`, replacement);
  }

  return escaped;
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function escapeAttribute(value) {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

async function waitForMediumEditor(page) {
  console.log("Waiting for the Medium editor. Log in in the browser if Medium asks for it.");

  const editor = page.locator('[contenteditable="true"]').first();
  await editor.waitFor({ state: "visible", timeout: 10 * 60 * 1000 });

  return editor;
}

async function pasteHtml(page, html, text) {
  const dispatched = await page.evaluate(
    ({ html: htmlValue, text: textValue }) => {
      const target = document.activeElement;
      if (!target) {
        return false;
      }

      const dataTransfer = new DataTransfer();
      dataTransfer.setData("text/html", htmlValue);
      dataTransfer.setData("text/plain", textValue);

      const event = new ClipboardEvent("paste", {
        bubbles: true,
        cancelable: true,
        clipboardData: dataTransfer
      });

      target.dispatchEvent(event);
      return true;
    },
    { html, text }
  );

  if (!dispatched) {
    await page.keyboard.insertText(text);
  }
}

async function fillMediumDraft(page, article) {
  const firstEditor = await waitForMediumEditor(page);

  await firstEditor.click();
  await page.keyboard.insertText(article.title);
  await page.keyboard.press("Enter");

  const editors = page.locator('[contenteditable="true"]');
  const editorCount = await editors.count();
  if (editorCount > 1) {
    await editors.nth(1).click();
  }

  await pasteHtml(page, article.bodyHtml, article.bodyText);
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  if (options.help) {
    printUsage();
    return;
  }

  const article = await readArticle(options.article);

  if (options.dryRun) {
    console.log(`Article: ${article.absolutePath}`);
    console.log(`Title: ${article.title}`);
    console.log("");
    console.log(article.bodyText.slice(0, 1200));
    return;
  }

  const { chromium } = await loadPlaywright();
  const profilePath = path.resolve(repoRoot, options.profile);
  await mkdir(profilePath, { recursive: true });

  const context = await chromium.launchPersistentContext(profilePath, {
    headless: false,
    viewport: { width: 1440, height: 1000 }
  });

  const page = context.pages()[0] ?? (await context.newPage());
  await page.goto(options.url, { waitUntil: "domcontentloaded" });
  await fillMediumDraft(page, article);

  console.log("");
  console.log("Draft content has been inserted into Medium.");
  console.log("Review formatting in the browser and publish manually when ready.");
  console.log("This script intentionally does not click Publish.");
  console.log("");
  console.log("Press Ctrl+C in this terminal when you are done.");

  await new Promise(() => {});
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
