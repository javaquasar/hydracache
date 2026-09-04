const port = process.env.HYDRACACHE_CONSOLE_PORT ?? "5174";
const baseURL = `http://127.0.0.1:${port}/console/`;

export default {
  testDir: "./tests",
  testIgnore: "**/history.test.js",
  timeout: 45_000,
  expect: {
    timeout: 10_000
  },
  reporter: [["list"]],
  use: {
    baseURL,
    screenshot: "only-on-failure",
    trace: "retain-on-failure"
  },
  webServer: {
    command: "npm run serve",
    url: baseURL,
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
