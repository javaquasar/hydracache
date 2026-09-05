import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

import {
  auditEnvelope,
  capabilitiesEnvelope,
  consensusEnvelope,
  clientsEnvelope,
  dashboardEnvelope,
  formationEnvelope,
  healthEnvelope,
  historyEnvelope,
  largeDashboardEnvelope,
  membersEnvelope,
  modeledDashboardEnvelope,
  namespaceCachesEnvelope,
  namespacesEnvelope,
  operationsEnvelope,
  partitionsEnvelope,
  persistenceEnvelope,
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
  await expect(page.getByTestId("members-list")).toContainText("0.72.0");
  await expect(page.getByTestId("members-list")).toContainText("27");
  await page.getByTestId("member-detail").first().locator("summary").click();
  await expect(page.getByTestId("member-detail").first()).toContainText("Authority epoch");
  await expect(page.getByTestId("member-detail").first()).toContainText("seq 9 · epoch 42 · serving · none");
  await expect(page.getByTestId("placement-row")).toHaveCount(2);
  await expect(page.getByTestId("placement-row").first()).toContainText("selected");
  await expect(page.getByTestId("placement-table")).toContainText("zone-conflict");
  await expect(page.getByTestId("client-protocol-row")).toHaveCount(3);
  await expect(page.getByTestId("client-table")).toContainText("hc-2-alpha");
  await expect(page.getByTestId("client-details")).toContainText("unavailable");
  await expect(page.getByTestId("client-details")).toContainText("reconnecting2");
  await expect(page.getByTestId("client-details")).toContainText("slow1");
  await expect(page.getByTestId("client-details")).toContainText("quota rejected7");
  await expect(page.getByTestId("client-table")).toContainText("2.0 KiB");
  await expect(page.getByTestId("namespace-row")).toContainText("orders");
  await expect(page.getByTestId("namespace-row")).toContainText("unavailable");
  await expect(page.getByTestId("namespace-row")).toContainText("120 / 1,000");
  await expect(page.getByTestId("namespace-row")).toContainText("120 / 500");
  await expect(page.getByTestId("cache-row")).toContainText("client-surface");
  await expect(page.getByTestId("cache-row")).toContainText("80 / 20 / 12");
  await expect(page.getByTestId("cache-row")).toContainText("512 B");
  await expect(page.getByTestId("cache-row")).toContainText("exact");
  await expect(page.getByTestId("health-row")).toHaveCount(18);
  await expect(page.getByTestId("health-counts")).toContainText("UNKNOWN15");
  await expect(page.getByTestId("health-aggregate")).toHaveText("FAIL");
  await expect(page.getByTestId("health-table")).toContainText("apply-lag=100 entries");
  await expect(page.getByTestId("remote-history-state")).toContainText("using browser-local history");
  await expect(page.getByTestId("persistence-details")).toContainText("verified backupunavailable");
  await expect(page.getByTestId("operations-table")).toContainText("accepted");
  await expect(page.getByTestId("operations-table")).toContainText("completed");
  await expect(page.getByTestId("audit-table")).toContainText("runtime_journal");
});

test("optional_prometheus_history_is_labeled_and_never_spliced_into_local_ring", async ({ page }) => {
  const remote = structuredClone(historyEnvelope);
  remote.source = "live";
  remote.data.state = "available";
  remote.data.series = [{ series_index: 0, points: [{ timestamp_unix_ms: 1, value: 999 }] }];
  await routeManagement(page, { history: remote });
  await page.goto(consoleUrl);
  await expect(page.getByTestId("remote-history-state")).toContainText("kept separate");
  const local = await page.evaluate(() => window.__HC_CONSOLE_STATE__.history.points("cache.entries"));
  expect(local.some((point) => point.value === 999)).toBe(false);
});

