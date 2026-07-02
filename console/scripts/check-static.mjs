import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const consoleRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const required = [
  "index.html",
  "app.js",
  "style.css",
  "README.md",
  "tests/console_readonly.spec.js",
  "tests/fixtures.js",
  "playwright.config.mjs"
];

for (const file of required) {
  readFileSync(join(consoleRoot, file), "utf8");
}

const app = readFileSync(join(consoleRoot, "app.js"), "utf8");
const css = readFileSync(join(consoleRoot, "style.css"), "utf8");
const spec = readFileSync(join(consoleRoot, "tests/console_readonly.spec.js"), "utf8");

for (const marker of [
  "MAX_RENDERED_MEMBERS",
  "/cluster/overview",
  "/metrics",
  "renderDegraded",
  "source-badge"
]) {
  if (!app.includes(marker)) {
    throw new Error(`missing console app marker: ${marker}`);
  }
}

for (const marker of [".topology-shell", ".source-badge", ".member-card", ".degraded"]) {
  if (!css.includes(marker)) {
    throw new Error(`missing console CSS marker: ${marker}`);
  }
}

for (const testName of [
  "console_renders_live_cluster_overview_from_endpoints",
  "console_is_read_only_no_mutate_controls",
  "modeled_source_is_shown_as_modeled_never_live",
  "console_shows_degraded_state_when_server_unreachable",
  "console_render_is_bounded_for_large_clusters"
]) {
  if (!spec.includes(testName)) {
    throw new Error(`missing W4 console spec: ${testName}`);
  }
}

console.log("console static checks passed");
