import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

test('study assistant opens from training center and keeps its timer and side-by-side Markdown notes', async ({ page }) => {
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
  await page.setViewportSize({ width: 160, height: 160 });
  const bubble = page.getByRole('button', { name: /展开做题小助手/ });
  await expect(bubble).toBeVisible();
  await expect.poll(async () => await bubble.boundingBox()).toMatchObject({ x: 0, y: 0, width: 160, height: 160 });
  await expect(bubble).toHaveCSS('border-radius', '0px');
  await expect(bubble).toHaveCSS('border-width', '0px');
  await expect(bubble.evaluate((element) => {
    const stroke = getComputedStyle(element, '::after');
    return [stroke.borderTopWidth, stroke.borderRightWidth, stroke.borderBottomWidth, stroke.borderLeftWidth];
  })).resolves.toEqual(['3px', '3px', '3px', '3px']);
  await expect(page.locator('body')).toHaveCSS('background-color', 'rgb(48, 50, 52)');
  await bubble.hover();
  await expect(bubble).toHaveCSS('transform', 'none');
  await bubble.click();
  await page.setViewportSize({ width: 720, height: 540 });
  const panel = page.locator('.study-assistant-panel');
  await expect.poll(async () => await panel.boundingBox()).toMatchObject({ x: 0, y: 0, width: 720, height: 540 });
  await expect(panel).toHaveCSS('border-radius', '0px');
  await expect(panel).toHaveCSS('border-width', '0px');
  await expect(panel.evaluate((element) => {
    const stroke = getComputedStyle(element, '::after');
    return [stroke.borderTopWidth, stroke.borderRightWidth, stroke.borderBottomWidth, stroke.borderLeftWidth];
  })).resolves.toEqual(['2px', '2px', '2px', '2px']);
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
  await page.setViewportSize({ width: 160, height: 160 });
  await expect(bubble).toBeVisible();
  await page.reload();
  await expect(bubble).toBeVisible();
  await bubble.click();
  await expect(page.getByRole('textbox', { name: '小助手 Markdown 笔记' })).toHaveValue('## 思路\n**先枚举**');
  await page.getByRole('button', { name: '结束本次做题' }).click();
  await expect(page.getByText('已保存到训练中心 · 解题手记')).toBeVisible();
  await expect.poll(() => page.evaluate(() => (JSON.parse(localStorage.getItem('oj-insight.solve-journal.v1') || '[]') as Array<{ status: string }>).some((item) => item.status === 'finished'))).toBe(true);
  await page.goto('/');
  await page.getByRole('button', { name: '解题手记' }).click();
  await expect(page.getByRole('heading', { name: '解题手记' })).toBeVisible();
  await page.getByRole('button', { name: /未命名练习/ }).click();
  await expect(page.getByRole('heading', { name: '做题笔记' })).toBeVisible();
  await expect(page.locator('.journal-note strong')).toHaveText('先枚举');
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
  await page.setViewportSize({ width: 160, height: 160 });
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
