import { expect, test } from "@playwright/test";

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
  await page.getByTestId("subscribe-button").click();
  await page.getByTestId("push-event-button").click();

  await expect(page.getByTestId("clients-panel")).toContainText("client-a");
  await page.getByTestId("step").click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("subscribers-panel")).toContainText("upserted");
});

test("node_controls_show_reelection_resync_and_scale_out", async ({ page }) => {
  await openReady(page);
  await page.getByText("Isolate").first().click();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("engine-banner")).toContainText("Formation");
  await page.getByText("Rejoin").first().click();
  await page.getByTestId("add-node-button").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("node-3");
});

test("modes_switch_and_topology_is_clickable_in_each", async ({ page }) => {
  await openReady(page);
  for (const mode of ["manual", "scripted", "mixed"]) {
    await page.getByTestId("mode-select").selectOption(mode);
    await expect(page.getByTestId("intervention-status")).toContainText(mode);
    await page.getByText("Isolate").first().click();
    await page.getByText("Rejoin").first().click();
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

test("consistency_panel_states_levels_are_not_modeled", async ({ page }) => {
  await openReady(page);
  const panel = page.getByTestId("consistency-panel");
  await expect(panel).toContainText("not simulated");
  await expect(panel).toContainText("live replicas agree on sampled keys");
  await expect(panel).not.toContainText("LOCAL_QUORUM");
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
  const keysPanel = page.locator("details:has([data-testid='keys-panel'])");
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
    const panel = node.closest("details");
    if (panel) {
      panel.open = true;
    }
  });
}
