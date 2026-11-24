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
  await page.getByTestId("add-client-button").click();
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
  await expect(page.getByTestId("guide-title")).toContainText("1/7");
  await page.getByTestId("guide-do").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("leader");
  // The guide reads the result back and stays on the step until Next.
  const outcome = page.getByTestId("guide-outcome");
  await expect(outcome).toHaveClass(/ok/);
  await expect(outcome).toContainText(/✓ Leader node-\d, term \d+, at step \d+\./);
  await expect(page.getByTestId("guide-title")).toContainText("1/7");
  await expect(page.getByTestId("guide-do")).toBeDisabled();
  await expect(page.locator(".guide-dot.done")).toHaveCount(1);
  // Next opens the tab the step is about.
  await page.getByTestId("guide-next").click();
  await expect(page.getByTestId("guide-title")).toContainText("2/7");
  await expect(page.getByTestId("tab-clients")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("guide-back").click();
  await expect(page.getByTestId("guide-title")).toContainText("1/7");
  await expect(outcome).toBeVisible();
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
  await page.getByTestId("add-client-button").click();
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
    page.locator(".packet-layer .packet").evaluateAll((dots) =>
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

// A plain click on the empty canvas is not a pan: the view keeps following the
// cluster (it used to freeze the view, so the cluster drifted off-centre as it
// grew). A real drag releases the Fit button, and Fit brings the view back.
test("canvas_click_keeps_the_view_fitted_and_drag_releases_fit", async ({ page }) => {
  await openReady(page);
  const fit = page.getByTestId("zoom-fit");
  await expect(fit).toHaveAttribute("aria-pressed", "true");
  const svg = page.locator("#cluster-graph");
  const box = await svg.boundingBox();
  const empty = { x: box.x + 12, y: box.y + box.height / 2 };

  await page.mouse.click(empty.x, empty.y);
  await expect(fit).toHaveAttribute("aria-pressed", "true");
  for (let index = 0; index < 3; index += 1) {
    await page.getByTestId("add-node-button").click();
  }
  await expect(page.locator(".vertex.node")).toHaveCount(6);
  const offCentre = () =>
    svg.evaluate((element) => {
      const frame = element.getBoundingClientRect();
      const bodies = [...element.querySelectorAll(".vertex.node .v-body")].map((body) => body.getBoundingClientRect());
      const left = Math.min(...bodies.map((rect) => rect.left));
      const right = Math.max(...bodies.map((rect) => rect.right));
      const top = Math.min(...bodies.map((rect) => rect.top));
      const bottom = Math.max(...bodies.map((rect) => rect.bottom));
      return Math.max(
        Math.abs((left + right) / 2 - (frame.left + frame.width / 2)) / frame.width,
        Math.abs((top + bottom) / 2 - (frame.top + frame.height / 2)) / frame.height,
      );
    });
  // Within 12% of the frame: centred up to the overlay insets.
  await expect.poll(offCentre, { timeout: 5000 }).toBeLessThan(0.12);

  // Adding nodes may have scrolled the page: measure the canvas again.
  await svg.scrollIntoViewIfNeeded();
  const now = await svg.boundingBox();
  const from = { x: now.x + 12, y: now.y + now.height / 2 };
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move(from.x + 60, from.y + 20, { steps: 4 });
  await page.mouse.up();
  await expect(fit).toHaveAttribute("aria-pressed", "false");
  await fit.click();
  await expect(fit).toHaveAttribute("aria-pressed", "true");
});

// A role change a message causes is shown when that message lands: on the
// step node-0 times out, it campaigns at once, the others turn only when its
// vote request reaches them, and its tally counts responses as they arrive.
test("election_is_staged_by_message_arrival", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=2&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 2 of 2");
  // Slow steps, so the in-step stages are observable.
  await page.locator("#speed-input").evaluate((input) => {
    input.value = input.max;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const node = (id) => page.locator(`.vertex.node[aria-label^="${id},"]`);
  await page.getByTestId("step").click();

  await expect(node("node-0")).toHaveClass(/role-candidate/);
  await expect(node("node-0")).toHaveClass(/campaigning/);
  await expect(node("node-1")).toHaveClass(/role-joining/);
  const tally = page.locator(".vote-tally");
  await expect(tally).toHaveText("1/2 votes");
  const votes = page.locator(".packet-layer .packet[data-family='vote']");
  await expect(votes.first()).toBeAttached();
  expect(await votes.first().locator("path.packet-shape").count()).toBe(1);

  // The request lands, the voter turns; the responses land, the tally fills.
  await expect(node("node-1")).toHaveClass(/role-candidate/);
  await expect(node("node-1")).not.toHaveClass(/campaigning/);
  await expect(tally).toHaveText("2/2 votes");
  await expect(tally).toHaveClass(/won/);

  // The next step elects node-0; its badge pops in (no previous leader).
  await page.getByTestId("step").click();
  await expect(node("node-0")).toHaveClass(/role-leader/);
  await expect(tally).toHaveCount(0);

  // Crash the leader: the star travels to whichever node wins next.
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.locator(".leader-baton")).toHaveCount(1);
  await expect(page.locator(".vertex.node.role-leader")).toHaveCount(1);
  await expect(page.locator(".leader-baton")).toHaveCount(0);
});

test("reduced_motion_applies_roles_at_once_without_overlays", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(`${demoUrl}?seed=80&steps=2&scenario=default`);
  await expect(page.locator("html")).toHaveAttribute("data-reduced-motion", "true");
  await page.getByTestId("step").click();
  await expect(page.locator('.vertex.node[aria-label^="node-1,"]')).toHaveClass(/role-candidate/);
  await expect(page.locator(".vote-tally")).toHaveText("2/2 votes");
  await expect(page.locator(".term-float")).toHaveCount(0);
  await page.getByTestId("step").click();
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.locator(".vertex.node.role-leader")).toHaveCount(1);
  await expect(page.locator(".leader-baton")).toHaveCount(0);
  await expect(page.locator(".vertex.node.down-hit")).toHaveCount(0);
  await page.waitForTimeout(1500);
  await expect(page.locator(".arrival-ripple, .loss-burst")).toHaveCount(0);
});

