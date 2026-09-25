import { expect, test, type Page } from '@playwright/test';
import { installTauriMock } from './mock-tauri';

// Deterministic local data; these tests never sync accounts or contact a tracker.
async function openPage(page: Page, startup: string, compact = false) {
  await page.addInitScript(({ startup, compact }) => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({
      theme: 'gray', autoSync: false, autoCheckUpdates: false, startupPage: 'last',
      reduceMotion: true, density: compact ? 'compact' : 'comfortable', fontSize: compact ? 'xlarge' : 'standard',
    }));
    localStorage.setItem('oj-insight.last-page', startup);
    localStorage.setItem('oj-insight.time-scope', '2024');
  }, { startup, compact });
  await installTauriMock(page, {
    snapshot: {
      stats: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
      career: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
      daily: [{ day: '2024-06-03', count: 4 }],
      platforms: [],
      difficulty: [],
      difficulty_daily: [{ platform: 'codeforces', day: '2024-06-03', label: '1600', order: 1600 }],
      knowledge: ['基础与模拟','数据结构','图论与树','动态规划','数学','字符串','搜索与构造','贪心与思维'].flatMap((axis, index) => [
        { platform: 'codeforces', axis, count: 18 + index * 7, score: 54 + index * 5 },
        { platform: 'leetcode', axis, count: 4 + index, score: 15 + index * 3 },
        { platform: 'qoj', axis, count: [1,2,1,3,2,1,4,2][index], score: 4 + index },
      ]),
      ratings: [],
      recent: [],
      metric_available: true,
      warnings: [],
    },
  });
  await page.route('**/__day?*', route => route.fulfill({ body: 'ok' }));
  await page.route(/^https:\/\/(kenkoooo\.com|cftracker\.netlify\.app|www\.nowcoder\.com)\//,
    route => route.fulfill({ contentType: 'text/html', body: '<body style="margin:0;background:#152e23"><main style="height:200vh;color:white">Tracker fixture</main></body>' }));
  await page.goto('/');
}

async function checkHeatmaps(page: Page, fill: boolean) {
  await expect(page.locator('.heatmap')).toHaveCount(2);
  await expect.poll(async () => page.locator('.heatmap').evaluateAll(elements => elements.every(element => {
    const scroll = element.parentElement!;
    const grid = element.getBoundingClientRect();
    return grid.width >= scroll.clientWidth - 1;
  }))).toBe(true);
  for (const heatmap of await page.locator('.heatmap').all()) {
    await expect(heatmap.locator('.heat-cell')).toHaveCount(366);
    const metrics = await heatmap.evaluate(element => {
      const grid = element.getBoundingClientRect();
      const first = element.querySelector('.heat-cell')!.getBoundingClientRect();
      const last = element.querySelector('.heat-cell:last-child')!.getBoundingClientRect();
      const scroll = element.parentElement!;
      return { cellWidth: first.width, cellHeight: first.height, bottom: last.bottom - grid.bottom,
        rightGap: grid.right - last.right, overflow: scroll.scrollWidth - scroll.clientWidth };
    });
    expect(Math.abs(metrics.cellWidth - metrics.cellHeight)).toBeLessThan(1);
    expect(metrics.cellWidth).toBeGreaterThanOrEqual(12);
    expect(metrics.bottom).toBeLessThanOrEqual(0);
    expect(metrics.rightGap).toBeLessThan(25);
    if (fill) expect(metrics.overflow).toBeLessThanOrEqual(1);
    else expect(metrics.overflow).toBeGreaterThan(0);
  }
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
}

