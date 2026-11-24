import { expect, test } from "@playwright/test";

import { stubViolatingServer, violatingServerUrl } from "./support/violation-stub.js";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

test("loads_steps_and_renders_verdict", async ({ page }) => {
  await openReady(page);
  await expect(page.getByTestId("engine-banner")).toContainText("election sim-model");
  await expandPanel(page, "signals-panel");
  await expect(page.getByTestId("signals-panel")).toBeVisible();

  const before = await page.getByTestId("progress-panel").textContent();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("progress-panel")).not.toHaveText(before ?? "");
});

test("election_source_chip_is_visible_and_labelled", async ({ page }) => {
  await openReady(page);
  const chip = page.getByTestId("election-source-chip");
  await expect(chip).toBeVisible();
  await expect(chip).toContainText(/sim-model|raft/);
  await expect(chip).toHaveAttribute("title", /consensus|simulator|raft/);
});

test("clicking_partition_updates_link_state", async ({ page }) => {
  await openReady(page);
  await selectFirstLink(page);
  await page.getByTestId("partition-link").click();

  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
});

test("loading_scenario_uses_curated_engine_preset", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("scenario-select").selectOption("minority_partition_cannot_commit");
  await expect(page.getByTestId("scenario-select")).toHaveValue("minority_partition_cannot_commit");
  await page.getByTestId("load-scenario").click();

  await expect(page.getByTestId("snapshot-hash")).toContainText("snapshot ");
  await expect(page).toHaveURL(/scenario=minority_partition_cannot_commit/);
});

test("manual_push_shows_diverge_converge_and_listener_receipt", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-clients").click();
  await page.getByTestId("subscribe-button").click();
  await page.getByTestId("push-event-button").click();

  await expect(page.getByTestId("clients-panel")).toContainText("client-a");
  await page.getByTestId("step").click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("subscribers-panel")).toContainText("upserted");
});

test("node_controls_show_reelection_resync_and_scale_out", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("nodes-panel").getByText("Isolate").first().click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("engine-banner")).toContainText("Formation");
  await page.getByTestId("nodes-panel").getByText("Rejoin").first().click();
  await page.getByTestId("add-node-button").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("node-3");
});

test("modes_switch_and_topology_is_clickable_in_each", async ({ page }) => {
  await openReady(page);
  for (const mode of ["manual", "scripted", "mixed"]) {
    await page.getByTestId("mode-select").selectOption(mode);
    await expect(page.getByTestId("intervention-status")).toContainText(mode);
    await page.getByTestId("nodes-panel").getByText("Isolate").first().click();
    await page.getByTestId("nodes-panel").getByText("Rejoin").first().click();
  }
});

test("glass_theme_renders_and_controls_remain_operable", async ({ page }, testInfo) => {
  await openReady(page);
  await expect(page.getByTestId("graph-legend")).toContainText("heartbeat");

  const bands = await page.locator(".topbar, .controls, .workspace").evaluateAll((nodes) =>
    nodes.map((node) => {
      const box = node.getBoundingClientRect();
      return { top: box.top, bottom: box.bottom };
    }),
  );
  expect(bands[0].bottom).toBeLessThanOrEqual(bands[1].top + 1);
  expect(bands[1].bottom).toBeLessThanOrEqual(bands[2].top + 1);

  await page.getByTestId("step").click();
  await selectFirstLink(page);
  await page.getByTestId("delay-link").click();
  await expect(page.getByTestId("selected-link")).toContainText(/delayed|none/);

  const contrast = await contrastRatio(page.getByTestId("verdict"));
  expect(contrast).toBeGreaterThanOrEqual(4.5);

  await page.screenshot({ path: testInfo.outputPath("glass-theme.png"), fullPage: true });
});

