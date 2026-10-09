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
        await expect(panel).toBeVisible();
        await page.mouse.move(900, 50);
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
  await expect(page.locator('.source-list')).toBeVisible();
  await expect(page.locator('.account-panel')).toHaveCount(0);
  await expect(page.locator('.settings-head').getByRole('button', { name: '同步全部', exact: true })).toBeVisible();
  await expect(page.locator('.data-tab-head').getByRole('button')).toHaveCount(0);
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

test('platform and tracker selections stay open until the pointer leaves', async ({ page }) => {
  await openApp(page);
  const panel = page.locator('.nav-flyout');
  await expect(page.getByTestId('nav-platforms')).toHaveText('平台');
  await page.getByTestId('nav-platforms').hover();
  await expect(panel.locator('.nav-flyout-title')).toHaveText('平台');
  await expect(panel.getByRole('button').first()).toHaveText('综合总览');
  expect(await panel.locator('.platform-icon').evaluateAll((icons) => icons.every((icon) => {
    const frame = icon.getBoundingClientRect();
    const image = icon.querySelector('img')?.getBoundingClientRect();
    return !image || image.width <= frame.width && image.height <= frame.height;
  }))).toBe(true);
  const luogu = panel.getByRole('button', { name: 'Luogu', exact: true });
  await luogu.click();
  await expect(luogu).toHaveAttribute('aria-current', 'true');
  await expect(panel).toBeVisible();
  await page.mouse.move(900, 50);
  await expect(panel).toHaveCount(0);
  await page.getByTestId('nav-trackers').hover();
  await expect(panel.getByRole('button')).toHaveText(['ICPC / CCPC', 'Codeforces', 'AtCoder']);
  await panel.getByRole('button', { name: 'Codeforces', exact: true }).click();
  await expect(panel).toBeVisible();
  await expect(panel.getByRole('button', { name: 'Codeforces', exact: true })).toHaveAttribute('aria-current', 'true');
  const sizes = await panel.getByRole('button').evaluateAll((buttons) => buttons.map((button) => button.getBoundingClientRect().height));
  expect(Math.max(...sizes) - Math.min(...sizes)).toBeLessThan(1);
  await page.mouse.move(900, 50);
  await expect(panel).toHaveCount(0);
});

test('training keeps all shortcuts in three separated groups', async ({ page }) => {
  await openApp(page);
  await page.getByTestId('nav-training').hover();
  const panel = page.locator('.nav-flyout');
  await expect(panel.getByRole('button')).toHaveText(['题单', '个性化组题', '模拟赛', '参赛区', '赛后分析', '解题手记', '收藏夹']);
  await expect(panel.getByRole('separator')).toHaveCount(2);
  for (const group of ['platforms', 'trackers', 'training']) {
    expect((await page.getByTestId(`nav-${group}`).boundingBox())!.height).toBeGreaterThanOrEqual(48);
  }
});

test('problem set sources switch inside the page and keep actions in the header', async ({ page }) => {
  await openApp(page);
  await page.getByTestId('nav-training').hover();
  await page.locator('.nav-flyout').getByRole('button', { name: '题单', exact: true }).click();
  await page.mouse.move(900, 50);
  const header = page.locator('.problem-sets-head');
  const tabs = page.locator('.problem-set-tabs');
  await expect(header.getByRole('heading', { name: '题单', exact: true })).toBeVisible();
  await expect(header.getByRole('button', { name: '导入题单', exact: true })).toBeVisible();
  await tabs.getByRole('button', { name: '推荐题单', exact: true }).click();
  await expect(header.getByRole('heading', { name: '推荐题单', exact: true })).toBeVisible();
  await expect(header.getByRole('button', { name: '投稿与审核', exact: true })).toBeVisible();
  await expect(header.getByRole('button', { name: '刷新', exact: true })).toBeVisible();
  await expect(page.locator('.data-tab-head')).toHaveCount(0);
  await tabs.getByRole('button', { name: '本地题单', exact: true }).click();
  await expect(header.getByRole('button', { name: '新建题单', exact: true })).toBeVisible();
});

test('contest pages use sidebar navigation without duplicate page switches', async ({ page }) => {
  await openApp(page);
  for (const name of ['模拟赛', '参赛区', '赛后分析']) {
    await page.getByTestId('nav-training').hover();
    await page.locator('.nav-flyout').getByRole('button', { name, exact: true }).click();
    await expect(page.locator('.main').getByRole('heading', { name, exact: true })).toBeVisible();
    for (const duplicate of ['模拟赛', '参赛区', '赛后分析']) {
      await expect(page.locator('.main').getByRole('button', { name: duplicate, exact: true })).toHaveCount(0);
    }
  }
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
