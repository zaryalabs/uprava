import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:5173";
const useSystemChrome = process.env.UPRAVA_E2E_SYSTEM_CHROME === "1";

export default defineConfig({
  testDir: "./e2e",
  reporter: "list",
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  webServer: process.env.PLAYWRIGHT_BASE_URL
    ? undefined
    : {
        command: "npm run dev",
        reuseExistingServer: true,
        timeout: 120_000,
        url: baseURL,
      },
  projects: [
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        ...(useSystemChrome ? { channel: "chrome" as const } : {}),
      },
    },
  ],
});
