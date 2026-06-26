import { expect, test } from "@playwright/test";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

test("loads_steps_and_renders_verdict", async ({ page }) => {
  await page.goto(demoUrl);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(page.getByTestId("engine-banner")).toContainText("election sim-model");
  await expect(page.getByTestId("signals-panel")).toBeVisible();

  const before = await page.getByTestId("progress-panel").textContent();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("progress-panel")).not.toHaveText(before ?? "");
});

test("clicking_partition_updates_link_state", async ({ page }) => {
  await page.goto(demoUrl);
  await page.locator(".link-hit").first().click();
  await page.getByTestId("partition-link").click();

  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
});

test("loading_scenario_uses_curated_engine_preset", async ({ page }) => {
  await page.goto(demoUrl);
  await page.getByTestId("scenario-select").selectOption("minority_partition_cannot_commit");
  await page.getByTestId("load-scenario").click();

  await expect(page.getByTestId("snapshot-hash")).toContainText("snapshot ");
  await expect(page).toHaveURL(/scenario=minority_partition_cannot_commit/);
});

test("manual_push_shows_diverge_converge_and_listener_receipt", async ({ page }) => {
  await page.goto(demoUrl);
  await page.getByTestId("subscribe-button").click();
  await page.getByTestId("push-event-button").click();

  await expect(page.getByTestId("clients-panel")).toContainText("client-a");
  await page.getByTestId("step").click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("subscribers-panel")).toContainText("upserted");
});

test("node_controls_show_reelection_resync_and_scale_out", async ({ page }) => {
  await page.goto(demoUrl);
  await page.getByText("Isolate").first().click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("engine-banner")).toContainText("Formation");
  await page.getByText("Rejoin").first().click();
  await page.getByTestId("add-node-button").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("node-3");
});
