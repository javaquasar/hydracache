import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

import { stubViolatingServer, violatingServerUrl } from "./support/violation-stub.js";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

// WCAG 2.2 A and AA rules. Any violation fails the test, and the message lists
// each rule with the selectors it hit, so the report says what to fix.
async function expectNoAxeViolations(page, label) {
  // Check the settled page: a colour mid-transition (theme switch, change
  // flash) is not what a reader sees and gives false contrast failures.
  // Capped, because a paused or restarted animation may never finish.
  await page.evaluate(() =>
    Promise.race([
      Promise.all(
        document
          .getAnimations()
          .filter((animation) => Number.isFinite(animation.effect?.getComputedTiming().endTime))
          .map((animation) => animation.finished.catch(() => undefined)),
      ),
      new Promise((resolve) => setTimeout(resolve, 3000)),
    ]),
  );
  const results = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  const summary = results.violations.map(
    (violation) =>
      `${violation.id} (${violation.impact}): ${violation.help}\n` +
      violation.nodes.map((node) => `    ${node.target.join(" ")}`).join("\n"),
  );
  expect(summary, `axe violations in ${label}`).toEqual([]);
}

async function openReady(page) {
  await page.goto(demoUrl);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
}

for (const tab of ["cluster", "clients", "data", "verdict"]) {
  test(`a11y_${tab}_tab_has_no_axe_violations`, async ({ page }) => {
    await openReady(page);
    await page.getByTestId("step").click();
    await page.getByTestId(`tab-${tab}`).click();
    await expect(page.getByTestId(`tab-${tab}`)).toHaveAttribute("aria-selected", "true");
    await expectNoAxeViolations(page, `${tab} tab`);
  });
}

test("a11y_dark_theme_has_no_axe_violations", async ({ page }) => {
  await openReady(page);
  const html = page.locator("html");
  if ((await html.getAttribute("data-theme")) !== "dark") {
    await page.getByTestId("theme-toggle").click();
  }
  await expect(html).toHaveAttribute("data-theme", "dark");
  for (const tab of ["cluster", "data"]) {
    await page.getByTestId(`tab-${tab}`).click();
    await expectNoAxeViolations(page, `dark theme, ${tab} tab`);
  }
});

test("a11y_node_popover_has_no_axe_violations", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("until-leader").click();
  await page.locator(".vertex.node").first().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("node-popover")).toBeVisible();
  await expectNoAxeViolations(page, "node popover");
});

test("a11y_shortcuts_dialog_has_no_axe_violations", async ({ page }) => {
  await openReady(page);
  await page.evaluate(() => document.activeElement?.blur());
  await page.keyboard.press("?");
  await expect(page.getByTestId("shortcuts-dialog")).toBeVisible();
  await expectNoAxeViolations(page, "shortcuts dialog");
});

test("a11y_violation_card_has_no_axe_violations", async ({ page }) => {
  await stubViolatingServer(page, demoUrl);
  await page.goto(violatingServerUrl(demoUrl));
  await expect(page.getByTestId("verdict")).toContainText("invariants hold");
  await page.getByTestId("step").click();
  await expect(page.getByTestId("violation-card")).toBeVisible();
  await expectNoAxeViolations(page, "violation card");
});