test("reduced_motion_and_transparency_fallbacks_apply", async ({ page }) => {
  await page.addInitScript(() => {
    const originalMatchMedia = window.matchMedia.bind(window);
    window.matchMedia = (query) => {
      if (
        query.includes("prefers-reduced-motion") ||
        query.includes("prefers-reduced-transparency")
      ) {
        return {
          matches: true,
          media: query,
          onchange: null,
          addEventListener() {},
          removeEventListener() {},
          addListener() {},
          removeListener() {},
          dispatchEvent() {
            return false;
          },
        };
      }
      return originalMatchMedia(query);
    };
  });

  await openReady(page);
  await expect(page.locator("html")).toHaveAttribute("data-reduced-motion", "true");
  await expect(page.locator("html")).toHaveAttribute("data-reduced-transparency", "true");

  await page.locator("#cluster-graph").evaluate((graph) => {
    const packet = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    packet.setAttribute("class", "packet");
    graph.append(packet);
  });

  const animation = await page.locator(".packet").last().evaluate((node) => {
    const style = getComputedStyle(node);
    return `${style.animationName}:${style.animationDuration}`;
  });
  expect(animation).toMatch(/none|0\.01ms/);

  const glassFallback = await page.locator(".topbar").evaluate((node) => {
    const style = getComputedStyle(node);
    return {
      background: style.backgroundColor,
      backdrop: style.backdropFilter || style.webkitBackdropFilter || "none",
    };
  });
  expect(alphaOf(glassFallback.background)).toBe(1);
  expect(glassFallback.backdrop).toBe("none");
});

test("consistency_panel_shows_product_level_availability", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-data").click();
  const panel = page.getByTestId("consistency-panel");
  await expect(panel).toContainText("does not run per-level reads or writes");
  await expect(panel).toContainText("live replicas agree on sampled keys");
  for (const level of ["one", "local_quorum", "quorum", "each_quorum", "all"]) {
    await expect(panel.locator(`[data-level='${level}']`)).toHaveClass(/ok/);
  }
  await expect(panel.locator("[data-level='quorum']")).toContainText("needs 2 of 3, 3 live");

  const crash = page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" });
  // Node controls live on the Cluster tab; the panel is read on the Data tab.
  await page.getByTestId("tab-cluster").click();
  await crash.nth(2).click();
  await page.getByTestId("tab-data").click();
  await expect(panel.locator("[data-level='all']")).toHaveClass(/bad/);
  await expect(panel.locator("[data-level='all']")).toContainText(
    "requires 3 live acknowledgements from 3 replicas, only 2 are live",
  );
  await expect(panel.locator("[data-level='quorum']")).toHaveClass(/ok/);

  await page.getByTestId("tab-cluster").click();
  await crash.nth(1).click();
  await page.getByTestId("tab-data").click();
  await expect(panel.locator("[data-level='quorum']")).toHaveClass(/bad/);
  await expect(panel.locator("[data-level='local_quorum']")).toContainText(
    "regional quorum unsatisfied: local needs 2, has 1",
  );
  await expect(panel.locator("[data-level='one']")).toHaveClass(/ok/);
});

test("engine_rejection_is_shown_to_the_user", async ({ page }) => {
  await openReady(page);
  await selectFirstLink(page);
  await page.getByTestId("heal-link").click();

  const notice = page.getByTestId("action-notice");
  await expect(notice).toBeVisible();
  await expect(notice).toContainText("not partitioned");

  await page.getByTestId("partition-link").click();
  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
  await expect(notice).toBeHidden();
});

test("invalid_seed_fails_loud_without_resetting", async ({ page }) => {
  await openReady(page);
  const hash = await page.getByTestId("snapshot-hash").textContent();

  await page.getByTestId("seed-input").fill("12abc");
  await expect(page.getByTestId("seed-error")).toBeVisible();
  await expect(page.getByTestId("seed-input")).toHaveAttribute("aria-invalid", "true");
  await page.getByTestId("reset").click();

  await expect(page.getByTestId("action-notice")).toContainText("Seed must be a whole number");
  await expect(page.getByTestId("snapshot-hash")).toHaveText(hash ?? "");

  await page.getByTestId("seed-input").fill("81");
  await expect(page.getByTestId("seed-error")).toBeHidden();
});

test("node_controls_stay_clickable_while_playing", async ({ page }) => {
  await openReady(page);
  await setFastestSpeed(page);
  const crash = page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first();
  const handle = await crash.elementHandle();

  await page.getByTestId("play").click();
  await expect(page.getByTestId("progress-panel")).not.toContainText(/^Step0/);
  await page.waitForTimeout(400);
  expect(await handle?.evaluate((node) => node.isConnected)).toBe(true);
  await crash.click();
  await expect(page.getByTestId("nodes-panel")).toContainText("crashed");
  await page.getByTestId("play").click();
  await expect(page.getByTestId("play")).toHaveText("Play");
});

