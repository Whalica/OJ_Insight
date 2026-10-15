import { expect, test, type Page } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

const guide = (page: Page) => page.getByRole('dialog');
const calls = (page: Page, command: string) => page.evaluate((command) => (window as unknown as { __TAURI_CALLS__: Array<{ command: string; args: Record<string, unknown> }> }).__TAURI_CALLS__.filter((call) => call.command === command), command);

async function start(page: Page) {
  await page.goto('/');
  await expect(guide(page)).toBeVisible();
  await guide(page).getByRole('button', { name: '开始设置', exact: true }).click();
}

test('fresh install saves selected accounts, syncs only on request, and completes once', async ({ page }) => {
  await installTauriMock(page, { onboarding: true });
  await start(page);
  await expect(guide(page).getByRole('button', { name: '保存并继续' })).toBeDisabled();
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('tourist');
  await guide(page).getByRole('button', { name: 'AtCoder', exact: true }).click();
  await guide(page).getByLabel('AtCoder · 账号 ID').fill('chokudai');
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
  expect(await calls(page, 'sync_platform')).toHaveLength(0);
  const saved = await calls(page, 'save_all_accounts');
  expect(saved).toHaveLength(1);
  expect(saved[0].args.accounts).toEqual([{ platform: 'codeforces', account: 'tourist', secret: '' }, { platform: 'atcoder', account: 'chokudai', secret: '' }]);
  await guide(page).getByRole('button', { name: '开始同步', exact: true }).click();
  await expect(guide(page).locator('.onboarding-sync-status strong').filter({ hasText: '同步成功' })).toHaveCount(2);
  expect((await calls(page, 'sync_platform')).map((call) => call.args.platform)).toEqual(['codeforces', 'atcoder']);
  await guide(page).getByRole('button', { name: '继续探索' }).click();
  await guide(page).getByRole('button', { name: '进入总览' }).click();
  await expect(guide(page)).toHaveCount(0);
  // Completing the guide must not trigger another startup sync.
  await page.waitForTimeout(300);
  expect(await calls(page, 'sync_platform')).toHaveLength(2);
  await page.reload();
  await expect(guide(page)).toHaveCount(0);
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('oj-insight.onboarding')!).status)).toBe('completed');
});

test('skip is remembered and the guide can be reopened from settings', async ({ page }) => {
  await installTauriMock(page, { onboarding: true });
  await page.goto('/');
  await guide(page).getByRole('button', { name: '稍后设置' }).click();
  await page.reload();
  await expect(guide(page)).toHaveCount(0);
  await page.getByRole('navigation').getByRole('button', { name: '设置', exact: true }).click();
  await page.getByRole('button', { name: '重新打开向导' }).click();
  await expect(guide(page)).toBeVisible();
  await guide(page).getByRole('button', { name: '开始设置' }).click();
  await guide(page).getByRole('button', { name: '稍后连接' }).click();
  await guide(page).getByRole('button', { name: /^题单/ }).click();
  await expect(page.locator('.main').getByRole('heading', { name: '题单', exact: true })).toBeVisible();
  expect(await calls(page, 'save_all_accounts')).toHaveLength(0);
});

test('choosing sync later does not start a background sync when the guide closes', async ({ page }) => {
  await installTauriMock(page, { onboarding: true });
  await start(page);
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('tourist');
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await guide(page).getByRole('button', { name: '稍后同步' }).click();
  await guide(page).getByRole('button', { name: '进入总览' }).click();
  await expect(guide(page)).toHaveCount(0);
  await page.waitForTimeout(300);
  expect(await calls(page, 'sync_platform')).toHaveLength(0);
});

test('upgrading users are not interrupted and replay preserves all existing credentials', async ({ page }) => {
  const accounts = [{ platform: 'qoj' as const, account: 'alice', secret: 'session=private-test-cookie' }, { platform: 'codeforces' as const, account: 'alice', secret: '{"apiKey":"key","apiSecret":"secret"}' }];
  await installTauriMock(page, { onboarding: true, accounts });
  await page.goto('/');
  await expect(page.locator('.main')).toBeVisible();
  await page.getByRole('navigation').getByRole('button', { name: '设置', exact: true }).click();
  await expect(guide(page)).toHaveCount(0);
  await page.getByRole('button', { name: '重新打开向导' }).click();
  await guide(page).getByRole('button', { name: '开始设置' }).click();
  await guide(page).getByRole('button', { name: 'AtCoder', exact: true }).click();
  await guide(page).getByLabel('AtCoder · 账号 ID').fill('bob');
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
  expect((await calls(page, 'save_all_accounts'))[0].args.accounts).toEqual([...accounts, { platform: 'atcoder', account: 'bob', secret: '' }]);
  const state = await page.evaluate(() => localStorage.getItem('oj-insight.onboarding'));
  expect(state).not.toContain('cookie');
  expect(state).not.toContain('apiSecret');
});

