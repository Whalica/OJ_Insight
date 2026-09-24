import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: '../tests/layout',
  fullyParallel: false,
  workers: 1,
  timeout: 90_000,
  expect: { timeout: 10_000 },
  use: { baseURL: 'http://127.0.0.1:1430', trace: 'retain-on-failure' },
  projects: [
    { name: 'chromium', use: { browserName: 'chromium' } },
    { name: 'webkit', use: { browserName: 'webkit' } },
  ],
  webServer: {
    command: 'pnpm dev --host 127.0.0.1 --port 1430',
    url: 'http://127.0.0.1:1430',
    reuseExistingServer: !process.env.CI,
  },
});