test("playing_throttles_share_url_writes", async ({ page }) => {
  await page.addInitScript(() => {
    const original = window.history.replaceState.bind(window.history);
    window.__urlWrites = 0;
    window.history.replaceState = (...args) => {
      window.__urlWrites += 1;
      return original(...args);
    };
  });
  await openReady(page);
  await setFastestSpeed(page);
  const before = await page.evaluate(() => window.__urlWrites);

  await page.getByTestId("play").click();
  await page.waitForTimeout(2500);
  await page.getByTestId("play").click();
  await expect(page.getByTestId("play")).toHaveText("Play");

  const step = await page.evaluate(() => Number(new URL(window.location.href).searchParams.get("steps")));
  expect(step).toBeGreaterThan(10);
  const writes = (await page.evaluate(() => window.__urlWrites)) - before;
  expect(writes).toBeLessThanOrEqual(6);
  await expect
    .poll(() => page.evaluate(() => new URL(window.location.href).searchParams.get("steps")))
    .toBe(
      (await page.getByTestId("progress-panel").locator("dd").first().textContent())?.trim(),
    );
});

test("cancelled_pointer_on_client_does_not_push", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-clients").click();
  await page.getByTestId("subscribe-button").click();
  await expect(page.getByTestId("intervention-status")).toContainText("1 replay action");
  const client = page.locator(".vertex.client").first();
  const init = { pointerId: 7, isPrimary: true, bubbles: true, clientX: 0, clientY: 0 };

  await client.dispatchEvent("pointerdown", init);
  await client.dispatchEvent("pointercancel", init);
  await page.waitForTimeout(300);
  await expect(page.getByTestId("intervention-status")).toContainText("1 replay action");

  await client.dispatchEvent("pointerdown", init);
  await client.dispatchEvent("pointerup", init);
  await expect(page.getByTestId("intervention-status")).toContainText("2 replay action");
});

test("verdict_panel_lists_exactly_what_is_checked", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-verdict").click();
  const panel = page.getByTestId("verdict-panel");
  await expect(panel).toContainText("election_safety");
  await expect(panel).toContainText("read-your-writes");
  await expect(panel).toContainText("resource_bounded_growth");
  await expect(panel).toContainText("not in this live verdict");
});

test("keys_panel_shows_per_node_checksums", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=16&scenario=default`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  const keys = page.getByTestId("keys-panel");
  await expandPanel(page, "keys-panel");
  await expect(keys.locator(".replica").first()).toBeVisible();
  await expect(keys).toContainText("node-0");
  await expect(keys).toContainText("node-2");
  await expect(keys).toContainText(/agree|replicas differ/);
});

test("run_to_leader_elects_and_run_to_violation_fails_loud", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=90&steps=0&scenario=default`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);

  await page.getByTestId("until-leader").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("leader");
  await expect(page.getByTestId("action-notice")).toBeHidden();

  await page.getByTestId("until-violation").click();
  await expect(page.getByTestId("action-notice")).toContainText("no violation within 500 steps", {
    timeout: 30_000,
  });
});

