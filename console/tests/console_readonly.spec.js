import { expect, test } from "@playwright/test";

import {
  consensusEnvelope,
  dashboardEnvelope,
  formationEnvelope,
  largeDashboardEnvelope,
  membersEnvelope,
  modeledDashboardEnvelope,
  partitionsEnvelope,
  placementTraceEnvelope,
  recoveryEnvelope,
} from "./fixtures.js";

const consoleUrl = process.env.HYDRACACHE_CONSOLE_URL ?? "http://127.0.0.1:5174/console/";

test("console_renders_typed_dashboard_and_all_truth_states", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);

  await expect(page.getByTestId("source-badge")).toHaveText("live");
  await expect(page.getByTestId("cluster-state")).toContainText("active");
  await expect(page.getByTestId("leader")).toContainText("node-opaque-2");
  await expect(page.getByTestId("partition-summary")).toContainText("under-replicated 2");
  await expect(page.getByTestId("metrics-strip")).toContainText("87.5%");
  await expect(page.getByTestId("metrics-strip")).toContainText("unavailable");
  await expect(page.getByTestId("formation-row")).toHaveCount(13);
  await expect(page.getByTestId("formation-table")).toContainText("cluster-identity-mismatch");
  await expect(page.getByTestId("consensus-table")).toContainText("unavailable");
  await expect(page.getByTestId("recovery-outcomes")).toContainText("corrupt");
  await expect(page.getByTestId("recovery-outcomes")).toContainText("failed");
  await expect(page.getByTestId("recovery-table")).toContainText("1000000+");
  await expect(page.getByTestId("truth-warnings")).toContainText("partial-observation");
  await expect(page.getByTestId("placement-state")).toHaveText("committed");
  await expect(page.getByTestId("placement-details")).toContainText("96");
  await expect(page.getByTestId("partition-row")).toHaveCount(2);
  await expect(page.getByTestId("members-list")).toContainText("sha256-v1:abc123");
  await expect(page.getByTestId("placement-row")).toHaveCount(2);
  await expect(page.getByTestId("placement-row").first()).toContainText("selected");
  await expect(page.getByTestId("placement-table")).toContainText("zone-conflict");
});

test("placement_outcomes_and_stale_warning_are_rendered_without_inference", async ({ page }) => {
  for (const outcome of ["rejected", "proposed", "committed", "applied", "stale", null]) {
    const fixture = structuredClone(dashboardEnvelope);
    fixture.warnings = [{ code: "stale-observation", affected_count: 1 }];
    fixture.data.placement = {
      outcome,
      selected: outcome == null ? null : 2,
      rejected: outcome == null ? null : 3,
      latest_committed_epoch: outcome == null ? null : 42,
      latest_applied_epoch: outcome === "applied" ? 42 : null,
    };
    await page.unrouteAll({ behavior: "wait" });
    await routeManagement(page, { dashboard: fixture, placementTrace: null });
    await page.goto(consoleUrl);
    await expect(page.getByTestId("placement-state")).toHaveText(outcome ?? "unavailable");
    await expect(page.getByTestId("truth-warnings")).toContainText("stale-observation");
  }
});

test("summary_links_open_read_only_filtered_sections", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  await page.getByTestId("formation-summary").click();
  await expect(page).toHaveURL(/#formation$/);
  await expect(page.locator("#formation")).toBeVisible();
  await page.getByTestId("recovery-summary").click();
  await expect(page).toHaveURL(/#recovery$/);
});

test("console_is_read_only_and_never_scrapes_prometheus", async ({ page }) => {
  let metricsRequests = 0;
  await page.route("**/metrics", (route) => {
    metricsRequests += 1;
    return route.abort("failed");
  });
  await routeManagement(page);
  await page.goto(consoleUrl);

  await expect(page.getByTestId("readonly-badge")).toHaveText(/read only/i);
  await expect(page.locator("button")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /drain|reshard|backup|delete|remove/i })).toHaveCount(0);
  expect(metricsRequests).toBe(0);
});

test("hostile_diagnostic_text_is_rendered_only_as_text", async ({ page }) => {
  const hostileMembers = structuredClone(membersEnvelope);
  hostileMembers.data.items[0].config_digest = '<img src=x onerror="window.__xss=1">';
  const hostileTrace = structuredClone(placementTraceEnvelope);
  hostileTrace.data.candidates.items[1].reasons = ['<script>window.__xss=1</script>'];
  await routeManagement(page, { members: hostileMembers, placementTrace: hostileTrace });
  await page.goto(consoleUrl);
  await expect(page.getByTestId("members-list")).toContainText("<img src=x");
  await expect(page.getByTestId("placement-table")).toContainText("<script>");
  expect(await page.evaluate(() => window.__xss)).toBeUndefined();
  await expect(page.locator("#members img, #placement script")).toHaveCount(0);
});

