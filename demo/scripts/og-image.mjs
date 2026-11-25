// Renders demo/og-image.png, the 1200x630 link-preview image named by the
// og:image / twitter:image tags in index.html, from the live page itself.
// Start the dev server first (npm --prefix demo run serve), then:
//   node demo/scripts/og-image.mjs
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { chromium } from "@playwright/test";

const demoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";
const output = join(demoRoot, "og-image.png");

const browser = await chromium.launch();
try {
  const page = await browser.newPage({ viewport: { width: 1200, height: 630 }, deviceScaleFactor: 1 });
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "dark" });
  // A settled cluster with an elected leader; no first-visit intro in the shot.
  await page.addInitScript(() => {
    try {
      window.localStorage.setItem("hydracache-demo-intro-seen", "1");
      // A folded legend keeps the preview about the cluster.
      window.localStorage.setItem("hydracache-demo-legend", "closed");
    } catch {
      // Storage blocked: the intro may show, which is still a valid preview.
    }
  });
  await page.goto(`${demoUrl}?seed=502&steps=12&scenario=default`);
  await page.getByTestId("verdict").filter({ hasText: /invariants hold|violation/ }).waitFor();
  // The preview is about the cluster: drop the control strip so the graph
  // sits right under the title, then let the layout re-fit and settle.
  await page.addStyleTag({ content: ".controls { display: none !important; }" });
  await page.evaluate(() => window.dispatchEvent(new Event("resize")));
  await page.waitForTimeout(1500);
  await page.screenshot({ path: output, animations: "disabled", caret: "hide" });
  console.log(`wrote ${output}`);
} finally {
  await browser.close();
}
