import { expect, test } from "@playwright/test";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

test("loads_steps_and_renders_verdict", async ({ page }) => {
  await openReady(page);
  await expect(page.getByTestId("engine-banner")).toContainText("election sim-model");
  await expect(page.getByTestId("signals-panel")).toBeVisible();

  const before = await page.getByTestId("progress-panel").textContent();
  await page.getByTestId("step").click();
  await expect(page.getByTestId("progress-panel")).not.toHaveText(before ?? "");
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
