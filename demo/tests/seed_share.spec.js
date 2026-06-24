import { expect, test } from "@playwright/test";

const demoUrl = process.env.HYDRACACHE_DEMO_URL ?? "http://127.0.0.1:5173/demo/";

test("url_seed_reproduces_identical_run", async ({ browser }) => {
  const url = `${demoUrl}?seed=500&steps=12&scenario=default`;

  const first = await browser.newPage();
  await first.goto(url);
  const firstHash = await first.getByTestId("snapshot-hash").textContent();
  await first.close();

  const second = await browser.newPage();
  await second.goto(url);
  await expect(second.getByTestId("snapshot-hash")).toHaveText(firstHash ?? "");
  await second.close();
});

test("copy_reproducer_uses_current_seed_and_step", async ({ page }) => {
  await page.goto(`${demoUrl}?seed=501&steps=3&scenario=default`);
  await page.getByTestId("copy-reproducer").click();

  await expect(page.getByTestId("copy-status")).toContainText(
    "cargo run -p hydracache-sim --bin vopr -- --seed 501 --steps 3",
  );
});