test("health_filters_use_server_verdicts_and_keep_unknown_visible", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  await page.getByTestId("health-status-filter").selectOption("UNKNOWN");
  await expect(page.getByTestId("health-row")).toHaveCount(15);
  await page.getByTestId("health-category-filter").selectOption("recovery");
  await expect(page.getByTestId("health-row")).toHaveCount(2);
  await page.getByTestId("health-search").fill("reconciliation");
  await expect(page.getByTestId("health-row")).toHaveCount(1);
  await expect(page.getByTestId("health-row")).toContainText("HC-RECOVERY-003");
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
  await expect(page.locator("#recovery")).toBeVisible();
  await expect(page.locator("#members")).toBeHidden();
  await page.reload();
  await expect(page).toHaveURL(/#recovery$/);
  await expect(page.locator("nav a[href='#recovery']")).toHaveAttribute("aria-current", "page");
});

test("capabilities hide unavailable views and suppress their request loop", async ({ page }) => {
  const capabilities = structuredClone(capabilitiesEnvelope);
  capabilities.data.capabilities = capabilities.data.capabilities.filter(
    (item) => item.id !== "consensus_progress",
  );
  capabilities.data.capabilities.find((item) => item.id === "persistence_recovery").availability = "unavailable";
  capabilities.data.capabilities.find((item) => item.id === "persistence_recovery").reason = "status-not-retained";
  const requestPaths = [];
  await routeManagement(page, { capabilities, requestPaths });
  await page.goto(consoleUrl);

  await expect(page.locator("nav a[href='#consensus']")).toBeHidden();
  await expect(page.locator("#consensus")).toBeHidden();
  await expect(page.locator("#recovery")).toBeHidden();
  await expect(page.getByTestId("capability-notices")).toContainText("consensus: unavailable (capability-not-advertised)");
  await expect(page.getByTestId("capability-notices")).toContainText("recovery: unavailable (status-not-retained)");
  expect(requestPaths).not.toContain("/management/v1/consensus/progress");
  expect(requestPaths).not.toContain("/management/v1/persistence/recovery");
});

test("console_is_read_only_and_never_scrapes_prometheus", async ({ page }) => {
  let metricsRequests = 0;
  const managementMethods = [];
  const managementHeaders = [];
  await page.route("**/metrics", (route) => {
    metricsRequests += 1;
    return route.abort("failed");
  });
  await routeManagement(page, { requestMethods: managementMethods, requestHeaders: managementHeaders });
  await page.goto(consoleUrl);

  await expect(page.getByTestId("readonly-badge")).toHaveText(/read only/i);
  await expect(page.locator("button")).toHaveCount(0);
  await expect(page.getByRole("button", { name: /drain|reshard|backup|delete|remove/i })).toHaveCount(0);
  expect(metricsRequests).toBe(0);
  expect([...new Set(managementMethods)]).toEqual(["GET"]);
  expect(managementHeaders.every((headers) => headers["x-hydracache-management-read"] === "true")).toBe(true);
  expect(managementHeaders.every((headers) => headers["x-hydracache-admin"] == null)).toBe(true);
});

test("hostile_diagnostic_text_is_rendered_only_as_text", async ({ page }) => {
  const hostileMembers = structuredClone(membersEnvelope);
  hostileMembers.data.items[0].config_digest = '<img src=x onerror="window.__xss=1">';
  const hostileTrace = structuredClone(placementTraceEnvelope);
  hostileTrace.data.candidates.items[1].reasons = ['<script>window.__xss=1</script>'];
  const hostileAudit = structuredClone(auditEnvelope);
  hostileAudit.data.items.items[0].action = '<img src=x onerror="window.__xss=1">';
  await routeManagement(page, { members: hostileMembers, placementTrace: hostileTrace, audit: hostileAudit });
  await page.goto(consoleUrl);
  await expect(page.getByTestId("members-list")).toContainText("<img src=x");
  await expect(page.getByTestId("placement-table")).toContainText("<script>");
  await expect(page.getByTestId("audit-table")).toContainText("<img src=x");
  expect(await page.evaluate(() => window.__xss)).toBeUndefined();
  await expect(page.locator("#members img, #placement script, #audit img")).toHaveCount(0);
});

