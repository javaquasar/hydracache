import playwright from "../console/node_modules/playwright/index.js";

const { chromium } = playwright;

const baseUrl = process.env.DOCS_BASE_URL ?? "http://127.0.0.1:3000";
const pages = [
  "/",
  "/architecture.html",
  "/production-checklist.html",
  "/guides/database-query-caching.html",
  "/reference/api-links.html",
];

const viewports = [
  { name: "desktop", width: 1440, height: 900 },
  { name: "mobile", width: 390, height: 844 },
];

const browser = await chromium.launch();
const failures = [];

try {
  for (const viewport of viewports) {
    const page = await browser.newPage({ viewport });
    for (const route of pages) {
      const url = new URL(route, baseUrl).toString();
      await page.goto(url, { waitUntil: "networkidle" });

      const title = await page.title();
      const metrics = await page.evaluate(() => ({
        scrollWidth: document.documentElement.scrollWidth,
        clientWidth: document.documentElement.clientWidth,
        mainText: document.querySelector("main")?.textContent?.trim().length ?? 0,
        logoVisible:
          document.querySelector(".brand-mark img")?.getBoundingClientRect().width > 0 ||
          !location.pathname.endsWith("/"),
      }));

      if (!title.includes("HydraCache")) {
        failures.push(`${viewport.name} ${route}: title does not include HydraCache`);
      }
      if (metrics.mainText < 100) {
        failures.push(`${viewport.name} ${route}: main content looks empty`);
      }
      if (metrics.scrollWidth > metrics.clientWidth + 1) {
        failures.push(
          `${viewport.name} ${route}: horizontal overflow ${metrics.scrollWidth} > ${metrics.clientWidth}`,
        );
      }
      if (!metrics.logoVisible) {
        failures.push(`${viewport.name} ${route}: logo is not visible on home page`);
      }
    }
    await page.close();
  }
} finally {
  await browser.close();
}

if (failures.length > 0) {
  console.error("Docs visual smoke failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`Checked ${pages.length} docs pages across ${viewports.length} viewports.`);
