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
