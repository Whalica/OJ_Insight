import { expect, test } from '@playwright/test';
import { installTauriMock } from './mock-tauri';
import type { ProblemSet } from '../../src/types';

const set: ProblemSet = {
  id: 1, title: '社区题单测试', description: '', setType: 'static',
  tagVisibility: 'after_ac', sourceSetId: null, sourceUrl: null,
  createdAt: 0, updatedAt: 0, problems: [], solvedKeys: [],
};

for (const [theme, colorScheme, resolved] of [
  ['light', 'light', 'light'], ['gray', 'dark', 'gray'], ['dark', 'dark', 'dark'],
  ['system', 'light', 'light'], ['system', 'dark', 'dark'],
] as const) {
  test(`community consent shows its checked state with ${theme} theme and ${colorScheme} system`, async ({ page }) => {
    await page.emulateMedia({ colorScheme });
    await page.addInitScript(({ theme, set }) => {
      localStorage.setItem('oj-insight.preferences', JSON.stringify({
        theme, autoSync: false, autoCheckUpdates: false, startupPage: 'last', reduceMotion: true,
      }));
      localStorage.setItem('oj-insight.last-page', 'problem-sets');
      localStorage.setItem('__test_problem_sets__', JSON.stringify([set]));
    }, { theme, set });
    await installTauriMock(page);
    await page.goto('/');
    await expect(page.locator('html')).toHaveAttribute('data-theme', resolved);
    await page.getByRole('button', { name: '制作社区投稿文件', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: '制作社区投稿文件' });
    const consent = dialog.getByRole('checkbox', { name: /我确认题单内容可公开/ });
    const background = () => consent.evaluate(element => getComputedStyle(element).backgroundColor);
    await expect(consent).not.toBeChecked();
    const unchecked = await background();
    await dialog.getByText('我确认题单内容可公开，说明与备注不含隐私或无权转载的题面。', { exact: true }).click();
    await expect(consent).toBeChecked();
    await expect.poll(background).not.toBe(unchecked);
    const checked = await background();
    const mark = dialog.locator('.community-confirm-check');
    await expect(mark).toHaveCSS('opacity', '1');
    expect(await mark.evaluate(element => getComputedStyle(element).color)).not.toBe(checked);
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    await expect(consent).toBeFocused();
    await expect(consent).toHaveCSS('outline-style', 'solid');
    await page.keyboard.press('Space');
    await expect(consent).not.toBeChecked();
    await expect.poll(background).toBe(unchecked);
    await expect(mark).toHaveCSS('opacity', '0');
    await page.keyboard.press('Space');
    await expect(consent).toBeChecked();
    await expect.poll(background).toBe(checked);
  });
}
