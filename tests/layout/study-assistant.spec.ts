import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

test('study assistant keeps its timer and Markdown notes while changing pages', async ({ page }) => {
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
  const bubble = page.getByRole('button', { name: /展开做题小助手/ });
  await expect(bubble).toBeVisible();
  await bubble.click();
  await page.getByRole('button', { name: '开始', exact: true }).click();
  await expect(page.getByText('计时中')).toBeVisible();
  await page.clock.runFor(2100);
  await expect(page.locator('.study-assistant-timer time')).toHaveText('00:00:02');
  await page.getByRole('button', { name: '暂停', exact: true }).click();
  await expect(page.getByText('已暂停')).toBeVisible();
  await page.clock.runFor(3000);
  await expect(page.locator('.study-assistant-timer time')).toHaveText('00:00:02');

  await page.getByRole('textbox', { name: '小助手 Markdown 笔记' }).fill('## 思路\n**先枚举**');
  await page.getByRole('button', { name: '预览', exact: true }).click();
  await expect(page.locator('.study-assistant-preview h3')).toHaveText('思路');
  await expect(page.locator('.study-assistant-preview strong')).toHaveText('先枚举');

  await page.getByRole('button', { name: '收起小助手' }).click();
  await expect(bubble).toBeVisible();
  await page.getByRole('button', { name: '题单', exact: true }).click();
  await expect(bubble).toBeVisible();
  await page.reload();
  await expect(bubble).toBeVisible();
  await bubble.click();
  await page.getByRole('button', { name: '编辑', exact: true }).click();
  await expect(page.getByRole('textbox', { name: '小助手 Markdown 笔记' })).toHaveValue('## 思路\n**先枚举**');
  await page.getByRole('button', { name: '停止', exact: true }).click();
  await expect(page.getByText('已停止')).toBeVisible();
});