for (const [width, height, dpr] of [[1920, 1080, 1], [2560, 1440, 1], [3840, 2160, 1], [1920, 1080, 2], [1707, 960, 1.5]]) {
  test.describe(`${width}x${height} @${dpr}`, () => {
    test.use({ viewport: { width, height }, deviceScaleFactor: dpr });

    test('activity and difficulty fill the panel with square, clickable cells', async ({ page }) => {
      const errors: string[] = []; page.on('pageerror', error => errors.push(error.message));
      await openPage(page, 'codeforces');
      await checkHeatmaps(page, true);
      if (width === 1920 && dpr === 1) {
        await page.evaluate(() => window.scrollTo(0, 200));
        await page.screenshot({ path: test.info().outputPath('dashboard-1080p.png') });
      }
      const day = page.locator('.difficulty-map').getByRole('button', { name: '2024-06-03: 1600', exact: true });
      const request = page.waitForRequest('**/__day?day=2024-06-03');
      await day.click(); await request;
      expect(errors).toEqual([]);
    });

    test('embedded tracker fills the available viewport in both axes', async ({ page }) => {
      await openPage(page, 'tracker-atcoder');
      await expect(page.locator('iframe')).toBeVisible();
      const metrics = await page.locator('.embedded-tracker-frame').evaluate(frame => {
        const inner = frame.querySelector('iframe')!.getBoundingClientRect();
        const outer = frame.getBoundingClientRect();
        return { heightGap: frame.clientHeight - inner.height, widthGap: frame.clientWidth - inner.width,
          bottomGap: innerHeight - outer.bottom, rightGap: innerWidth - outer.right,
          documentHeight: document.documentElement.scrollHeight, viewportHeight: innerHeight };
      });
      expect(Math.abs(metrics.heightGap)).toBeLessThan(1);
      expect(Math.abs(metrics.widthGap)).toBeLessThan(1);
      expect(metrics.bottomGap).toBeGreaterThanOrEqual(0);
      expect(metrics.bottomGap).toBeLessThanOrEqual(56);
      expect(metrics.rightGap).toBeLessThanOrEqual(40);
      expect(metrics.documentHeight).toBeLessThanOrEqual(metrics.viewportHeight);
      if (width === 1920 && dpr === 1) await page.screenshot({ path: test.info().outputPath('tracker-1080p.png') });
    });
  });
}

test('resizing and folding the sidebar recalculates heatmaps; small windows scroll to the last day', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await openPage(page, 'codeforces', true);
  await checkHeatmaps(page, false);
  const last = page.locator('.heatmap').first().getByRole('button', { name: '2024-12-31: 0', exact: true });
  await last.scrollIntoViewIfNeeded();
  await expect(last).toBeInViewport();
  await page.setViewportSize({ width: 1400, height: 900 });
  await checkHeatmaps(page, true);
  const before = await page.locator('.heatmap .heat-cell').first().boundingBox();
  await page.getByRole('button', { name: '收起侧栏' }).click();
  await expect.poll(async () => (await page.locator('.heatmap .heat-cell').first().boundingBox())!.width).toBeGreaterThan(before!.width);
  await checkHeatmaps(page, true);
  await page.setViewportSize({ width: 1024, height: 768 });
  await page.getByRole('button', { name: '展开侧栏' }).click();
  await checkHeatmaps(page, false);
});

test('Luogu half-year stays inside its panel after resizing', async ({ page }) => {
  await openPage(page, 'luogu');
  await expect(page.locator('.heat-cell')).toHaveCount(183);
  for (const width of [1024, 1920]) {
    await page.setViewportSize({ width, height: 1080 });
    await expect.poll(async () => page.locator('.heatmap-scroll').evaluate(scroll => Math.abs(scroll.scrollWidth - scroll.clientWidth))).toBeLessThanOrEqual(1);
  }
});

test('all external trackers also fill a compact window with large fonts', async ({ page }) => {
  await page.setViewportSize({ width: 1024, height: 768 });
  await openPage(page, 'tracker-codeforces', true);
  for (const name of ['Codeforces', 'AtCoder']) {
    await page.locator('.tracker-items').getByRole('button', { name, exact: true }).click();
    await expect(page.locator('iframe')).toBeVisible();
    const gap = await page.locator('.embedded-tracker-frame').evaluate(frame => frame.clientHeight - frame.querySelector('iframe')!.getBoundingClientRect().height);
    expect(Math.abs(gap)).toBeLessThan(1);
    expect(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight)).toBe(true);
  }
});

test('resizing the tracker and collapsing the sidebar fills its new bounds without reloading', async ({ page }) => {
  await openPage(page, 'tracker-atcoder');
  await page.frameLocator('iframe').getByText('Tracker fixture').waitFor();
  const frame = page.frameLocator('iframe');
  await frame.locator('body').evaluate(body => body.setAttribute('data-loaded', 'retained'));
  for (const viewport of [{ width: 2560, height: 1440 }, { width: 1024, height: 768 }]) {
    await page.setViewportSize(viewport);
    await page.locator('.sidebar-toggle').click();
    const gap = await page.locator('.embedded-tracker-frame').evaluate(element => {
      const iframe = element.querySelector('iframe')!;
      return Math.abs(element.clientWidth - iframe.clientWidth) + Math.abs(element.clientHeight - iframe.clientHeight);
    });
    expect(gap).toBeLessThan(1);
    await expect(frame.locator('body')).toHaveAttribute('data-loaded', 'retained');
  }
});

