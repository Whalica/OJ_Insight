import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

test('solve journal filters and preserves edits to archived records', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'gray', autoSync: false, autoCheckUpdates: false, startupPage: 'last' }));
    localStorage.setItem('oj-insight.last-page', 'solve-journal');
    localStorage.setItem('oj-insight.study-assistant.migrated', 'true');
    if (!localStorage.getItem('oj-insight.solve-journal.v1')) localStorage.setItem('oj-insight.solve-journal.v1', JSON.stringify([{
      id: 'test-solve-1', title: '最短路练习', url: 'https://example.com/problem', platform: 'codeforces', status: 'finished', outcome: 'unfinished',
      createdAt: Date.UTC(2026, 9, 1), updatedAt: Date.UTC(2026, 9, 1, 1), endedAt: Date.UTC(2026, 9, 1, 1), elapsedMs: 120000,
      note: '先画图 #图论', tags: ['图论'], mistakes: [{ id: 'mistake-1', at: Date.UTC(2026, 9, 1), reason: '粗心', note: '漏判边界', lostMinutes: 3 }],
    }]));
  });
  await installTauriMock(page);
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '解题手记' })).toBeVisible();
  await page.getByRole('textbox', { name: '搜索解题手记' }).fill('图论');
  await page.getByRole('button', { name: /最短路练习/ }).click();
  await expect(page.getByText('漏判边界')).toBeVisible();
  await page.getByRole('button', { name: '编辑', exact: true }).click();
  await page.getByRole('textbox', { name: '编辑手记' }).fill('复盘：检查边界 #图论');
  await page.getByRole('button', { name: '保存修改' }).click();
  await page.reload();
  await page.getByRole('button', { name: /最短路练习/ }).click();
  await expect(page.locator('.journal-note')).toContainText('复盘：检查边界');
});

test('desktop journal shows and updates the Competitive Companion port', async ({ page }) => {
  await page.addInitScript(() => {
    (window as unknown as { isTauri: boolean }).isTauri = true;
    localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'gray', autoSync: false, autoCheckUpdates: false, startupPage: 'last' }));
    localStorage.setItem('oj-insight.last-page', 'solve-journal');
    localStorage.setItem('oj-insight.study-assistant.migrated', 'true');
  });
  await installTauriMock(page);
  await page.goto('/');
  await expect(page.getByText('正在监听 127.0.0.1:10046')).toBeVisible();
  await page.getByRole('spinbutton', { name: 'Competitive Companion 端口' }).fill('10047');
  await page.getByRole('button', { name: '应用端口' }).click();
  await expect(page.getByText('正在监听 127.0.0.1:10047')).toBeVisible();
});