test("guide_runs_the_default_tour", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=91&steps=0&scenario=default`);
  await expect(page.getByTestId("guide-title")).toContainText("1/6");
  await page.getByTestId("guide-do").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("leader");
  await expect(page.getByTestId("guide-title")).toContainText("2/6");
  await page.getByTestId("guide-back").click();
  await expect(page.getByTestId("guide-title")).toContainText("1/6");
});

test("workload_checkbox_follows_the_engine", async ({ page }) => {
  await page.goto(`${demoUrl}?scenario=each_quorum_region_loss_fails_loud`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(page.getByTestId("workload-toggle")).not.toBeChecked();
  await expect(page.getByTestId("guide-text")).toContainText("not a quorum");
});

test("keyboard_selects_and_partitions_a_link", async ({ page }) => {
  await openReady(page);
  const firstHit = page.locator(".link-hit").first();
  await firstHit.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("selected-link")).toContainText(/node-\d -> node-\d/);

  await page.getByTestId("link-select").selectOption("node-1>node-2");
  await expect(page.getByTestId("selected-link")).toContainText("node-1 -> node-2 (up)");
  await page.getByTestId("partition-link").focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
  // The drawn edge follows the engine state (it used to keep the state from
  // when the graph DOM was built).
  await expect(page.locator(".link.partitioned")).toHaveCount(1);
  await expect(page.locator(".link-hit[aria-label*='partitioned']")).toHaveCount(1);
});

test("keyboard_pushes_from_a_client_vertex", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-clients").click();
  await page.getByTestId("subscribe-button").click();
  await expect(page.getByTestId("intervention-status")).toContainText("1 replay action");
  const client = page.locator(".vertex.client").first();
  await expect(client).toHaveAttribute("role", "button");
  await client.focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("intervention-status")).toContainText("2 replay action");
});

test("graph_has_a_text_alternative", async ({ page }) => {
  await openReady(page);
  await expect(page.locator("#cluster-graph")).toHaveAttribute("aria-describedby", "graph-summary");
  const summary = page.getByTestId("graph-summary");
  await expect(summary).toContainText("3 nodes: node-0");
  await expect(summary).toContainText("Directed links: all up");
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(summary).toContainText("node-0 crashed");
});

test("controls_wait_for_the_engine_to_load", async ({ page }) => {
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  await page.route("**/pkg/hydracache_sim_wasm_bg.wasm", async (route) => {
    await gate;
    await route.continue();
  });
  await page.goto(demoUrl);
  await expect(page.getByTestId("demo-shell")).toHaveAttribute("aria-busy", "true");
  await expect(page.getByTestId("verdict")).toContainText("loading the simulator engine");
  await expect(page.getByTestId("step")).toBeDisabled();
  await expect(page.getByTestId("until-leader")).toBeDisabled();

  release();
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(page.getByTestId("demo-shell")).toHaveAttribute("aria-busy", "false");
  await expect(page.getByTestId("step")).toBeEnabled();
});

test("secondary_panels_collapse_on_small_screens", async ({ page }, testInfo) => {
  await openReady(page);
  const mobile = testInfo.project.name.startsWith("mobile");
  // Read-only panels in the default Cluster tab start collapsed on phones.
  const keysPanel = page.locator("details:has([data-testid='progress-panel'])");
  const nodesPanel = page.locator("details:has([data-testid='nodes-panel'])");
  await expect(nodesPanel).toHaveAttribute("open", "");
  if (mobile) {
    await expect(keysPanel).not.toHaveAttribute("open", "");
    await keysPanel.locator("summary").click();
    await expect(keysPanel).toHaveAttribute("open", "");
  } else {
    await expect(keysPanel).toHaveAttribute("open", "");
  }
});

async function contrastRatio(locator) {
  return locator.evaluate((node) => {
    const style = getComputedStyle(node);
    const foreground = parseColor(style.color);
    const background = parseColor(style.backgroundColor);
    return contrast(foreground, background);

    function parseColor(value) {
      const rgb = value.match(/rgba?\(([^)]+)\)/);
      if (rgb) {
        const [r, g, b] = rgb[1].split(",").slice(0, 3).map((part) => Number.parseFloat(part));
        return [r / 255, g / 255, b / 255];
      }
      const srgb = value.match(/color\(srgb\s+([^)]+)\)/);
      if (srgb) {
        return srgb[1].split(/\s+/).slice(0, 3).map((part) => Number.parseFloat(part));
      }
      {
        throw new Error(`unsupported color ${value}`);
      }
    }

    function contrast(a, b) {
      const lighter = Math.max(luminance(a), luminance(b));
      const darker = Math.min(luminance(a), luminance(b));
      return (lighter + 0.05) / (darker + 0.05);
    }

    function luminance(rgb) {
      return rgb
        .map((channel) =>
          channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4,
        )
        .reduce((total, channel, index) => total + channel * [0.2126, 0.7152, 0.0722][index], 0);
    }
  });
}

function alphaOf(color) {
  const match = color.match(/rgba?\(([^)]+)\)/);
  if (!match) {
    throw new Error(`unsupported color ${color}`);
  }
  const parts = match[1].split(",").map((part) => part.trim());
  return parts.length === 4 ? Number.parseFloat(parts[3]) : 1;
}

async function openReady(page) {
  await page.goto(demoUrl);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
}

test("visual_controls_theme_zoom_and_timeline_marks", async ({ page }) => {
  await openReady(page);
  const html = page.locator("html");

  // Theme toggle flips data-theme and survives a reload.
  const initial = await html.getAttribute("data-theme");
  expect(initial).toMatch(/light|dark/);
  await page.getByTestId("theme-toggle").click();
  const toggled = initial === "dark" ? "light" : "dark";
  await expect(html).toHaveAttribute("data-theme", toggled);
  await page.reload();
  await expect(html).toHaveAttribute("data-theme", toggled);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);

  // Speed shows a readable value.
  await expect(page.getByTestId("speed-value")).toHaveText("1.2 s/step");
  await setFastestSpeed(page);
  await expect(page.getByTestId("speed-value")).toHaveText("80 ms/step");

  // Zoom buttons change the view; Fit eases back to the automatic view.
  const viewport = page.locator(".graph-viewport");
  const scaleOf = async () =>
    Number((await viewport.getAttribute("transform"))?.match(/scale\(([\d.]+)\)/)?.[1]);
  await expect.poll(scaleOf).toBeGreaterThan(0);
  await page.waitForTimeout(400);
  const fitted = await scaleOf();
  // The fitted view follows the layout as it settles, so compare loosely.
  await page.getByTestId("zoom-in").click();
  const zoomed = await scaleOf();
  expect(zoomed).toBeGreaterThan(fitted * 1.1);
  await page.getByTestId("zoom-fit").click();
  await expect.poll(scaleOf).toBeLessThan(zoomed * 0.92);

  // A recorded action becomes a tick over the scrubber; a partitioned link is labelled.
  await selectFirstLink(page);
  await page.getByTestId("partition-link").click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("timeline-marks").locator(".timeline-mark.action")).toHaveCount(1);
  await expect(page.locator(".link-label.partitioned")).toHaveText("partitioned");
  await expect(page.locator(".node.role-leader .v-badge, .node .v-badge").first()).toBeAttached();

  // No horizontal scrolling at any viewport size.
  const overflow = await page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
  );
  expect(overflow).toBeLessThanOrEqual(0);
});

test("plain_wheel_over_graph_scrolls_the_page", async ({ page }) => {
  await openReady(page);
  const prevented = await page.locator("#cluster-graph").evaluate((graph) => {
    const event = new WheelEvent("wheel", { deltaY: 120, bubbles: true, cancelable: true });
    graph.dispatchEvent(event);
    return event.defaultPrevented;
  });
  expect(prevented).toBe(false);
  const zoomPrevented = await page.locator("#cluster-graph").evaluate((graph) => {
    const box = graph.getBoundingClientRect();
    const event = new WheelEvent("wheel", {
      deltaY: -120,
      ctrlKey: true,
      clientX: box.left + box.width / 2,
      clientY: box.top + box.height / 2,
      bubbles: true,
      cancelable: true,
    });
    graph.dispatchEvent(event);
    return event.defaultPrevented;
  });
  expect(zoomPrevented).toBe(true);
});

async function selectFirstLink(page) {
  await page.locator(".link-hit").first().dispatchEvent("click");
  await expect(page.getByTestId("selected-link")).toContainText(/node-/);
}

async function setFastestSpeed(page) {
  await page.locator("#speed-input").evaluate((input) => {
    input.value = input.min;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function expandPanel(page, testId) {
  await page.getByTestId(testId).evaluate((node) => {
    // Panels live in side-panel tabs; show the tab first.
    const tabpanel = node.closest("[role='tabpanel']");
    if (tabpanel?.hidden) {
      document.getElementById(tabpanel.getAttribute("aria-labelledby"))?.click();
    }
    const panel = node.closest("details");
    if (panel) {
      panel.open = true;
    }
  });
}

test("first_visit_intro_dismisses_and_reopens", async ({ page }) => {
  await openReady(page);
  const intro = page.getByTestId("intro");
  await expect(intro).toBeVisible();
  await intro.getByRole("button", { name: "Show me" }).nth(2).click();
  await expect(page.getByTestId("verdict")).toHaveClass(/spotlight/);
  await page.getByTestId("intro-dismiss").click();
  await expect(intro).toBeHidden();
  await page.reload();
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(intro).toBeHidden();
  await page.getByTestId("show-intro").click();
  await expect(intro).toBeVisible();
});

test("step_changes_are_listed_and_flash", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("until-leader").click();
  await expect(page.getByTestId("step-changes")).toContainText("leader");
  await expect(page.locator(".vertex.node.changed").first()).toBeAttached();
  await expandPanel(page, "nodes-panel");
  await expect(page.locator("#nodes-panel .node-row.changed").first()).toBeAttached();

  await selectFirstLink(page);
  await page.getByTestId("partition-link").click();
  await expect(page.getByTestId("step-changes")).toContainText("up → partitioned");
});

test("node_popover_shows_details_and_runs_node_actions", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("until-leader").click();
  const node = page.locator(".vertex.node").first();
  await node.focus();
  await page.keyboard.press("Enter");
  const popover = page.getByTestId("node-popover");
  await expect(popover).toBeVisible();
  await expect(popover).toContainText("commit index");
  await expect(popover).toContainText("term");
  await expect(page.getByTestId("node-popover-crash")).toBeFocused();

  await page.getByTestId("node-popover-crash").click();
  await expect(popover.locator(".popover-status")).toHaveText("crashed");
  await expect(page.getByTestId("node-popover-crash")).toHaveText("Restart");
  await expandPanel(page, "nodes-panel");
  await expect(page.getByTestId("nodes-panel")).toContainText("crashed");

  await page.keyboard.press("Escape");
  await expect(popover).toBeHidden();
});

// The real checker does not violate on ordinary seeds, so this drives the UI
// through the server-engine path against a stubbed /sim API whose snapshots
// are real WASM snapshots with a violated verdict spliced in.
test("violation_pauses_jumps_to_first_step_and_explains", async ({ page }) => {
  const { ids, controlTargets } = await stubViolatingServer(page, demoUrl);
  await page.goto(violatingServerUrl(demoUrl));
  await expect(page.getByTestId("verdict")).toContainText("invariants hold");
  await expect(page.getByTestId("violation-card")).toBeHidden();

  await setFastestSpeed(page);
  await page.getByTestId("play").click();
  const card = page.getByTestId("violation-card");
  await expect(card).toBeVisible();
  await expect(card).toContainText("Invariant violated: election_safety");
  await expect(page.getByTestId("violation-detail")).toContainText(`${ids[0]} and ${ids[2]}`);
  // Paused, and rewound to the first violating step reported by the engine.
  await expect(page.getByTestId("play")).toHaveText("Play");
  await expect.poll(() => controlTargets.at(-1)).toBe(9);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 9 of 12");
  await expect(page.getByTestId("violation-meta")).toContainText("First reported at step 9; this is that step.");
  await expect(page.getByTestId("violation-meta")).toContainText(`Highlighted on the graph: ${ids[0]}, ${ids[2]}`);
  await expect(page.locator(".vertex.node.implicated")).toHaveCount(2);
  await expect(page.getByTestId("violation-go-first")).toBeDisabled();
  await expect(page.getByTestId("timeline-marks").locator(".timeline-mark.verdict")).toHaveCount(1);

  await page.getByTestId("violation-copy").click();
  await expect(page.getByTestId("copy-status")).not.toBeEmpty();

  await page.getByTestId("violation-close").click();
  await expect(card).toBeHidden();
});

test("side_panel_tabs_switch_by_mouse_and_keyboard_and_show_badges", async ({ page }) => {
  await openReady(page);
  await expect(page.getByTestId("tab-cluster")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("nodes-panel")).toBeVisible();
  await expect(page.getByTestId("keys-panel")).toBeHidden();
  await expect(page.getByTestId("tab-verdict").locator(".tab-badge")).toHaveText("✓");

  await page.getByTestId("tab-data").click();
  await expect(page.getByTestId("tab-data")).toHaveAttribute("aria-selected", "true");
  await expect(page.getByTestId("consistency-panel")).toBeVisible();
  await expect(page.getByTestId("nodes-panel")).toBeHidden();

  await page.keyboard.press("ArrowRight");
  await expect(page.getByTestId("tab-verdict")).toBeFocused();
  await expect(page.getByTestId("verdict-panel")).toBeVisible();
  await page.keyboard.press("Home");
  await expect(page.getByTestId("tab-cluster")).toBeFocused();
  await expect(page.getByTestId("nodes-panel")).toBeVisible();

  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.getByTestId("tab-cluster").locator(".tab-badge")).toHaveText("1↓");
});

test("keys_heatmap_colours_checksums_and_marks_down_nodes", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=16&scenario=default`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await page.getByTestId("tab-data").click();
  const grid = page.getByTestId("keys-panel").locator(".key-grid");
  await expect(grid.locator("thead")).toContainText("node-0");
  const cells = grid.locator("tbody .replica:not(.missing)");
  await expect(cells.first()).toBeVisible();
  // Equal checksum text => equal colour.
  const pairs = await cells.evaluateAll((nodes) =>
    nodes.map((node) => [node.textContent, getComputedStyle(node).backgroundColor]),
  );
  const colourOf = new Map();
  for (const [text, colour] of pairs) {
    if (colourOf.has(text)) {
      expect(colourOf.get(text)).toBe(colour);
    }
    colourOf.set(text, colour);
  }

  await page.getByTestId("tab-cluster").click();
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).nth(2).click();
  await page.getByTestId("tab-data").click();
  await expect(grid.locator(".replica.down").first()).toBeVisible();
});

