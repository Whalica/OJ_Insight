import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

test('study assistant opens from tools and keeps its timer and side-by-side Markdown notes', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({
      theme: 'gray', autoSync: false, autoCheckUpdates: false, startupPage: 'last',
      reduceMotion: true, density: 'comfortable', fontSize: 'standard',
    }));
    localStorage.setItem('oj-insight.last-page', 'training');
  });
  await installTauriMock(page);
  await page.goto('/');
  await page.clock.install();

  await page.getByRole('button', { name: '做题小助手' }).click();
  await page.goto('/?study_assistant=1');
  await page.setViewportSize({ width: 220, height: 96 });
  const bubble = page.getByRole('button', { name: /展开做题小助手/ });
  await expect(bubble).toBeVisible();
  await expect.poll(async () => (await bubble.boundingBox())?.width).toBe(220);
  await expect(bubble).toHaveCSS('border-radius', '0px');
  await expect(page.locator('body')).toHaveCSS('background-color', 'rgb(48, 50, 52)');
  await bubble.hover();
  await expect(bubble).toHaveCSS('transform', 'none');
  await bubble.click();
  await page.setViewportSize({ width: 720, height: 540 });
  await expect(page.locator('.study-assistant-resize')).toHaveCount(8);
  await page.locator('.study-assistant-resize-southeast').dispatchEvent('pointerdown', { button: 0 });
  await expect.poll(() => page.evaluate(() => (window as unknown as { __ASSISTANT_RESIZE_DIRECTION__?: string }).__ASSISTANT_RESIZE_DIRECTION__)).toBe('SouthEast');
  await expect(page.getByRole('textbox', { name: '小助手 Markdown 笔记' })).toHaveCSS('font-size', '16px');
  await expect(page.locator('.study-assistant-preview .vp-markdown')).toHaveCSS('font-size', '16px');
  await expect(page.locator('.study-assistant-note-pane > span').first()).toHaveCSS('font-size', '14px');
  await page.getByRole('button', { name: '开始', exact: true }).click();
  await expect(page.getByText('计时中')).toBeVisible();
  await page.clock.runFor(2100);
  await expect(page.locator('.study-assistant-timer time')).toHaveText('00:00:02');
  await page.getByRole('button', { name: '暂停', exact: true }).click();
  await expect(page.getByText('已暂停')).toBeVisible();
  await page.clock.runFor(3000);
  await expect(page.locator('.study-assistant-timer time')).toHaveText('00:00:02');

  await page.getByRole('textbox', { name: '小助手 Markdown 笔记' }).fill('## 思路\n**先枚举**');
  await expect(page.locator('.study-assistant-preview h3')).toHaveText('思路');
  await expect(page.locator('.study-assistant-preview strong')).toHaveText('先枚举');
  await expect(page.locator('.study-assistant-note-pane')).toHaveCount(2);

  await page.getByRole('button', { name: '收起小助手' }).click();
  await page.setViewportSize({ width: 220, height: 96 });
  await expect(bubble).toBeVisible();
  await page.reload();
  await expect(bubble).toBeVisible();
  await bubble.click();
  await expect(page.getByRole('textbox', { name: '小助手 Markdown 笔记' })).toHaveValue('## 思路\n**先枚举**');
  await page.getByRole('button', { name: '停止', exact: true }).click();
  await expect(page.getByText('已停止')).toBeVisible();
});

test('light assistant primary action remains legible', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'light', autoSync: false, autoCheckUpdates: false })));
  await installTauriMock(page);
  await page.goto('/?study_assistant=1');
  await page.getByRole('button', { name: /展开做题小助手/ }).click();
  const action = page.getByRole('button', { name: '开始', exact: true });
  await expect(action).toHaveCSS('background-color', 'rgb(37, 135, 73)');
  await expect(action).toHaveCSS('color', 'rgb(255, 255, 255)');
});

test('collapsed assistant keeps a long elapsed time inside its window', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('oj-insight.study-assistant.v1', JSON.stringify({ elapsedMs: ((123 * 60 + 4) * 60 + 5) * 1000, startedAt: null, status: 'paused', note: '' })));
  await installTauriMock(page);
  await page.setViewportSize({ width: 220, height: 96 });
  await page.goto('/?study_assistant=1');
  const card = page.getByRole('button', { name: /展开做题小助手/ });
  await expect(card.locator('time')).toHaveText('123:04:05');
  const fits = await card.evaluate((element) => {
    const outer = element.getBoundingClientRect();
    const timer = element.querySelector('time')!.getBoundingClientRect();
    return timer.left >= outer.left + 8 && timer.right <= outer.right - 8 && timer.top >= outer.top && timer.bottom <= outer.bottom;
  });
  expect(fits).toBe(true);
});
