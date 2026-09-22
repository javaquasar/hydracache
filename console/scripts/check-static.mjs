import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const consoleRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const repoRoot = join(consoleRoot, "..");
const distRoot = join(consoleRoot, "dist");
const embeddedRoot = join(repoRoot, "crates/hydracache-server/console");
const required = [
  "index.html",
  "package.json",
  "package-lock.json",
  "tsconfig.json",
  "vite.config.ts",
  "vitest.config.ts",
  "src/app.tsx",
  "src/controller.ts",
  "src/api.ts",
  "src/capabilities.ts",
  "src/state.ts",
  "src/router.ts",
  "src/history.ts",
  "src/components/shell.tsx",
  "src/components/primitives.tsx",
  "src/components/error-boundary.tsx",
  "src/pages/visibility.ts",
  "README.md",
  "tests/console_readonly.spec.js",
  "tests/fixtures.js",
  "playwright.config.mjs",
  "scripts/embed-dist.mjs",
  "scripts/package-management-center.mjs",
  "tests/package.test.js",
];
for (const file of required) readFileSync(join(consoleRoot, file), "utf8");

const distFiles = files(distRoot);
if (!distFiles.includes("manifest.json") || !distFiles.includes("index.html")) {
  throw new Error("production console is missing its Vite manifest or index");
}
if (distFiles.some((path) => path.endsWith(".map"))) {
  throw new Error("production console contains source maps");
}
for (const file of distFiles) {
  const source = readFileSync(join(distRoot, file));
  const embedded = readFileSync(join(embeddedRoot, file));
  if (!source.equals(embedded)) throw new Error(`embedded console asset drifted: ${file}`);
}
const embeddedFiles = files(embeddedRoot);
if (JSON.stringify(embeddedFiles) !== JSON.stringify(distFiles)) {
  throw new Error("embedded console asset set differs from Vite dist");
}

const manifest = JSON.parse(readFileSync(join(distRoot, "manifest.json"), "utf8"));
const entry = manifest["index.html"];
if (!entry?.isEntry || !/^assets\/.+-[A-Za-z0-9_-]+\.js$/.test(entry.file)) {
  throw new Error("Vite entry is not content hashed");
}
if (!Array.isArray(entry.css) || entry.css.some((path) => !/^assets\/.+-[A-Za-z0-9_-]+\.css$/.test(path))) {
  throw new Error("Vite styles are not content hashed");
}

const sourceText = files(join(consoleRoot, "src"))
  .filter((path) => /\.tsx?$/.test(path))
  .map((path) => readFileSync(join(consoleRoot, "src", path), "utf8"))
  .join("\n");
const history = readFileSync(join(consoleRoot, "src/history.ts"), "utf8");
const css = readFileSync(join(consoleRoot, "style.css"), "utf8");
const spec = readFileSync(join(consoleRoot, "tests/console_readonly.spec.js"), "utf8");
const bounds = readFileSync(join(repoRoot, "docs/testing/management-center/0.72/bounds.toml"), "utf8");

for (const marker of [
  "MAX_RENDERED_MEMBERS",
  "/management/v1/dashboard",
  "AbortController",
  "visibilitychange",
  "renderDegraded",
  "source-badge",
  "render(<AppShell />",
  "MANAGEMENT_ROUTES",
  "capabilityAllowsEndpoint",
  "ManagementQueryCache",
]) {
  if (!sourceText.includes(marker)) throw new Error(`missing console source marker: ${marker}`);
}
for (const [key, value] of [["max_series", "24"], ["max_points_per_series", "360"], ["max_total_points", "4320"], ["max_encoded_bytes", "262144"]]) {
  if (!bounds.includes(`${key} = ${value}`)) throw new Error(`browser history bound drifted: ${key}`);
}
for (const marker of ["HISTORY_LIMITS", "maxSeries", "maxPointsPerSeries", "maxTotalPoints", "maxBytes", "counter", "gauge"]) {
  if (!history.includes(marker)) throw new Error(`missing bounded history marker: ${marker}`);
}
for (const marker of [".app-shell", ".source-badge", ".truth-chip", ".degraded"]) {
  if (!css.includes(marker)) throw new Error(`missing console CSS marker: ${marker}`);
}
for (const forbidden of ["innerHTML", "outerHTML", "insertAdjacentHTML", "document.write", "eval(", "new Function", "x-hydracache-admin"]) {
  if (sourceText.includes(forbidden)) throw new Error(`unsafe or write-capable console marker: ${forbidden}`);
}
for (const [name, pattern] of [
  ["private key", /-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----/],
  ["AWS access key", /AKIA[0-9A-Z]{16}/],
  ["bearer credential", /Bearer\s+[A-Za-z0-9._~+\/-]{12,}/i],
  ["embedded password", /(?:password|passwd|secret)\s*[:=]\s*["'][^"']{4,}["']/i],
]) {
  for (const [file, text] of [["source", sourceText], ["style.css", css]]) {
    if (pattern.test(text)) throw new Error(`possible ${name} in console ${file}`);
  }
}
for (const marker of ["x-hydracache-management-read", ":focus-visible", "prefers-reduced-motion", "forced-colors"]) {
  if (!`${sourceText}\n${css}`.includes(marker)) throw new Error(`missing W11 marker: ${marker}`);
}
for (const testName of [
  "console_renders_typed_dashboard_and_all_truth_states",
  "console_is_read_only_and_never_scrapes_prometheus",
  "modeled_source_and_missing_raft_values_are_never_painted_live_or_zero",
  "console_shows_degraded_state_when_required_snapshot_is_unreachable",
  "console_render_is_bounded_for_large_clusters",
  "offline_and_hidden_lifecycle_pauses_polling_and_aborts_obsolete_work",
  "accelerated_browser_soak_respects_frozen_history_budget",
  "hostile_diagnostic_text_is_rendered_only_as_text",
  "management center passes automated accessibility checks",
  "keyboard navigation exposes a visible focus path",
]) {
  if (!spec.includes(testName)) throw new Error(`missing console spec: ${testName}`);
}
console.log(`console static checks passed (${distFiles.length} manifest-bound assets)`);

function files(root) {
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