test("events_are_grouped_by_step_with_icons_and_jump", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("until-leader").click();
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  const events = page.getByTestId("events-panel");
  const crash = events.locator(".event-group", { hasText: "crash node-0" });
  await expect(crash).toHaveCount(1);
  await expect(crash.locator(".event-item.action .event-icon")).toHaveText("▸");
  await expect(events.locator(".event-item.leader .event-icon").first()).toHaveText("★");

  const first = events.locator(".event-group").first();
  await first.locator("button").click();
  await expect(first).toHaveClass(/current/);
  await expect(page.getByTestId("timeline-help")).toContainText("Rewound");
});

test("keyboard_shortcuts_step_scrub_play_and_help", async ({ page }) => {
  await openReady(page);
  await page.evaluate(() => document.activeElement?.blur());
  const label = page.getByTestId("timeline-label");
  await page.keyboard.press(".");
  await expect(label).toHaveText("step 1 of 1");
  await page.keyboard.press(".");
  await page.keyboard.press(".");
  await expect(label).toHaveText("step 3 of 3");
  await page.keyboard.press("ArrowLeft");
  await expect(label).toHaveText("step 2 of 3");
  await page.keyboard.press("Shift+ArrowLeft");
  await expect(label).toHaveText("step 0 of 3");
  await page.keyboard.press("ArrowRight");
  await expect(label).toHaveText("step 1 of 3");

  await page.keyboard.press(" ");
  await expect(page.getByTestId("play")).toHaveText("Pause");
  await page.keyboard.press(" ");
  await expect(page.getByTestId("play")).toHaveText("Play");

  // Typing in a field never triggers a shortcut.
  const stepNow = await label.textContent();
  await page.getByTestId("tab-clients").click();
  await page.getByTestId("manual-key").press(".");
  await expect(label).toHaveText(stepNow ?? "");

  await page.evaluate(() => document.activeElement?.blur());
  await page.keyboard.press("?");
  await expect(page.getByTestId("shortcuts-dialog")).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByTestId("shortcuts-dialog")).toBeHidden();
});