// Node badges draw their own icons where a font glyph is unreliable, and each
// node shows on how many sampled keys it holds the live majority's version.
test("node_badges_and_replica_agreement", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=12&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 12 of 12");
  const node = (id) => page.locator(`.vertex.node[aria-label^="${id},"]`);
  const rows = page.getByTestId("nodes-panel");
  await expect(node("node-0").locator(".v-progress")).toHaveAttribute("data-agree", /^(\d+)\/\1$/);

  await rows.getByRole("button", { name: "Crash" }).nth(2).click();
  await expect(node("node-2")).toHaveClass(/role-crashed/);
  await expect(node("node-2").locator(".v-badge text")).toHaveText("✕");
  // The crashed node stops applying writes; the others move on, so it lags.
  await page.getByTestId("step").click();
  await page.getByTestId("step").click();
  await expect(node("node-2").locator(".v-progress")).toHaveClass(/behind/);

  await rows.getByRole("button", { name: "Disable" }).nth(1).click();
  await expect(node("node-1")).toHaveClass(/role-disabled/);
  await expect(node("node-1").locator(".v-badge text")).toHaveText("");
  await expect(node("node-1").locator(".v-icon")).toHaveAttribute("d", /^M/);
});

test("link_labels_are_pills_and_delay_shows_its_time", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=12&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 12 of 12");
  const link = async (value, action) => {
    await page.getByTestId("link-select").selectOption(value);
    await page.getByTestId(`${action}-link`).click();
  };
  await link("node-0>node-1", "partition");
  await link("node-0>node-2", "delay");
  await expect(page.locator(".link-label.partitioned .link-text")).toHaveText("partitioned");
  await expect(page.locator(".link-label.delayed .link-text")).toHaveText(/^delayed · \d+ ms$/);
  const pills = page.locator(".link-label:not(.up) .link-pill");
  await expect(pills).toHaveCount(2);
  for (const width of await pills.evaluateAll((rects) => rects.map((rect) => Number(rect.getAttribute("width"))))) {
    expect(width).toBeGreaterThan(40);
  }
});

test("legend_folds_and_remembers_the_choice", async ({ page }, testInfo) => {
  await openReady(page);
  const legend = page.getByTestId("graph-legend");
  const mobile = testInfo.project.name.startsWith("mobile");
  // Open on wide screens, folded on narrow ones.
  expect(await legend.evaluate((details) => details.open)).toBe(!mobile);
  if (!mobile) {
    // The fit keeps the cluster clear of the open legend overlay.
    await expect
      .poll(() =>
        page.evaluate(() => {
          const box = document.querySelector(".graph-legend").getBoundingClientRect();
          return [...document.querySelectorAll(".vertex.node .v-body")].some((node) => {
            const rect = node.getBoundingClientRect();
            return rect.bottom > box.top && rect.left < box.right && rect.right > box.left;
          });
        }),
      )
      .toBe(false);
  }
  await page.getByTestId("legend-toggle").click();
  expect(await legend.evaluate((details) => details.open)).toBe(mobile);
  // The <details> toggle event (which stores the choice) is asynchronous.
  await expect
    .poll(() => page.evaluate(() => window.localStorage.getItem("hydracache-demo-legend")))
    .toBe(mobile ? "open" : "closed");
  await page.reload();
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  expect(await legend.evaluate((details) => details.open)).toBe(mobile);
  await expect(legend).toContainText("heartbeat");
});

