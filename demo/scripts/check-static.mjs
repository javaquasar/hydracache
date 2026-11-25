import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { isInsideRoot, resolveRequestPath, startServer } from "./serve-static.mjs";

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
  "tests/server_engine.spec.js",
  "tests/a11y.spec.js",
  "tests/visual.spec.js",
  "tests/support/violation-stub.js",
  "unit/share.test.mjs",
  "unit/insight.test.mjs",
  "unit/packets.test.mjs",
  "scripts/serve-static.mjs",
  "scripts/og-image.mjs",
  "og-image.png",
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

// 2b. Link previews: the og:image is an absolute URL under the published demo
//     path, the file it names exists locally, is a 1200x630 PNG, and is both
//     bundled and checksummed by the Pages workflow.
const PAGES_DEMO_URL = "https://javaquasar.github.io/hydracache/demo/";
for (const property of ["og:title", "og:description", "og:url", "og:image", "og:image:alt"]) {
  check(html.includes(`property="${property}"`), `missing <meta property="${property}"> in index.html`);
}
check(html.includes('name="twitter:card" content="summary_large_image"'), "missing twitter:card summary_large_image");
check(html.includes(`property="og:url" content="${PAGES_DEMO_URL}"`), `og:url must be ${PAGES_DEMO_URL}`);
const ogImage = html.match(/property="og:image" content="([^"]+)"/)?.[1] ?? "";
check(ogImage.startsWith(PAGES_DEMO_URL), `og:image must be an absolute URL under ${PAGES_DEMO_URL}: ${ogImage}`);
const ogFile = ogImage.slice(PAGES_DEMO_URL.length);
if (/^[\w.-]+\.png$/.test(ogFile) && existsSync(join(demoRoot, ogFile))) {
  const png = readFileSync(join(demoRoot, ogFile));
  const isPng = png.subarray(0, 8).equals(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
  check(isPng && png.readUInt32BE(16) === 1200 && png.readUInt32BE(20) === 630, `${ogFile} must be a 1200x630 PNG`);
  check(bundled.has(ogFile), `${ogFile} is the og:image but not bundled by .github/workflows/docs-site.yml`);
  check(
    docsWorkflow
      .split(/\r?\n/)
      .some((line) => line.includes("sha256sum --binary") && line.split(/\s+/).includes(ogFile)),
    `${ogFile} is bundled but not checksummed by .github/workflows/docs-site.yml`,
  );
} else {
  check(false, `og:image file missing or not a local .png: ${ogFile || "(none)"} (render it with demo/scripts/og-image.mjs)`);
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

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`demo static check failed: ${failure}`);
  }
  // Let pending handles close; process.exit() here trips a libuv assertion on Windows.
  process.exitCode = 1;
} else {
  console.log("demo static checks passed");
}