test('save errors keep inputs and allow retry without advancing', async ({ page }) => {
  await installTauriMock(page, { onboarding: true });
  await start(page);
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('https://codeforces.com/profile/tourist');
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('alert')).toContainText('不要粘贴完整主页链接');
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('tourist');
  await page.evaluate(() => { (window as unknown as { __TEST_SAVE_FAIL__: boolean }).__TEST_SAVE_FAIL__ = true; });
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('alert')).toContainText('保存失败');
  await expect(guide(page).getByLabel('Codeforces · 账号 ID')).toHaveValue('tourist');
  await page.evaluate(() => { (window as unknown as { __TEST_SAVE_FAIL__: boolean }).__TEST_SAVE_FAIL__ = false; });
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
});

test('reload resumes the sync step and failures remain visible and retryable', async ({ page }) => {
  await installTauriMock(page, { onboarding: true, syncErrors: { codeforces: '网络暂时不可用' }, syncDelay: 500 });
  await start(page);
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('tourist');
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
  await page.reload();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
  expect(await calls(page, 'sync_platform')).toHaveLength(0);
  await guide(page).getByRole('button', { name: '开始同步' }).click();
  await expect(guide(page).getByRole('button', { name: '跳过向导' })).toBeDisabled();
  await page.keyboard.press('Escape');
  await expect(guide(page)).toBeVisible();
  await expect(guide(page).getByText('同步失败，可重试', { exact: true })).toBeVisible();
  await expect(guide(page).getByText('网络暂时不可用', { exact: true })).toBeVisible();
  await guide(page).getByRole('button', { name: '重新同步' }).click();
  await expect(guide(page).getByRole('button', { name: '重新同步' })).toBeEnabled();
  expect(await calls(page, 'sync_platform')).toHaveLength(2);
  await guide(page).getByRole('button', { name: '继续探索' }).click();
  await guide(page).getByRole('button', { name: '进入总览' }).click();
  await expect(guide(page)).toHaveCount(0);
});

test('retrying a refresh after a successful save does not save accounts twice', async ({ page }) => {
  await installTauriMock(page, { onboarding: true });
  await start(page);
  await guide(page).getByLabel('Codeforces · 账号 ID').fill('tourist');
  await page.evaluate(() => { (window as unknown as { __TEST_REFRESH_FAIL__: boolean }).__TEST_REFRESH_FAIL__ = true; });
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('alert')).toContainText('账号刷新失败');
  await expect(guide(page).getByRole('button', { name: '保存并继续' })).toBeEnabled();
  await page.evaluate(() => { (window as unknown as { __TEST_REFRESH_FAIL__: boolean }).__TEST_REFRESH_FAIL__ = false; });
  await guide(page).getByRole('button', { name: '保存并继续' }).click();
  await expect(guide(page).getByRole('heading', { name: '把已有训练记录带进来' })).toBeVisible();
  expect(await calls(page, 'save_all_accounts')).toHaveLength(1);
});

for (const theme of ['light', 'gray', 'dark']) {
  test(`guide fits a small window with ${theme} theme and large text`, async ({ page }) => {
    await page.setViewportSize({ width: 640, height: 560 });
    await installTauriMock(page, { onboarding: true });
    await page.goto('/');
    await expect(guide(page)).toBeVisible();
    await page.evaluate((theme) => { document.documentElement.dataset.theme = theme; document.documentElement.dataset.fontSize = 'xlarge'; }, theme);
    await expect(guide(page).locator('.onboarding-body > p').first()).toHaveCSS('font-size', '18px');
    expect(await guide(page).evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
    await guide(page).getByRole('button', { name: '开始设置' }).click();
    await expect(guide(page).getByLabel('Codeforces · 账号 ID')).toBeVisible();
    expect(await guide(page).evaluate((element) => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
    await page.keyboard.press('Escape');
    await expect(guide(page)).toHaveCount(0);
  });
}

test.describe('English onboarding', () => {
  test.use({ locale: 'en-US' });
  test('follows the system language and keeps keyboard focus inside the modal', async ({ page }) => {
    await installTauriMock(page, { onboarding: true });
    await page.goto('/');
    await expect(guide(page).getByRole('heading', { name: 'Your training, in one place' })).toBeVisible();
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      expect(await page.evaluate(() => !!document.activeElement?.closest('dialog'))).toBe(true);
    }
    await guide(page).getByRole('button', { name: 'Get started' }).click();
    await expect(guide(page).getByLabel('Codeforces · Account ID')).toBeVisible();
    await guide(page).getByRole('button', { name: 'Connect later' }).click();
    await guide(page).getByRole('button', { name: 'Go to Overview' }).click();
    await expect(guide(page)).toHaveCount(0);
  });
});
