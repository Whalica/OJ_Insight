import { expect, test, type Page } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

async function openApp(page: Page, collapsed = false) {
  await page.setViewportSize({ width: 1024, height: 680 });
  await page.addInitScript((collapsed) => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({
      theme: 'gray', fontSize: 'xlarge', autoSync: false, autoCheckUpdates: false, startupPage: 'last',
    }));
    localStorage.setItem('oj-insight.last-page', 'overview');
    localStorage.setItem('oj-insight.sidebar-collapsed', String(collapsed));
    localStorage.setItem('oj-insight.relationship-auto-check', 'false');
  }, collapsed);
  await installTauriMock(page);
  await page.goto('/');
}

for (const collapsed of [false, true]) {
  test(`sidebar flyouts remain visible and clickable with collapsed=${collapsed}`, async ({ page }) => {
    await openApp(page, collapsed);
    const panel = page.locator('.nav-flyout');
    for (const group of ['platforms', 'trackers', 'training']) {
      await page.getByTestId(`nav-${group}`).hover();
      await expect(panel).toBeVisible();
      await expect.poll(() => panel.evaluate((element) => {
        const rect = element.getBoundingClientRect();
        const sidebar = document.querySelector('.sidebar')!.getBoundingClientRect();
        const buttons = [...element.querySelectorAll('button')];
        return rect.left >= sidebar.left && rect.right > sidebar.right
          && rect.top >= 0 && rect.bottom <= innerHeight
          && buttons.every((button) => {
            const box = button.getBoundingClientRect();
            return button.contains(document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2));
          });
      })).toBe(true);
      await panel.getByRole('button').first().focus();
      await expect(panel).toBeVisible();
      if (group === 'training') {
        await panel.getByRole('button', { name: '解题手记', exact: true }).click();
        await expect(page.getByRole('heading', { name: '解题手记', exact: true })).toBeVisible();
      } else {
        await page.keyboard.press('Escape');
      }
      await expect(panel).toHaveCount(0);
    }
  });
}

test('settings data sources tab only shows source controls at the top', async ({ page }) => {
  await openApp(page);
  await page.getByRole('navigation').getByRole('button', { name: '设置', exact: true }).click();
  const tabs = page.locator('.settings-tabs');
  await expect(tabs.getByRole('button').first()).toHaveText('数据源');
  await expect(page.locator('.account-panel')).toBeVisible();
  await tabs.getByRole('button', { name: '个性化', exact: true }).click();
  await expect(page.locator('.preferences-panel')).toBeVisible();
  await expect(page.locator('.source-list')).toHaveCount(0);
  await tabs.getByRole('button', { name: '数据源', exact: true }).click();
  await expect(page.locator('.source-list')).toBeVisible();
  await expect(page.locator('.source-list > article')).toHaveCount(6);
  await expect(page.locator('.preferences-panel')).toHaveCount(0);
  await expect(page.locator('.account-panel')).toHaveCount(0);
  expect((await page.locator('.data-tab-head').boundingBox())!.y).toBeLessThan(300);
  await tabs.getByRole('button', { name: '账号设置', exact: true }).click();
  await expect(page.locator('.account-panel')).toBeVisible();
  await expect(page.getByText('怎么填写？', { exact: true })).toHaveCount(0);
  await expect(page.getByText('在 OJI 中登录 QOJ', { exact: true })).toHaveCount(0);
  await expect(page.getByPlaceholder('完整 Cookie：cookie_name=cookie_value（多个用分号分隔）')).toBeVisible();
  await expect(page.locator('.source-list')).toHaveCount(0);
});

test('clicking a flyout trigger opens it immediately and never toggles it closed', async ({ page }) => {
  await openApp(page);
  const trigger = page.getByTestId('nav-platforms');
  const panel = page.locator('.nav-flyout');
  // Dispatch directly to exercise a click during the entry animation without
  // Playwright waiting for the target animation to settle first.
  await trigger.hover();
  await expect(panel).toBeAttached();
  await trigger.dispatchEvent('click');
  await expect(trigger).toHaveAttribute('aria-expanded', 'true');
  await expect(panel).toHaveCSS('animation-name', 'none');
  await expect(panel).toHaveCSS('opacity', '1');
  await trigger.click();
  await expect(panel).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(panel).toHaveCount(0);
  await trigger.dispatchEvent('click');
  await expect(panel).toHaveCSS('animation-name', 'none');
  await expect(panel).toBeVisible();
});
