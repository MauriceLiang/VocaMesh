import { defineConfig, devices } from "@playwright/test";

/**
 * Playwright 视觉与交互验收配置。
 * 前置：`pnpm dev`（或由 webServer 自行启动）。截图输出到 test-results/visual。
 * 基线：UI/qa/evidence/*.png（画布 1:1 导出），人工并排比对。
 */
export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 30_000,
  expect: { timeout: 5_000 },
  fullyParallel: false,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://localhost:5273",
    trace: "retain-on-failure",
    viewport: { width: 1280, height: 800 },
  },
  projects: [
    { name: "chromium-1280", use: { ...devices["Desktop Chrome"], viewport: { width: 1280, height: 800 } } },
    { name: "chromium-1024", use: { ...devices["Desktop Chrome"], viewport: { width: 1024, height: 720 } } },
    { name: "chromium-900", use: { ...devices["Desktop Chrome"], viewport: { width: 900, height: 650 } } },
  ],
  webServer: {
    command: "pnpm dev -- --port 5273",
    port: 5273,
    reuseExistingServer: true,
    timeout: 60_000,
  },
});
