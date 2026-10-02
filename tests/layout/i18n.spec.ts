import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

test.describe('new installation', () => {
  test.use({ locale: 'en-US' });

  test('follows the system language by default', async ({ page }) => {
    await installTauriMock(page);
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
    await expect(page.getByRole('button', { name: 'Settings' })).toBeVisible();
    await expect.poll(() => page.evaluate(() => localStorage.getItem('oj-insight.preferences'))).toBeNull();
  });
});

test('existing settings keep Chinese and language switch persists across windows', async ({ page }) => {
  await page.addInitScript(() => {
    if (!localStorage.getItem('oj-insight.preferences')) {
      localStorage.setItem('oj-insight.preferences', JSON.stringify({
        theme: 'gray', autoSync: false, autoCheckUpdates: false,
      }));
    }
  });
  await installTauriMock(page);
  await page.goto('/');
  await expect(page.locator('html')).toHaveAttribute('lang', 'zh-CN');
  await page.getByRole('button', { name: '设置', exact: true }).click();
  await page.getByRole('button', { name: '个性化', exact: true }).click();
  await page.getByRole('button', { name: 'English', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  await expect(page.getByRole('heading', { name: 'Settings' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Training Center' })).toBeVisible();
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('oj-insight.preferences') || '{}').language)).toBe('en-US');

  await page.getByRole('button', { name: 'Training Center' }).click();
  await page.getByRole('button', { name: 'Personalized Training' }).click();
  await expect(page.getByRole('heading', { name: 'Personalized Training' })).toBeVisible();
});

test('detached study assistant uses the saved English language', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({
      language: 'en-US', theme: 'gray', autoSync: false, autoCheckUpdates: false,
    }));
  });
  await installTauriMock(page);
  await page.goto('/?study_assistant=1');
  await expect(page.locator('html')).toHaveAttribute('lang', 'en-US');
  await page.getByRole('button', { name: /Expand Study Assistant/ }).click();
  await expect(page.getByRole('button', { name: 'Start', exact: true })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Study Assistant Markdown notes' })).toBeVisible();
});
