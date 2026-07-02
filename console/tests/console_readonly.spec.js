import { expect, test } from "@playwright/test";

import {
  largeOverviewFixture,
  liveOverviewFixture,
  metricsFixture,
  modeledOverviewFixture,
  noLeaderFixture,
} from "./fixtures.js";

const consoleUrl = process.env.HYDRACACHE_CONSOLE_URL ?? "http://127.0.0.1:5174/console/";
const maxRenderedMembers = 48;

test("console_renders_live_cluster_overview_from_endpoints", async ({ page }) => {
  await routeOverview(page, liveOverviewFixture);
  await routeMetrics(page, metricsFixture);

  await page.goto(consoleUrl);

  await expect(page.getByTestId("source-badge")).toHaveText("live");
  await expect(page.getByTestId("member")).toHaveCount(3);
  await expect(page.getByTestId("leader")).toContainText("node-2");
  await expect(page.getByTestId("partition-summary")).toContainText("64");
  await expect(page.getByTestId("backup-age")).toContainText("2m");
  await expect(page.getByTestId("consistency-panel")).toContainText("quorum");
  await expect(page.getByTestId("metrics-strip")).toContainText("87.5%");
  await expect(page.getByTestId("topology-graph")).toBeVisible();
});

test("console_is_read_only_no_mutate_controls", async ({ page }) => {
  await routeOverview(page, liveOverviewFixture);
  await routeMetrics(page, metricsFixture);

  await page.goto(consoleUrl);

  await expect(page.getByTestId("readonly-badge")).toHaveText(/read only/i);
  await expect(page.locator("button")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /drain|reshard|backup|delete|remove/i })).toHaveCount(
    0,
  );
});

test("modeled_source_is_shown_as_modeled_never_live", async ({ page }) => {
  await routeOverview(page, modeledOverviewFixture);
  await routeMetrics(page, "");

  await page.goto(consoleUrl);

  await expect(page.getByTestId("source-badge")).toHaveText("modeled");
  await expect(page.getByTestId("source-badge")).not.toHaveText(/live/i);
  await expect(page.getByTestId("leader")).toContainText("electing");
  await expect(page.getByTestId("member")).toHaveCount(0);
});

test("console_shows_degraded_state_when_server_unreachable", async ({ page }) => {
  await page.route("**/cluster/overview", (route) => route.abort("failed"));
  await page.route("**/metrics", (route) => route.abort("failed"));

  await page.goto(consoleUrl);

  await expect(page.getByTestId("source-badge")).toHaveText("unreachable");
  await expect(page.getByTestId("degraded-state")).toContainText("Cannot reach cluster");
  await expect(page.getByTestId("leader")).toContainText("unavailable");
  await expect(page.getByTestId("member")).toHaveCount(0);
});

test("console_render_is_bounded_for_large_clusters", async ({ page }) => {
  await routeOverview(page, largeOverviewFixture(120));
  await routeMetrics(page, metricsFixture);

  await page.goto(consoleUrl);

  await expect(page.getByTestId("member")).toHaveCount(maxRenderedMembers);
  await expect(page.getByTestId("render-cap")).toContainText("48 rendered, 72 not rendered");
  expect(await page.locator(".graph-node").count()).toBeLessThanOrEqual(maxRenderedMembers);
});

test("console_renders_one_node_without_leader_as_electing", async ({ page }) => {
  await routeOverview(page, noLeaderFixture);
  await routeMetrics(page, metricsFixture);

  await page.goto(consoleUrl);

  await expect(page.getByTestId("member")).toHaveCount(1);
  await expect(page.getByTestId("leader")).toContainText("electing");
  await expect(page.getByTestId("source-badge")).toHaveText("live");
});

async function routeOverview(page, overview) {
  await page.route("**/cluster/overview", (route) => route.fulfill({ json: overview }));
}

async function routeMetrics(page, metrics) {
  await page.route("**/metrics", (route) =>
    route.fulfill({
      status: 200,
      contentType: "text/plain; version=0.0.4",
      body: metrics,
    }),
  );
}
