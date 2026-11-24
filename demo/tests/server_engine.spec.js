import { expect, test } from "@playwright/test";

// Browser path through the sandbox HTTP simulator (`engine=server`). Runs only
// when HYDRACACHE_SANDBOX_URL points at a running sandbox, e.g.
//   cargo run -p hydracache-sandbox -- --backend memory
//   HYDRACACHE_SANDBOX_URL=http://127.0.0.1:3000 npx --prefix demo playwright test demo/tests/server_engine.spec.js
// The sandbox holds one simulator world for every client, so this file runs
// serially and in a single Playwright project.
const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";
const sandboxUrl = process.env.HYDRACACHE_SANDBOX_URL;

test.describe.configure({ mode: "serial" });

test.beforeEach(async ({}, testInfo) => {
  test.skip(!sandboxUrl, "HYDRACACHE_SANDBOX_URL is not set");
  test.skip(
    testInfo.project.name !== "desktop-1440x900",
    "the sandbox simulator world is shared; run it from one project",
  );
});

function serverUrl(query) {
  return `${demoUrl}?engine=server&api=${encodeURIComponent(sandboxUrl)}&${query}`;
}

async function openServer(page, query) {
  await page.goto(serverUrl(query));
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);
  await expect(page.getByTestId("demo-shell")).toHaveAttribute("aria-busy", "false");
}

test("server_engine_runs_real_raft_election", async ({ page }) => {
  await openServer(page, "seed=700&steps=4&scenario=default");

  await expect(page.getByTestId("election-source-chip")).toHaveText("raft");
  await expect(page.getByTestId("engine-banner")).toContainText("election raft");
  await expect(page.getByTestId("timeline-label")).toHaveText("step 4 of 4");

  // raft-rs needs its election timeout in ticks; step the server until it elects.
  await page.getByTestId("until-leader").click();
  await expect(page.getByTestId("nodes-panel")).toContainText("leader", { timeout: 30_000 });
  await expect(page.getByTestId("action-notice")).toBeHidden();
});

test("server_engine_scenarios_keep_raft", async ({ page }) => {
  await openServer(page, "scenario=leader_crash_failover_no_committed_loss");

  await expect(page.getByTestId("election-source-chip")).toHaveText("raft");
  await expect(page.getByTestId("scenario-select")).toHaveValue("leader_crash_failover_no_committed_loss");
  await expect(page.getByTestId("guide-title")).toContainText("Leader crash, no committed loss");
});

test("server_engine_interventions_replay_identically", async ({ page, browser }) => {
  await openServer(page, "seed=701&steps=10&scenario=default");
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.getByTestId("nodes-panel")).toContainText("crashed");
  await page.getByTestId("link-select").selectOption("node-1>node-2");
  await page.getByTestId("partition-link").click();
  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
  await expect(page.getByTestId("intervention-status")).toContainText("2 replay action");

  await page.getByTestId("copy-reproducer").click();
  await expect(page.getByTestId("copy-status")).toContainText("script=");
  const replayUrl = await page.getByTestId("copy-status").textContent();
  const liveHash = await page.getByTestId("snapshot-hash").textContent();
  expect(new URL(replayUrl ?? "").searchParams.get("engine")).toBe("server");

  // The replayed page rebuilds the run through POST /sim/control.
  const replayed = await browser.newPage();
  await replayed.goto(replayUrl ?? "");
  await expect(replayed.getByTestId("snapshot-hash")).toHaveText(liveHash ?? "");
  await expect(replayed.getByTestId("election-source-chip")).toHaveText("raft");
  await replayed.close();
});

test("server_engine_timeline_rewinds_and_returns", async ({ page }) => {
  await openServer(page, "seed=702&steps=8&scenario=default");
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.getByTestId("nodes-panel")).toContainText("crashed");
  const startStep = Number(
    (await page.getByTestId("timeline-label").textContent())?.match(/step (\d+)/)?.[1],
  );
  for (let index = 0; index < 3; index += 1) {
    await page.getByTestId("step").click();
  }
  // Steps are queued engine calls; wait for the last one before reading.
  await expect(page.getByTestId("timeline-label")).toHaveText(
    `step ${startStep + 3} of ${startStep + 3}`,
  );
  const endStep = Number(
    (await page.getByTestId("timeline-label").textContent())?.match(/step (\d+)/)?.[1],
  );
  await expect(page.getByTestId("timeline-label")).toHaveText(`step ${endStep} of ${endStep}`);
  const endHash = await page.getByTestId("snapshot-hash").textContent();

  await scrubTo(page, 4);
  await expect(page.getByTestId("timeline-label")).toHaveText(`step 4 of ${endStep}`);
  await expect(page.getByTestId("nodes-panel")).not.toContainText("crashed");

  await scrubTo(page, endStep);
  await expect(page.getByTestId("snapshot-hash")).toHaveText(endHash ?? "");
});

test("server_engine_rejections_are_shown", async ({ page }) => {
  await openServer(page, "seed=703&steps=6&scenario=default");
  await page.getByTestId("link-select").selectOption("node-0>node-1");
  await page.getByTestId("heal-link").click();

  await expect(page.getByTestId("action-notice")).toContainText("server simulator 400");
  await expect(page.getByTestId("action-notice")).toContainText("not partitioned");
});

async function scrubTo(page, step) {
  await page.getByTestId("timeline-input").evaluate((input, value) => {
    input.value = String(value);
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, step);
}
