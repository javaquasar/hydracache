import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { isInsideRoot, resolveRequestPath, startServer } from "./serve-static.mjs";
import {
  keyAgreement,
  scriptAtStep,
  timelineEvents,
  verdictView,
} from "../insight.js";

const demoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const repoRoot = resolve(demoRoot, "..");
const failures = [];

function check(condition, message) {
  if (!condition) {
    failures.push(message);
  }
}

function read(relativePath) {
  return readFileSync(join(demoRoot, relativePath), "utf8");
}

// 1. Every file the page and the gates need exists, including the wasm-pack
//    output (build it first: see demo/README.md "Local Build").
const required = [
  "index.html",
  "app.js",
  "share.js",
  "scenarios.js",
  "style.css",
  "pkg/hydracache_sim_wasm.js",
  "pkg/hydracache_sim_wasm_bg.wasm",
  "tests/ui_smoke.spec.js",
  "tests/seed_share.spec.js",
  "scripts/serve-static.mjs",
  "playwright.config.mjs",
];
for (const file of required) {
  check(
    existsSync(join(demoRoot, file)),
    file.startsWith("pkg/")
      ? `missing ${file}: run wasm-pack build crates/hydracache-sim-wasm --target web --out-dir ../../demo/pkg --release -- --locked`
      : `missing ${file}`,
  );
}

