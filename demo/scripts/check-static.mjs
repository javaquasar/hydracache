import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const demoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const required = [
  "index.html",
  "app.js",
  "style.css",
  "tests/ui_smoke.spec.js",
  "playwright.config.mjs"
];

for (const file of required) {
  readFileSync(join(demoRoot, file), "utf8");
}

const css = readFileSync(join(demoRoot, "style.css"), "utf8");
const spec = readFileSync(join(demoRoot, "tests/ui_smoke.spec.js"), "utf8");

for (const marker of [
  "--glass",
  "backdrop-filter",
  "prefers-reduced-motion",
  "prefers-reduced-transparency",
  ".packet-trail"
]) {
  if (!css.includes(marker)) {
    throw new Error(`missing glass CSS marker: ${marker}`);
  }
}

for (const testName of [
  "glass_theme_renders_and_controls_remain_operable",
  "reduced_motion_and_transparency_fallbacks_apply"
]) {
  if (!spec.includes(testName)) {
    throw new Error(`missing W6 UI smoke: ${testName}`);
  }
}

console.log("demo static checks passed");
