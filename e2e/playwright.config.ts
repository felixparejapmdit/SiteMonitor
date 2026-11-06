import { defineConfig, devices } from "@playwright/test";

const PORT = Number(process.env.SITEMONITOR_PORT ?? 8737);

export default defineConfig({
  testDir: "./tests",
  // Tests share one database, so they run one file at a time.
  fullyParallel: false,
  workers: 1,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: `http://localhost:${PORT}`,
    viewport: { width: 1440, height: 900 },
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], viewport: { width: 1440, height: 900 } } }],
  webServer: {
    command: "cargo run -p backend --release",
    cwd: "..",
    url: `http://localhost:${PORT}/api/health`,
    reuseExistingServer: true,
    timeout: 300_000,
  },
});
