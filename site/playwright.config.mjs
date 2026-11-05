import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests",
  timeout: 10000,
  expect: { timeout: 3000 },
  use: { baseURL: "http://127.0.0.1:4173", headless: true },
  webServer: {
    command: "yarn site:dev",
    url: "http://127.0.0.1:4173",
    reuseExistingServer: false,
  },
});
