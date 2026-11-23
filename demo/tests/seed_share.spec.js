import { expect, test } from "@playwright/test";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

test("url_seed_reproduces_identical_run", async ({ browser }) => {
  const url = `${demoUrl}?seed=500&steps=12&scenario=default`;

  const first = await browser.newPage();
  await first.goto(url);
  await expect(first.getByTestId("snapshot-hash")).toContainText(/^snapshot /);
  const firstHash = await first.getByTestId("snapshot-hash").textContent();
  await first.close();

  const second = await browser.newPage();
  await second.goto(url);
  await expect(second.getByTestId("snapshot-hash")).toHaveText(firstHash ?? "");
  await second.close();
});

test("copy_reproducer_uses_current_seed_and_step", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=501&steps=3&scenario=default`);
  await expect(page.getByTestId("snapshot-hash")).toContainText(/^snapshot /);
  await page.getByTestId("copy-reproducer").click();

  await expect(page.getByTestId("copy-status")).toContainText(
    "cargo run -p hydracache-sim --bin vopr -- --seed 501 --steps 3",
  );
});

test("copy_reproducer_roundtrips_mode_and_actions", async ({ page, browser }) => {
  await page.goto(`${demoUrl}?seed=502&steps=0&scenario=default`);
  await expect(page.getByTestId("snapshot-hash")).toContainText(/^snapshot /);
  await page.getByTestId("mode-select").selectOption("mixed");
  await expect(page.getByTestId("intervention-status")).toContainText("mixed");
  await page.getByTestId("subscribe-button").click();
  await page.getByTestId("push-event-button").click();
  await expect(page.getByTestId("subscribers-panel")).toContainText("client-a@profiles");
  await page.getByTestId("copy-reproducer").click();

  await expect(page.getByTestId("copy-status")).toContainText("script=");
  const replayUrl = await page.getByTestId("copy-status").textContent();
  expect(replayUrl).toContain("script=");

  const replayed = await browser.newPage();
  await replayed.goto(replayUrl ?? "");
  await expect(replayed.getByTestId("intervention-status")).toContainText("mixed");
  await expect(replayed.getByTestId("intervention-status")).toContainText("3 replay action");
  await expect(replayed.getByTestId("subscribers-panel")).toContainText("client-a@profiles");
  await replayed.close();
});

test("copy_reproducer_replays_faults_to_identical_snapshot", async ({ page, browser }) => {
  await page.goto(`${demoUrl}?seed=503&steps=4&scenario=default`);
  await expect(page.getByTestId("snapshot-hash")).toContainText(/^snapshot /);

  await page.getByTestId("workload-toggle").uncheck();
  await expect(page.getByTestId("intervention-status")).toContainText("1 replay action");
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Crash" }).first().click();
  await expect(page.getByTestId("nodes-panel")).toContainText("crashed");
  await page.locator(".link-hit").first().dispatchEvent("click");
  await page.getByTestId("partition-link").click();
  await expect(page.getByTestId("selected-link")).toContainText("partitioned");
  await expect(page.getByTestId("intervention-status")).toContainText("3 replay action");

  await page.getByTestId("copy-reproducer").click();
  await expect(page.getByTestId("copy-status")).toContainText("script=");
  const replayUrl = await page.getByTestId("copy-status").textContent();
  const liveHash = await page.getByTestId("snapshot-hash").textContent();

  const replayed = await browser.newPage();
  await replayed.goto(replayUrl ?? "");
  await expect(replayed.getByTestId("snapshot-hash")).toHaveText(liveHash ?? "");
  await expect(replayed.getByTestId("intervention-status")).toContainText("3 replay action");
  await expect(replayed.getByTestId("workload-toggle")).not.toBeChecked();
  await replayed.close();
});

test("scenario_with_intervention_replays_from_curated_preset", async ({ page, browser }) => {
  await page.goto(`${demoUrl}?scenario=minority_partition_cannot_commit`);
  await expect(page.getByTestId("snapshot-hash")).toContainText(/^snapshot /);
  await page.getByTestId("nodes-panel").getByRole("button", { name: "Isolate" }).first().click();
  await expect(page.getByTestId("intervention-status")).toContainText("1 replay action");

  await page.getByTestId("copy-reproducer").click();
  await expect(page.getByTestId("copy-status")).toContainText("script=");
  const replayUrl = await page.getByTestId("copy-status").textContent();
  const liveHash = await page.getByTestId("snapshot-hash").textContent();

  const replayed = await browser.newPage();
  await replayed.goto(replayUrl ?? "");
  await expect(replayed.getByTestId("snapshot-hash")).toHaveText(liveHash ?? "");
  await expect(replayed.getByTestId("scenario-select")).toHaveValue(
    "minority_partition_cannot_commit",
  );
  await replayed.close();
});

test("invalid_url_seed_is_reported_not_silently_replaced", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=abc&steps=2&scenario=default`);
  await expect(page.getByTestId("verdict")).toContainText(/invariants hold|violation/);

  const notice = page.getByTestId("action-notice");
  await expect(notice).toBeVisible();
  await expect(notice).toContainText('URL seed "abc"');
  await expect(notice).toContainText("using 80");
});

test("foreign_simulator_api_is_refused", async ({ page }) => {
  const foreign = [];
  page.on("request", (request) => {
    if (new URL(request.url()).hostname === "evil.example") {
      foreign.push(request.url());
    }
  });
  await page.goto(`${demoUrl}?engine=server&api=https://evil.example`);

  await expect(page.getByTestId("verdict")).toContainText("server simulator unavailable");
  await expect(page.getByTestId("action-notice")).toContainText("was refused");
  await expect(page.getByTestId("step")).toBeDisabled();
  expect(foreign).toEqual([]);
});

