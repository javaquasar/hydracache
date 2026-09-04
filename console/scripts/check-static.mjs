import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const consoleRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const required = [
  "index.html",
  "app.js",
  "history.js",
  "style.css",
  "README.md",
  "tests/console_readonly.spec.js",
  "tests/fixtures.js",
  "playwright.config.mjs"
];

for (const file of required) {
  readFileSync(join(consoleRoot, file), "utf8");
}

for (const file of ["index.html", "app.js", "history.js", "style.css"]) {
  const source = readFileSync(join(consoleRoot, file), "utf8").replaceAll("\r\n", "\n");
  const embedded = readFileSync(
    join(consoleRoot, "../crates/hydracache-server/console", file),
    "utf8",
  ).replaceAll("\r\n", "\n");
  if (source !== embedded) {
    throw new Error(`embedded console asset drifted: ${file}`);
  }
}

const app = readFileSync(join(consoleRoot, "app.js"), "utf8");
const history = readFileSync(join(consoleRoot, "history.js"), "utf8");
const css = readFileSync(join(consoleRoot, "style.css"), "utf8");
const spec = readFileSync(join(consoleRoot, "tests/console_readonly.spec.js"), "utf8");
const bounds = readFileSync(
  join(consoleRoot, "../docs/testing/management-center/0.72/bounds.toml"),
  "utf8",
);

for (const marker of [
  "MAX_RENDERED_MEMBERS",
  "/management/v1/dashboard",
  "AbortController",
  "visibilitychange",
  "renderDegraded",
  "source-badge"
]) {
  if (!app.includes(marker)) {
    throw new Error(`missing console app marker: ${marker}`);
  }
}

for (const [key, value] of [
  ["max_series", "24"],
  ["max_points_per_series", "360"],
  ["max_total_points", "4320"],
  ["max_encoded_bytes", "262144"],
]) {
  if (!bounds.includes(`${key} = ${value}`)) {
    throw new Error(`browser history source/bound registry drifted: ${key}`);
  }
}

for (const marker of [
  "HISTORY_LIMITS",
  "maxSeries",
  "maxPointsPerSeries",
  "maxTotalPoints",
  "maxBytes",
  "counter",
  "gauge"
]) {
  if (!history.includes(marker)) {
    throw new Error(`missing bounded history marker: ${marker}`);
  }
}

for (const marker of [".app-shell", ".source-badge", ".truth-chip", ".degraded"]) {
  if (!css.includes(marker)) {
    throw new Error(`missing console CSS marker: ${marker}`);
  }
}

for (const testName of [
  "console_renders_typed_dashboard_and_all_truth_states",
  "console_is_read_only_and_never_scrapes_prometheus",
  "modeled_source_and_missing_raft_values_are_never_painted_live_or_zero",
  "console_shows_degraded_state_when_required_snapshot_is_unreachable",
  "console_render_is_bounded_for_large_clusters",
  "offline_and_hidden_lifecycle_pauses_polling_and_aborts_obsolete_work",
  "accelerated_browser_soak_respects_frozen_history_budget"
]) {
  if (!spec.includes(testName)) {
    throw new Error(`missing W4 console spec: ${testName}`);
  }
}

console.log("console static checks passed");