// 2. Every module the page loads is local and is published by the Pages
//    bundle step, so a new module cannot ship to GitHub Pages missing.
const html = read("index.html");
const pageModules = new Set();
for (const match of html.matchAll(/<(?:script|link)\b[^>]*\b(?:src|href)="([^"]+)"/g)) {
  pageModules.add(match[1]);
}
const queue = ["app.js"];
const seen = new Set();
while (queue.length > 0) {
  const file = queue.shift();
  if (seen.has(file) || !existsSync(join(demoRoot, file))) {
    continue;
  }
  seen.add(file);
  const source = read(file);
  for (const match of source.matchAll(/(?:from\s+|import\s*\(\s*)"(\.[^"]+)"/g)) {
    const target = join(dirname(file), match[1]).replace(/\\/g, "/");
    pageModules.add(`./${target}`);
    if (!target.startsWith("pkg/")) {
      queue.push(target);
    }
  }
}
for (const module of pageModules) {
  check(
    /^\.\/[\w./-]+$/.test(module) && !module.includes(".."),
    `page asset must be a local relative path: ${module}`,
  );
  check(existsSync(join(demoRoot, module)), `page asset does not exist: ${module}`);
}
const docsWorkflow = readFileSync(join(repoRoot, ".github/workflows/docs-site.yml"), "utf8");
const bundled = new Set();
for (const match of docsWorkflow.matchAll(/install -m 0644 demo\/((?:[\w-]+\/)*)(?:\{([^}]+)\}|([^\s"]+))/g)) {
  const [, directory, group, single] = match;
  for (const name of (group ?? single).split(",")) {
    bundled.add(`${directory}${name.trim()}`);
  }
}
check(bundled.size > 0, "could not read the demo bundle list from .github/workflows/docs-site.yml");
for (const module of pageModules) {
  const file = module.replace(/^\.\//, "");
  check(
    bundled.has(file),
    `${file} is loaded by the page but not bundled by .github/workflows/docs-site.yml`,
  );
}

// 3. The demo package version follows the workspace release version.
const cargoToml = readFileSync(join(repoRoot, "Cargo.toml"), "utf8");
const workspaceVersion = cargoToml.match(/\[workspace\.package\][^[]*?\nversion\s*=\s*"([^"]+)"/)?.[1];
const packageVersion = JSON.parse(read("package.json")).version;
check(Boolean(workspaceVersion), "could not read [workspace.package] version from Cargo.toml");
check(
  packageVersion === workspaceVersion,
  `demo/package.json version ${packageVersion} must match workspace version ${workspaceVersion}`,
);

// 4. Accessibility/glass markers and the named smoke tests stay in place.
const css = read("style.css");
for (const marker of [
  "--glass",
  "backdrop-filter",
  "prefers-reduced-motion",
  "prefers-reduced-transparency",
  ".packet-trail",
]) {
  check(css.includes(marker), `missing glass CSS marker: ${marker}`);
}
const spec = read("tests/ui_smoke.spec.js");
for (const testName of [
  "glass_theme_renders_and_controls_remain_operable",
  "reduced_motion_and_transparency_fallbacks_apply",
]) {
  check(spec.includes(testName), `missing W6 UI smoke: ${testName}`);
}

// 5. The local static server only serves demo files and never crashes on a
//    malformed request.
for (const [pathname, expected] of [
  ["/demo/", { file: join(demoRoot, "index.html") }],
  ["/hydracache/demo/app.js", { file: join(demoRoot, "app.js") }],
  ["/demo", { redirect: "/demo/" }],
  ["/", { redirect: "/demo/" }],
  ["/demo/%E0%A4%A", { status: 400 }],
  ["/demo/..%5c..%5cCargo.toml", { status: 400 }],
  ["/demo/../Cargo.toml", { status: 404 }],
  ["/demo/%2e%2e/Cargo.toml", { status: 404 }],
  ["/Cargo.toml", { status: 404 }],
  ["/crates/hydracache-sim/src/lib.rs", { status: 404 }],
  ["/demo/node_modules/@playwright/test/package.json", { status: 404 }],
  ["/demo/.env", { status: 404 }],
]) {
  const actual = resolveRequestPath(pathname, demoRoot);
  check(
    JSON.stringify(actual) === JSON.stringify(expected),
    `serve-static ${pathname}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
  );
}
check(
  !isInsideRoot(demoRoot, `${demoRoot}-evil${join("/", "index.html")}`) &&
    isInsideRoot(demoRoot, join(demoRoot, "index.html")),
  "serve-static must not treat a sibling directory with the same name prefix as inside the root",
);
const server = await startServer(0);
try {
  const { port } = server.address();
  for (const [pathname, status] of [
    ["/demo/", 200],
    ["/hydracache/demo/pkg/hydracache_sim_wasm_bg.wasm", 200],
    ["/demo/%E0%A4%A", 400],
    ["/Cargo.toml", 404],
  ]) {
    const response = await fetch(`http://127.0.0.1:${port}${pathname}`, { redirect: "manual" });
    check(response.status === status, `serve-static GET ${pathname}: expected ${status}, got ${response.status}`);
  }
} finally {
  server.closeAllConnections();
  await new Promise((resolveClose) => server.close(resolveClose));
}

// 6. View-model helpers: the verdict names a reproducer for *this* run, replica
//    agreement is computed over live nodes, and rewinding keeps only the
//    interventions at or before the target step.
const baseSnapshot = {
  seed: 7,
  step: 40,
  intervention_count: 0,
  active_scenario: null,
  first_violation_step: 31,
  verdict: { status: "violated", invariant: "election_safety", detail: "two leaders" },
  nodes: [
    { id: "node-0", crashed: false, disabled: false, vote_state: "leader" },
    { id: "node-1", crashed: false, disabled: false, vote_state: "follower" },
    { id: "node-2", crashed: true, disabled: false, vote_state: "crashed" },
  ],
  keys: [],
};
const plain = verdictView(baseSnapshot);
check(plain.kind === "bad", "violated verdict must render as bad");
check(
  plain.text.includes("vopr -- --seed 7 --steps 40") && plain.text.includes("first at step 31"),
  `plain-seed violation must give the seed command and first step: ${plain.text}`,
);
const intervened = verdictView({ ...baseSnapshot, intervention_count: 2 });
check(
  intervened.text.includes("--script hydracache-replay-seed-7-step-40.json") &&
    !intervened.text.includes("--seed 7"),
  `intervened violation must not offer the plain seed command: ${intervened.text}`,
);
const scenarioRun = verdictView({ ...baseSnapshot, active_scenario: "minority_partition_cannot_commit" });
check(scenarioRun.text.includes("--script"), "scenario violation must point at the replay script");
const recovered = verdictView({ ...baseSnapshot, verdict: { status: "holding" } });
check(
  recovered.kind === "warn" && recovered.text.includes("first reported at step 31"),
  `a recovered run must keep its first violation visible: ${recovered.text}`,
);
const agreement = keyAgreement({
  ...baseSnapshot,
  keys: [
    { key: "a", replicas: [{ node_id: "node-0", version: 5 }, { node_id: "node-1", version: 5 }] },
    { key: "b", replicas: [{ node_id: "node-0", version: 5 }, { node_id: "node-1", version: 6 }] },
    { key: "c", replicas: [{ node_id: "node-0", version: 5 }] },
  ],
});
check(agreement.keys[0].agree, "matching live replicas agree even if a crashed node lacks the key");
check(!agreement.keys[1].agree, "different live checksums must not agree");
check(!agreement.keys[2].agree, "a live node missing the key must not agree");
check(!agreement.agree, "overall agreement requires every key to agree");
const rewound = scriptAtStep(
  { version: 1, seed: 7, actions: [{ type: "crash", at_step: 3 }, { type: "restart", at_step: 9 }] },
  5,
);
check(
  rewound.target_step === 5 && rewound.actions.length === 1 && rewound.actions[0].type === "crash",
  "rewinding keeps interventions at or before the target step only",
);
const events = timelineEvents(
  { actions: [{ type: "crash", node: "node-0", at_step: 4 }] },
  new Map([
    [3, { leader: "node-0", verdict: "holding" }],
    [6, { leader: "node-1", verdict: "holding" }],
  ]),
);
check(
  events.map((event) => `${event.step}:${event.kind}`).join(",") === "4:action,6:leader",
  `timeline events must merge recorded actions and observed leader changes: ${JSON.stringify(events)}`,
);

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`demo static check failed: ${failure}`);
  }
  // Let pending handles close; process.exit() here trips a libuv assertion on Windows.
  process.exitCode = 1;
} else {
  console.log("demo static checks passed");
}