test('daily check-in is saved locally and cannot be repeated on reload', async ({ page }) => {
  await openPage(page, 'overview');
  const checkin = page.getByRole('button', { name: '今日打卡', exact: true });
  await expect(checkin).toBeEnabled();
  await checkin.click();
  await expect(page.getByRole('button', { name: '今天已打卡', exact: true })).toBeDisabled();
  await expect(page.locator('.today-checkin-count')).toContainText('累计打卡 1 天');
  await expect(page.locator('.today-checkin-feedback')).toBeVisible();
  expect(await page.evaluate(() => JSON.parse(localStorage.getItem('oj-insight.checkins.v1') || '[]'))).toHaveLength(1);
  await page.reload();
  await expect(page.getByRole('button', { name: '今天已打卡', exact: true })).toBeDisabled();
  await expect(page.locator('.today-checkin-count')).toContainText('累计打卡 1 天');
  await expect(page.locator('.today-checkin-feedback')).toBeVisible();
  await expect(page.getByRole('button', { name: '撤销', exact: true })).toHaveCount(0);
});

test('production UI has no animation test button and Luogu omits unsupported detail panels', async ({ page }) => {
  await openPage(page, 'overview');
  await expect(page.getByRole('button', { name: /测试 \+1/ })).toHaveCount(0);
  await page.getByRole('button', { name: 'Platforms', exact: true }).click();
  await page.getByRole('button', { name: 'Luogu', exact: true }).click();
  await expect(page.locator('.rating-panel')).toHaveCount(0);
  await expect(page.locator('.recent-panel')).toHaveCount(0);
});

test('sync growth appears beside the changed solved total and fades away', async ({ page }) => {
  test.setTimeout(60_000);
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'dark', autoSync: false, autoCheckUpdates: false, startupPage: 'last' }));
    localStorage.setItem('oj-insight.last-page', 'overview');
  });
  const base = {
    stats: { solved: 10, accepted_submissions: 10, active_days: 1, longest_streak: 1, current_streak: 1, peak_day: '2026-09-18', peak_count: 1 },
    career: { solved: 10, accepted_submissions: 10, active_days: 1, longest_streak: 1, current_streak: 1, peak_day: '2026-09-18', peak_count: 1 },
    daily: [], difficulty: [], difficulty_daily: [], knowledge: [], ratings: [], recent: [], metric_available: true, warnings: [],
    platforms: [{ platform: 'codeforces' as const, account: 'tourist', solved: 10, accepted_submissions: 10, active_days: 1, today_count: 0, last_success: null, status: 'ok', message: '', activity_only: false, cached_records: 10, last_attempt: null }],
  };
  await installTauriMock(page, {
    snapshot: base,
    afterSyncSnapshot: { ...base, platforms: [{ ...base.platforms[0], solved: 12, today_count: 2, cached_records: 12 }] },
    accounts: [{ platform: 'codeforces', account: 'tourist', secret: '' }],
  });
  await page.clock.install();
  await page.goto('/');
  await expect(page.locator('.today-progress-count').first()).toContainText('0', { timeout: 15_000 });
  await page.getByRole('button', { name: '同步全部', exact: true }).click();
  await expect(page.locator('.today-progress-count').first()).toContainText('2');
  await expect(page.locator('.today-progress-count').first().locator('.solved-gain')).toHaveText('+2');
  await page.clock.fastForward(4_000);
  await expect(page.locator('.today-progress-count .solved-gain')).toHaveCount(0);
});

test('multiple NowCoder rating users keep separate histories and show public names', async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'dark', autoSync: false, autoCheckUpdates: false, startupPage: 'last' })));
  const stats = { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 };
  const rating = (account: string, display_name: string, current: number) => ({
    last_updated: 1_790_000_000, stale: false, platform: 'nowcoder' as const, account, display_name,
    current, maximum: current, last_change: 10, contest_count: 1, last_contest_epoch: 1_790_000_000,
    history: [{ contest_id: account, contest_name: `${display_name} 的比赛`, epoch_second: 1_790_000_000, old_rating: current - 10, new_rating: current, rank: 12 }],
  });
  await installTauriMock(page, { snapshot: { stats, career: stats, daily: [], platforms: [], difficulty: [], difficulty_daily: [], knowledge: [], ratings: [rating('10001', '小牛一号', 1450), rating('10002', '小牛二号', 1670)], recent: [], metric_available: true, warnings: [] } });
  await page.goto('/');
  await page.locator('.rating-tabs button').filter({ hasText: 'NC' }).click();
  const accounts = page.getByRole('combobox', { name: 'NowCoder Rating 账号' });
  await expect(accounts.locator('option')).toHaveText(['小牛一号 · 10001', '小牛二号 · 10002']);
  await accounts.selectOption('10002');
  await expect(page.locator('.rating-current strong')).toHaveText('1,670');
});