test("leader_changes_are_announced_politely", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("until-leader").click();
  await expect(page.getByTestId("announcer")).toContainText(/Leader is now node-\d at step \d+/);
  await expect(page.getByTestId("announcer")).toHaveAttribute("aria-live", "polite");
});

test("mobile_run_dock_follows_when_controls_scroll_away", async ({ page }, testInfo) => {
  await openReady(page);
  const dock = page.getByTestId("run-dock");
  await expect(dock).toBeHidden();
  await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
  if (!testInfo.project.name.startsWith("mobile")) {
    await expect(dock).toBeHidden();
    return;
  }
  await expect(dock).toBeVisible();
  await expect(page.getByTestId("dock-verdict")).toContainText("holds");
  await page.getByTestId("dock-step").click();
  await expect(page.getByTestId("dock-step-label")).toHaveText("step 1");
  await page.getByTestId("dock-play").click();
  await expect(page.getByTestId("dock-play")).toHaveText("Pause");
  await page.getByTestId("dock-play").click();
  await expect(page.getByTestId("dock-play")).toHaveText("Play");
  await page.evaluate(() => window.scrollTo(0, 0));
  await expect(dock).toBeHidden();
});

test("hidden_tab_panels_paint_when_their_tab_opens", async ({ page }) => {
  await openReady(page);
  // Advance while the Data tab is hidden: its panels are not rebuilt, but
  // opening the tab shows them painted from the current step.
  await page.getByTestId("step").click();
  await page.getByTestId("step").click();
  await page.getByTestId("tab-data").click();
  await expect(page.getByTestId("consistency-panel")).toContainText("live replicas agree");
  const keys = await page.getByTestId("keys-panel").textContent();
  expect((keys ?? "").trim().length).toBeGreaterThan(0);
  await page.getByTestId("tab-verdict").click();
  await expect(page.getByTestId("verdict-panel")).toContainText("election_safety");
});