test("crash_hit_recovery_and_message_effects", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=12&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 12 of 12");
  const node = (id) => page.locator(`.vertex.node[aria-label^="${id},"]`);
  const rows = page.getByTestId("nodes-panel");

  // Fast steps; step until a delivered non-control message ripples at its
  // target (some steps carry only quiet heartbeat traffic).
  await page.locator("#speed-input").evaluate((input) => {
    input.value = input.min;
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
  const stepUntil = async (selector) => {
    for (let attempt = 0; attempt < 12; attempt += 1) {
      await page.getByTestId("step").click();
      // Effects fire when a packet lands, a few hundred ms into the step.
      for (let poll = 0; poll < 9; poll += 1) {
        if ((await page.locator(selector).count()) > 0) return true;
        await page.waitForTimeout(100);
      }
    }
    return false;
  };
  expect(await stepUntil(".arrival-ripple")).toBe(true);

  await rows.getByRole("button", { name: "Crash" }).nth(2).click();
  await expect(node("node-2")).toHaveClass(/down-hit/);
  // Writes still addressed to the crashed node are lost: they burst.
  expect(await stepUntil(".loss-burst")).toBe(true);

  await rows.getByRole("button", { name: "Restart" }).first().click();
  await expect(node("node-2")).toHaveClass(/synced|recovering/);
});

test("link_cut_marker_delay_crawl_and_backlog", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=80&steps=12&scenario=default`);
  await expect(page.getByTestId("timeline-label")).toHaveText("step 12 of 12");
  await expect(page.locator(".link-cut.shown")).toHaveCount(0);
  const link = async (value, action) => {
    await page.getByTestId("link-select").selectOption(value);
    await page.getByTestId(`${action}-link`).click();
  };
  await link("node-0>node-1", "partition");
  await expect(page.locator(".link-cut.shown")).toHaveCount(1);

  await link("node-0>node-2", "delay");
  const delayed = page.locator(".link.delayed");
  await expect(delayed).toHaveCount(1);
  expect(await delayed.evaluate((line) => getComputedStyle(line).animationName)).toBe("delay-crawl");
  // The held-back message is a backlog on that pair: the line swells.
  expect(Number(await delayed.getAttribute("data-backlog"))).toBeGreaterThan(0);
  expect(await delayed.evaluate((line) => parseFloat(getComputedStyle(line).strokeWidth))).toBeGreaterThan(3);
  // Undelayed pairs carry no backlog.
  const others = await page.locator(".link.up").evaluateAll((lines) => lines.map((line) => line.dataset.backlog));
  expect(others.every((value) => value === "0")).toBe(true);
});

// The default tour crosses the quorum boundary, checks a prediction against
// what the engine did, and ends with a way on to the next tour.
test("guide_prediction_boundary_and_finale", async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto(`${demoUrl}?seed=91&steps=0&scenario=default`);
  await expect(page.getByTestId("guide-title")).toContainText("1/7");
  const doStep = async () => {
    await page.getByTestId("guide-do").click();
    await expect(page.getByTestId("guide-outcome")).toBeVisible({ timeout: 30_000 });
    await expect(page.getByTestId("guide-outcome")).toHaveClass(/ok/);
  };
  for (let index = 0; index < 4; index += 1) {
    await doStep();
    await page.getByTestId("guide-next").click();
  }
  await expect(page.getByTestId("guide-title")).toContainText("5/7");
  // Guess wrong on purpose: one node alone cannot lead.
  await page.getByTestId("guide-predict").getByRole("button", { name: "It becomes leader on its own" }).click();
  await doStep();
  await expect(page.getByTestId("guide-outcome")).toContainText("No leader: 1 of 3 nodes live, a quorum needs 2.");
  await expect(page.locator(".guide-guess.wrong")).toContainText('the engine did "No leader: one node is not a quorum"');
  await expect(page.locator(".guide-option.actual")).toContainText("No leader");
  await expect(page.locator(".guide-option.wrong")).toContainText("It becomes leader");
  await expect(page.getByTestId("tab-data")).toHaveAttribute("aria-selected", "true");

  for (let index = 5; index < 7; index += 1) {
    await page.getByTestId("guide-next").click();
    await doStep();
  }
  const finale = page.getByTestId("guide-finale");
  await expect(finale).toBeVisible();
  await expect(finale).toContainText("Tour complete: 7 of 7 checked steps confirmed by the engine.");
  await expect(page.locator(".guide-dot.done")).toHaveCount(7);
  await page.getByTestId("guide-next-tour").click();
  await expect(page.getByTestId("guide-title")).toContainText("Minority partition cannot commit · 1/3");
});

test("guide_deep_link_play_and_honest_failure", async ({ page }) => {
  test.setTimeout(60_000);
  // A link opens the guide on its step and its tab.
  await page.goto(`${demoUrl}?seed=91&steps=0&scenario=default&tourStep=3`);
  await expect(page.getByTestId("guide-title")).toContainText("3/7");
  await expect(page.getByTestId("tab-data")).toHaveAttribute("aria-selected", "true");
  await page.getByTestId("guide-link").click();
  await expect(page.getByTestId("copy-status")).toContainText("tourStep=3");

  // A step whose precondition never happened says so instead of moving on.
  await page.locator(".guide-dot").nth(5).click();
  await expect(page.getByTestId("guide-title")).toContainText("6/7");
  await page.getByTestId("guide-do").click();
  await expect(page.getByTestId("guide-outcome")).toHaveClass(/bad/);
  await expect(page.getByTestId("guide-outcome")).toContainText("✗ The step stopped: this guide has not crashed a node yet");
  await expect(page.locator(".guide-dot.failed")).toHaveCount(1);

  // Play the tour runs the remaining steps with pauses and stops at the end.
  await page.goto(`${demoUrl}?scenario=each_quorum_region_loss_fails_loud`);
  await expect(page.getByTestId("guide-title")).toContainText("1/2");
  await page.getByTestId("guide-run").click();
  await expect(page.getByTestId("guide-run")).toHaveText("Stop the tour");
  await expect(page.getByTestId("guide-finale")).toBeVisible({ timeout: 20_000 });
  await expect(page.getByTestId("guide-outcome")).toContainText(/✓ Leader node-\d/);
  await expect(page.getByTestId("guide-run")).toHaveText("Play the tour");
});

// Writers and subscribers are separate: a writer is added without writing, a
// subscriber is only a reader, each row has its own action, and the graph
// links writers to the subscribers of their namespace.
test("writers_and_subscribers_are_managed_separately", async ({ page }) => {
  await openReady(page);
  await page.getByTestId("tab-clients").click();
  const writers = page.getByTestId("clients-panel");
  const readers = page.getByTestId("subscribers-panel");

  await page.getByTestId("add-client-button").click();
  await expect(writers).toContainText("client-a");
  await expect(writers).toContainText("no writes yet");
  await expect(page.getByTestId("manual-client")).toHaveValue("client-b");
  // Adding an existing writer fails loud.
  await page.getByTestId("manual-client").fill("client-a");
  await page.getByTestId("add-client-button").click();
  await expect(page.getByTestId("action-notice")).toContainText("client 'client-a' already exists");

  await page.getByTestId("subscribe-button").click();
  await expect(readers).toContainText("reader-1@profiles");
  await expect(readers).toContainText("waiting for events");
  await expect(page.getByTestId("manual-reader")).toHaveValue("reader-2");
  await expect(page.locator(".vertex.subscriber")).toHaveCount(1);
  await expect(page.locator(".vertex.client")).toHaveCount(1);
  await expect(page.locator(".namespace-flow")).toHaveCount(1);

  await page.getByTestId("push-client-a").click();
  await expect(writers).toContainText("push profiles:profile-42");
  await page.getByTestId("step").click();
  await expect(readers).toContainText(/upserted profiles:profile-42 @ step \d+/);

  // The subscriber on the graph opens its row.
  await page.getByTestId("tab-data").click();
  await page.locator(".vertex.subscriber").first().focus();
  await page.keyboard.press("Enter");
  await expect(page.getByTestId("tab-clients")).toHaveAttribute("aria-selected", "true");

  await page.getByTestId("unsubscribe-reader-1@profiles").click();
  await expect(readers).toContainText("no subscribers");
  await expect(page.locator(".vertex.subscriber")).toHaveCount(0);
  await expect(page.locator(".namespace-flow")).toHaveCount(0);

  // Namespaces in use are offered in the namespace fields.
  await page.getByTestId("manual-client").fill("client-c");
  await page.getByTestId("manual-ns").fill("orders");
  await page.getByTestId("add-client-button").click();
  await expect(page.locator("#namespace-options option[value='orders']")).toHaveCount(1);
});