test('export chart choices are obvious and the preview stays inside its panel', async ({ page }, testInfo) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await openPage(page, 'export');
  const picker = page.getByRole('region', { name: '选择导出图表' });
  await expect(picker.getByRole('button')).toHaveCount(4);
  const previewCells = page.locator('.preview-heatmap i');
  await expect(previewCells).toHaveCount(35);
  const previewMetrics = await previewCells.evaluateAll((cells) => ({
    visible: cells.every((cell) => {
      const box = cell.getBoundingClientRect(); return box.width >= 7 && box.height >= 7;
    }),
    colors: new Set(cells.map((cell) => getComputedStyle(cell).backgroundColor)).size,
  }));
  expect(previewMetrics.visible).toBe(true);
  expect(previewMetrics.colors).toBeGreaterThanOrEqual(3);
  await expect(page.locator('.export-preview-notice')).toContainText('布局示意');
  for (const name of ['活动砖', '难度分布', '能力画像', '生涯总图']) {
    await picker.getByRole('button', { name: new RegExp(name) }).click();
    await expect(picker.getByRole('button', { name: new RegExp(name) })).toHaveClass(/active/);
    const fits = await page.locator('.export-preview-sheet').evaluate((sheet) => {
      const panel = sheet.parentElement!.getBoundingClientRect();
      const box = sheet.getBoundingClientRect();
      return box.left >= panel.left && box.right <= panel.right + 1;
    });
    expect(fits).toBe(true);
  }
  if (testInfo.project.name === 'chromium') {
    await page.getByRole('button', { name: '复制图片', exact: true }).click();
    await expect(page.getByText('图片已复制到剪贴板', { exact: true })).toBeVisible();
  }
  await page.screenshot({ path: test.info().outputPath('export-picker.png') });
});

test('contest review uses one compact workflow and reports the fixed package structure', async ({ page }) => {
  await page.setViewportSize({ width: 1180, height: 820 });
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'gray', autoSync: false, autoCheckUpdates: false, startupPage: 'last' }));
    localStorage.setItem('oj-insight.last-page', 'contest-review');
  });
  await installTauriMock(page, {
    accounts: [{ platform: 'atcoder', account: 'tourist', secret: '' }],
    contestReview: {
      platform: 'atcoder', contestId: 'abc380', contestName: 'AtCoder Beginner Contest 380', contestUrl: 'https://atcoder.jp/contests/abc380',
      account: 'tourist', startEpoch: 1_790_000_000, durationSeconds: 7_200, problemCount: 6, submissionCount: 9,
      codeAvailable: true, completeness: 'complete', notes: [],
    },
  });
  await page.goto('/');
  await expect(page.getByRole('heading', { name: '赛后分析', exact: true })).toBeVisible();
  await page.getByRole('button', { name: '正式比赛复盘', exact: true }).click();
  await page.getByPlaceholder('例如 abc380 或比赛链接').fill('abc380');
  await page.getByRole('button', { name: '检查比赛', exact: true }).click();
  await expect(page.getByRole('heading', { name: 'AtCoder Beginner Contest 380' })).toBeVisible();
  await expect(page.locator('.review-package')).toContainText('00-START-HERE · 01-CONTEST · 02-PROBLEMS · 03-SUBMISSIONS');
  await page.setViewportSize({ width: 1040, height: 680 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('knowledge radar labels keep their own positions and do not collapse into a corner', async ({ page }) => {
  await openPage(page, 'overview');
  const tabs = page.locator('.knowledge-tabs');
  await expect(tabs.getByRole('button')).toHaveText(['总览', 'Codeforces', 'LeetCode', 'ICPC/CCPC']);
  await expect(tabs.getByRole('button', { name: '总览', exact: true })).toHaveClass(/active/);
  await expect(page.locator('.knowledge-legend>div').first().locator('strong')).toContainText('36分');
  const labels = page.locator('.knowledge-labels text');
  await expect(labels).toHaveCount(8);
  const positions = await labels.evaluateAll((items) => items.map((item) => {
    const box = item.getBoundingClientRect(); return `${Math.round(box.x / 8)}:${Math.round(box.y / 8)}`;
  }));
  expect(new Set(positions).size).toBeGreaterThanOrEqual(7);
  await tabs.getByRole('button', { name: 'LeetCode', exact: true }).click();
  await expect(page.locator('.knowledge-legend>div').first().locator('strong')).toContainText('15分');
  await tabs.getByRole('button', { name: 'ICPC/CCPC', exact: true }).click();
  const qojWidths = await page.locator('.knowledge-legend>div>i>b').evaluateAll((bars) => bars.map((bar) => parseFloat((bar as HTMLElement).style.width)));
  expect(Math.max(...qojWidths)).toBe(11);
  expect(Math.min(...qojWidths.filter(Boolean))).toBe(4);
});
