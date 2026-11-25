import { existsSync } from "node:fs";

import { expect, test } from "@playwright/test";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

// Screenshot regressions for the panels whose look carries meaning (verdict,
// heatmap colours, node roles, theme). Baselines are per OS and per viewport,
// because fonts and anti-aliasing differ: Playwright names them
// <name>-<project>-<platform>.png next to this spec. A platform without
// committed baselines skips these tests instead of failing; record them with
//   npx playwright test tests/visual.spec.js --update-snapshots
// Only the graph-free panels are captured: the force layout is not pixel-stable.
async function expectScreenshot(page, locator, name) {
  const info = test.info();
  const baseline = info.snapshotPath(name);
  test.skip(
    info.config.updateSnapshots !== "all" && info.config.updateSnapshots !== "changed" && !existsSync(baseline),
    `no ${process.platform} baseline: ${baseline}`,
  );
  await expect(locator).toHaveScreenshot(name, { animations: "disabled", caret: "hide", maxDiffPixelRatio: 0.01 });
}

async function openStable(page) {
  await page.emulateMedia({ reducedMotion: "reduce", colorScheme: "light" });
  await page.goto(`${demoUrl}?seed=502&steps=6&scenario=default`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 6 of 6");
}

test("visual_nodes_panel_light", async ({ page }) => {
  await openStable(page);
  await expectScreenshot(page, page.getByTestId("nodes-panel"), "nodes-panel-light.png");
});

test("visual_keys_heatmap_light", async ({ page }) => {
  await openStable(page);
  await page.getByTestId("tab-data").click();
  await expectScreenshot(page, page.getByTestId("keys-panel"), "keys-panel-light.png");
});

test("visual_verdict_panel_light", async ({ page }) => {
  await openStable(page);
  await page.getByTestId("tab-verdict").click();
  await expectScreenshot(page, page.getByTestId("verdict-panel"), "verdict-panel-light.png");
});

test("visual_nodes_panel_dark", async ({ page }) => {
  await openStable(page);
  const html = page.locator("html");
  if ((await html.getAttribute("data-theme")) !== "dark") {
    await page.getByTestId("theme-toggle").click();
  }
  await expect(html).toHaveAttribute("data-theme", "dark");
  await expectScreenshot(page, page.getByTestId("nodes-panel"), "nodes-panel-dark.png");
});
