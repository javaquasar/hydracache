export default {
  testDir: "./tests",
  timeout: 45_000,
  expect: {
    timeout: 10_000
  },
  reporter: [["list"]],
  use: {
    baseURL: "http://127.0.0.1:5173/demo/",
    screenshot: "only-on-failure",
    trace: "retain-on-failure"
  },
  webServer: {
    command: "npm run serve",
    url: "http://127.0.0.1:5173/demo/",
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