test("modeled_source_and_missing_raft_values_are_never_painted_live_or_zero", async ({ page }) => {
  await routeManagement(page, { dashboard: modeledDashboardEnvelope, consensus: null });
  await page.goto(consoleUrl);

  await expect(page.getByTestId("source-badge")).toHaveText("modeled");
  await expect(page.getByTestId("leader")).toContainText("electing");
  await expect(page.getByTestId("consensus-summary")).toContainText("unavailable");
  await expect(page.getByTestId("member")).toHaveCount(0);
});

test("console_shows_degraded_state_when_required_snapshot_is_unreachable", async ({ page }) => {
  await page.route("**/management/v1/**", (route) => route.abort("failed"));
  await page.goto(consoleUrl);
  await expect(page.getByTestId("source-badge")).toHaveText("unavailable");
  await expect(page.getByTestId("degraded-state")).toContainText("Cannot reach cluster");
  await expect(page.getByTestId("leader")).toContainText("unavailable");
});

test("console_render_is_bounded_for_large_clusters", async ({ page }) => {
  await routeManagement(page, { dashboard: largeDashboardEnvelope(120) });
  await page.goto(consoleUrl);
  await expect(page.getByTestId("member")).toHaveCount(48);
  await expect(page.getByTestId("render-cap")).toContainText("48 rendered, 72 not rendered");
});

test("offline_and_hidden_lifecycle_pauses_polling_and_aborts_obsolete_work", async ({ page, context }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  await expect(page.getByTestId("source-badge")).toHaveText("live");

  await context.setOffline(true);
  await page.evaluate(() => window.dispatchEvent(new Event("offline")));
  await expect(page.getByTestId("poll-state")).toContainText("paused while offline");
  expect(await page.evaluate(() => window.__HC_CONSOLE_STATE__.state.paused)).toBe(true);
  await context.setOffline(false);
  await page.evaluate(() => window.dispatchEvent(new Event("online")));
  await expect(page.getByTestId("source-badge")).toHaveText("live");

  await page.evaluate(() => {
    Object.defineProperty(document, "hidden", { configurable: true, value: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect(page.getByTestId("poll-state")).toContainText("paused while hidden");
});

test("accelerated_browser_soak_respects_frozen_history_budget", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  const budget = await page.evaluate(() => {
    const { history, limits } = window.__HC_CONSOLE_STATE__;
    const base = {
      authority_epoch: 42,
      replication: { success_total: 0, failure_total: 0, backpressure_total: 0, under_replicated: 0, repair_debt: 0, zone_underspread: 0 },
      reshard: { moves_inflight: 0, backfill_lag: 0 },
      cache: { entries: 0, hits_total: 0, misses_total: 0, loads_total: 0, admission_queue_depth: 0, admission_rejected_total: 0 },
      consensus: { apply_lag: 0 }, members: [],
    };
    for (let minute = 0; minute < 720; minute += 1) history.ingest({ ...base, cache: { ...base.cache, entries: minute % 50, hits_total: minute } }, minute * 60_000);
    return { snapshot: history.snapshot(), limits };
  });
  expect(budget.snapshot.totalPoints).toBeLessThanOrEqual(budget.limits.maxTotalPoints);
  expect(budget.snapshot.byteSize).toBeLessThanOrEqual(budget.limits.maxBytes);
  await expect(page.getByTestId("history-budget")).toBeVisible();
});

test("responsive dashboard remains usable on configured viewport", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  await expect(page.locator("#dashboard")).toBeVisible();
  await expect(page.locator("#members")).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
});

async function routeManagement(page, overrides = {}) {
  const fixtures = {
    dashboard: overrides.dashboard ?? dashboardEnvelope,
    formation: overrides.formation === undefined ? formationEnvelope : overrides.formation,
    consensus: overrides.consensus === undefined ? consensusEnvelope : overrides.consensus,
    recovery: overrides.recovery === undefined ? recoveryEnvelope : overrides.recovery,
    members:
      overrides.members === undefined
        ? overrides.dashboard
          ? {
              ...membersEnvelope,
              source: overrides.dashboard.source,
              data: { ...membersEnvelope.data, items: overrides.dashboard.data.members ?? [] },
            }
          : membersEnvelope
        : overrides.members,
    partitions: overrides.partitions === undefined ? partitionsEnvelope : overrides.partitions,
    placementTrace:
      overrides.placementTrace === undefined ? placementTraceEnvelope : overrides.placementTrace,
  };
  await page.route("**/management/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    const fixture = path.includes("/placement-traces/")
      ? fixtures.placementTrace
      : path.endsWith("/dashboard")
      ? fixtures.dashboard
      : path.endsWith("/cluster/members")
        ? fixtures.members
        : path.endsWith("/cluster/partitions")
          ? fixtures.partitions
      : path.endsWith("/formation")
        ? fixtures.formation
        : path.endsWith("/consensus/progress")
          ? fixtures.consensus
          : fixtures.recovery;
    if (fixture == null) return route.fulfill({ status: 503, json: { code: "unavailable" } });
    return route.fulfill({ status: 200, json: fixture });
  });
}