test("intro_and_violation_cards_align_with_the_controls", async ({ page }) => {
  await openReady(page);
  const box = async (locator) => (await locator.boundingBox()) ?? { x: NaN, y: NaN, width: NaN, height: NaN };
  const controls = await box(page.locator(".controls"));
  const intro = page.getByTestId("intro");
  await expect(intro).toBeVisible();
  const card = await box(intro);
  expect(Math.abs(card.x - controls.x)).toBeLessThanOrEqual(1);
  expect(Math.abs(card.width - controls.width)).toBeLessThanOrEqual(1);
  // A visible gap under the controls, not flush against them.
  expect(card.y - (controls.y + controls.height)).toBeGreaterThanOrEqual(8);

  // The violation card shares the same inset (checked without a violation by
  // showing it directly; its content is covered by the violation smoke test).
  await page.getByTestId("violation-card").evaluate((node) => {
    node.hidden = false;
  });
  const violation = await box(page.getByTestId("violation-card"));
  expect(Math.abs(violation.x - controls.x)).toBeLessThanOrEqual(1);
  expect(Math.abs(violation.width - controls.width)).toBeLessThanOrEqual(1);
});

// Packets are drawn from the engine's in-flight messages and follow its rules:
// nothing crosses a cut link (the election model honours partitions too), a
// delayed message covers only its share of the edge per step, and a message
// whose target is down is drawn as lost.
test("packets_follow_link_state_and_delivery_schedule", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=12&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 12 of 12");
  const packets = () =>
    page.locator(".packet-layer circle.packet").evaluateAll((dots) =>
      dots.map((dot) => ({
        from: dot.dataset.from,
        to: dot.dataset.to,
        kind: dot.dataset.kind,
        span: dot.dataset.span.split("-").map(Number),
        doomed: dot.classList.contains("doomed"),
      })),
    );
  const initial = await packets();
  expect(initial.length).toBeGreaterThan(0);
  expect(initial.every((packet) => packet.span[0] === 0 && packet.span[1] === 1)).toBe(true);

  const linkAction = async (link, action) => {
    await page.getByTestId("link-select").selectOption(link);
    await page.getByTestId(`${action}-link`).click();
    await expect(page.getByTestId("selected-link")).toContainText(link.replace(">", " -> "));
  };
  await linkAction("node-0>node-1", "partition");
  await linkAction("node-1>node-0", "partition");
  await linkAction("node-0>node-2", "delay");

  const delayedSpan = async () => {
    const all = await packets();
    const crossing = all.filter(
      (packet) => [packet.from, packet.to].sort().join("|") === "node-0|node-1",
    );
    expect(crossing, "nothing crosses the cut link").toEqual([]);
    const delayed = all.find(
      (packet) => packet.from === "node-0" && packet.to === "node-2" && packet.span[1] - packet.span[0] < 0.1,
    );
    expect(delayed, "the delayed message covers a small share per step").toBeTruthy();
    return delayed.span;
  };
  const before = await delayedSpan();
  const label = await page.getByTestId("timeline-label").textContent();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("timeline-label")).not.toHaveText(label ?? "");
  const after = await delayedSpan();
  expect(after[0]).toBeGreaterThan(before[0]);

  const crash = page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" });
  await crash.nth(2).click();
  await expect(page.locator(".vertex.node.crashed")).toHaveCount(1);
  const toCrashed = (await packets()).filter((packet) => packet.to === "node-2");
  expect(toCrashed.length).toBeGreaterThan(0);
  expect(toCrashed.every((packet) => packet.doomed), "messages to a crashed node are drawn as lost").toBe(true);
});