test("management center passes automated accessibility checks in normal and forced modes", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  const results = await new AxeBuilder({ page }).analyze();
  expect(results.violations).toEqual([]);
  await page.emulateMedia({ reducedMotion: "reduce", forcedColors: "active" });
  await expect(page.getByTestId("health-aggregate")).toHaveText("FAIL");
  await expect(page.getByTestId("operations-table")).toContainText("accepted");
  const motion = await page.evaluate(() => getComputedStyle(document.documentElement).scrollBehavior);
  expect(motion).toBe("auto");
});

test("keyboard navigation exposes a visible focus path through management sections", async ({ page }) => {
  await routeManagement(page);
  await page.goto(consoleUrl);
  await page.keyboard.press("Tab");
  const focused = page.locator(":focus-visible");
  await expect(focused).toBeVisible();
  const outline = await focused.evaluate((element) => getComputedStyle(element).outlineStyle);
  expect(outline).not.toBe("none");
  for (let step = 0; step < 20 && (await page.locator(":focus").getAttribute("href")) !== "#operations"; step += 1) {
    await page.keyboard.press("Tab");
  }
  await expect(page.locator(":focus")).toHaveAttribute("href", "#operations");
  await page.keyboard.press("Enter");
  await expect(page).toHaveURL(/#operations$/);
});

test("member formation detail is keyboard operable and keeps current generation evidence", async ({ page }) => {
  await routeManagement(page);
  await page.goto(`${consoleUrl}#members`);
  const summary = page.getByTestId("member-detail").first().locator("summary");
  await summary.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("member-detail").first()).toHaveAttribute("open", "");
  await expect(page.getByTestId("member-detail").first().getByRole("list", { name: "Current generation formation timeline" })).toBeVisible();
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

test("tablet layout retains truth at effective 200 percent zoom without page overflow", async ({ page }) => {
  // Browser zoom halves the CSS-pixel viewport. A 384px CSS viewport is the deterministic
  // cross-browser equivalent of a 768px tablet at 200% zoom.
  await page.setViewportSize({ width: 384, height: 512 });
  await routeManagement(page);
  await page.goto(consoleUrl);
  await expect(page.getByTestId("source-badge")).toHaveText("live");
  await expect(page.getByTestId("health-aggregate")).toHaveText("FAIL");
  await expect(page.getByTestId("operations-table")).toContainText("accepted");
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
});

async function routeManagement(page, overrides = {}) {
  const fixtures = {
    capabilities: overrides.capabilities ?? capabilitiesEnvelope,
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
    clients: overrides.clients === undefined ? clientsEnvelope : overrides.clients,
    namespaces: overrides.namespaces === undefined ? namespacesEnvelope : overrides.namespaces,
    health: overrides.health === undefined ? healthEnvelope : overrides.health,
    history: overrides.history === undefined ? historyEnvelope : overrides.history,
    persistence: overrides.persistence === undefined ? persistenceEnvelope : overrides.persistence,
    operations: overrides.operations === undefined ? operationsEnvelope : overrides.operations,
    audit: overrides.audit === undefined ? auditEnvelope : overrides.audit,
    namespaceCaches:
      overrides.namespaceCaches === undefined
        ? namespaceCachesEnvelope
        : overrides.namespaceCaches,
  };
  await page.route("**/management/v1/**", (route) => {
    const path = new URL(route.request().url()).pathname;
    overrides.requestPaths?.push(path);
    overrides.requestMethods?.push(route.request().method());
    overrides.requestHeaders?.push(route.request().headers());
    const fixture = path.endsWith("/capabilities")
      ? fixtures.capabilities
      : path.includes("/placement-traces/")
      ? fixtures.placementTrace
      : /\/namespaces\/[^/]+\/caches$/.test(path)
        ? fixtures.namespaceCaches
      : path.endsWith("/dashboard")
      ? fixtures.dashboard
      : path.endsWith("/namespaces")
        ? fixtures.namespaces
      : path.endsWith("/healthchecks")
        ? fixtures.health
      : path.endsWith("/history")
        ? fixtures.history
      : path.endsWith("/persistence")
        ? fixtures.persistence
      : path.endsWith("/operations")
        ? fixtures.operations
      : path.endsWith("/audit")
        ? fixtures.audit
      : path.endsWith("/clients")
        ? fixtures.clients
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
