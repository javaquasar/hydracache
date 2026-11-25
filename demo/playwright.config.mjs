const externalUrl = process.env.HYDRACACHE_DEMO_URL;
const demoUrl = externalUrl ?? "http://127.0.0.1:5173/demo/";

export default {
  testDir: "./tests",
  timeout: 45_000,
  expect: {
    timeout: 10_000
  },
  reporter: [["list"]],
  use: {
    baseURL: demoUrl,
    screenshot: "only-on-failure",
    trace: "retain-on-failure"
  },
  webServer: externalUrl ? undefined : {
    command: "npm run serve",
    url: demoUrl,
    reuseExistingServer: !process.env.CI,
    timeout: 120_000
  },
  projects: [
    {
      name: "desktop-1440x900",
      use: {
        viewport: { width: 1440, height: 900 }
      }
    },
    {
      name: "mobile-390x844",
      use: {
        viewport: { width: 390, height: 844 },
        isMobile: true
      }
    }
  ]
};
