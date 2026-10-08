import { expect, test, type Page } from '@playwright/test';
import type { XcpcContest } from '../../src/lib/xcpc';
import { installTauriMock } from './mock-tauri';

const cases: Array<Pick<XcpcContest, 'name' | 'shortName' | 'series' | 'stage' | 'site' | 'boardSource'>> = [
  { name: 'CCPC 2026 黑龙江省大学生程序设计竞赛', shortName: '2026 CCPC 黑龙江省赛', series: ['CCPC', '省赛'], stage: '省赛', site: '黑龙江', boardSource: 'XCPCIO' },
  { name: 'The 10th Hebei Collegiate Programming Contest', shortName: '第10届 河北省赛', series: ['省赛'], stage: '省赛', site: '河北', boardSource: null },
  { name: '其他比赛', shortName: '其他比赛', series: ['其他'], stage: '其他', site: '全国', boardSource: null },
  { name: 'The 2026 ICPC Asia East Continent Online Contest (I)', shortName: '2026 ICPC 亚洲东区网络赛 (I)', series: ['ICPC'], stage: '网络赛', site: '全国', boardSource: 'RankLand' },
  { name: '河南省赛（重复分类的旧缓存）', shortName: '河南省赛', series: ['CCPC', 'CCPC', '省赛'], stage: '省赛', site: '河南', boardSource: null },
];
const contests: XcpcContest[] = cases.map((item, index) => ({
  ...item, id: String(index), url: `https://qoj.ac/contest/${index}`, date: '2026-09-06', year: '2026', ratingsStale: false,
  problems: [{ index: 'A', name: 'Recall', problemId: String(index), url: `https://qoj.ac/problem/${index}`,
    tier: (['gold', 'silver', 'iron', 'bronze', 'bronze'] as const)[index], acceptedTeams: 1019, totalTeams: 2535, tagAxes: ['图论与树'], tags: ['最短路'], solved: true }],
}));

async function openTracker(page: Page) {
  await page.addInitScript(() => {
    localStorage.setItem('oj-insight.preferences', JSON.stringify({ theme: 'dark', autoSync: false, autoCheckUpdates: false, startupPage: 'last' }));
    localStorage.setItem('oj-insight.last-page', 'xcpc');
    localStorage.setItem('oj-insight.xcpc.show-problem-names', 'true');
  });
  await installTauriMock(page, { contests });
  await page.route('**/__open?*', route => route.fulfill({ body: 'ok' }));
  await page.goto('/');
  await expect(page.locator('tbody tr')).toHaveCount(contests.length);
}

test('cached contest tags are unique while keeping their order and styling', async ({ page }) => {
  const errors: string[] = [];
  page.on('pageerror', error => errors.push(error.message));
  page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
  await openTracker(page);
  await expect(page.getByRole('button', { name: 'ICPC/CCPC 题目标签来源：xcpcrating', exact: true })).toHaveAttribute('title', '题目标签数据整理自 xcpcrating，点击打开 GitHub 仓库');
  const expected = [
    ['CCPC', '省赛', '黑龙江', 'XCPCIO'], ['省赛', '河北'], ['其他', '全国'],
    ['ICPC', '网络赛', '全国', 'RankLand'], ['CCPC', '省赛', '河南'],
  ];
  for (let index = 0; index < expected.length; index++) {
    await expect(page.locator('tbody tr').nth(index).locator('.xcpc-contest-column > div > span')).toHaveText(expected[index]);
  }
  const provincial = page.locator('tbody tr').first();
  await expect(provincial.locator('.series')).toHaveText(['CCPC', '省赛']);
  await expect(provincial.locator('.board')).toHaveText('XCPCIO');
  // Updating an existing catalog must use the same deduplicated display.
  await page.getByRole('button', { name: '更新赛事数据', exact: true }).click();
  await expect(provincial.locator('.xcpc-contest-column > div > span')).toHaveText(expected[0]);
  await page.screenshot({ path: test.info().outputPath('xcpc-unique-tags.png') });
  expect(errors).toEqual([]);
});

test('series, stage, site and search filters still use the original classifications', async ({ page }) => {
  await openTracker(page);
  const series = page.locator('.xcpc-toolbar > .source-segments');
  await series.getByRole('button', { name: '省赛', exact: true }).click();
  await expect(page.locator('tbody tr')).toHaveCount(3);
  await series.getByRole('button', { name: 'CCPC', exact: true }).click();
  await expect(page.locator('tbody tr')).toHaveCount(2);
  const filter = page.getByRole('button', { name: '筛选', exact: true });
  await filter.press('Enter');
  await expect(filter).toHaveAttribute('aria-expanded', 'true');
  const stages = page.locator('.xcpc-filter-group').filter({ has: page.locator('legend', { hasText: '阶段' }) });
  await stages.locator('label').filter({ hasText: /^省赛$/ }).click();
  await expect(page.locator('tbody tr')).toHaveCount(2);
  await series.getByRole('button', { name: '全部', exact: true }).click();
  await expect(page.locator('tbody tr')).toHaveCount(3);
  await page.locator('.xcpc-filter-trigger').click();
  const sites = page.locator('.xcpc-filter-group').filter({ has: page.locator('legend', { hasText: '省份 / 赛站' }) });
  await sites.locator('label').filter({ hasText: /^黑龙江$/ }).click();
  await expect(page.locator('tbody tr')).toHaveCount(1);
  await page.getByRole('button', { name: '清除筛选', exact: true }).click();
  await expect(page.locator('tbody tr')).toHaveCount(5);
  await page.getByRole('textbox', { name: '搜索比赛' }).fill('黑龙江');
  await expect(page.locator('tbody tr')).toHaveCount(1);
});

test('name preferences, AC progress, rating colors, counts and links are preserved', async ({ page }) => {
  await openTracker(page);
  await page.getByRole('button', { name: '简称', exact: true }).click();
  const online = page.locator('tbody tr').nth(3);
  await expect(online.locator('.xcpc-contest-column > button')).toHaveText(cases[3].shortName);
  await expect(online.locator('.xcpc-progress-column > strong')).toHaveText('1 / 1');
  await expect(online).toHaveClass('complete');
  await expect(online.locator('.xcpc-problem')).toHaveClass(/solved.*tier-bronze/);
  await expect(online.locator('.xcpc-problem-tags')).toHaveCount(0);
  await page.getByRole('switch', { name: '知识标签', exact: true }).click();
  await expect(online.locator('.xcpc-problem-tags > i')).toHaveText(['图论与树', '最短路']);
  await expect(online.locator('.xcpc-problem-accepted')).toHaveText('1019 / 2535 队通过');
  const verticalOrder = await online.locator('.xcpc-problem button').evaluate((button) => {
    const title = button.querySelector('strong')!.getBoundingClientRect();
    const tags = button.querySelector('.xcpc-problem-tags')!.getBoundingClientRect();
    const accepted = button.querySelector('.xcpc-problem-accepted')!.getBoundingClientRect();
    return title.bottom <= tags.top && tags.bottom <= accepted.top;
  });
  expect(verticalOrder).toBe(true);
  const request = page.waitForRequest('**/__open?*');
  await online.locator('.xcpc-problem button').click();
  expect(new URL((await request).url()).searchParams.get('url')).toBe(contests[3].problems[0].url);
  await page.getByRole('button', { name: '全称', exact: true }).click();
  await expect(online.locator('.xcpc-contest-column > button')).toHaveText(cases[3].name);
  await page.getByRole('switch', { name: '难度颜色', exact: true }).click();
  await expect(page.locator('.xcpc-panel')).toHaveClass(/hide-difficulty/);
  await page.getByRole('switch', { name: '难度颜色', exact: true }).click();
  await expect(page.locator('.xcpc-panel')).not.toHaveClass(/hide-difficulty/);
});

test('difficulty color switch changes rendered problem colors in every theme', async ({ page }) => {
  await openTracker(page);
  const problemColors = () => page.locator('tbody .xcpc-problem > button strong').evaluateAll((items) =>
    items.map((item) => getComputedStyle(item).color));
  const toggle = page.getByRole('switch', { name: '难度颜色', exact: true });
  for (const theme of ['dark', 'light', 'gray']) {
    await page.evaluate((value) => { document.documentElement.dataset.theme = value; }, theme);
    const enabled = await problemColors();
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-checked', 'false');
    const disabled = await problemColors();
    expect(new Set(disabled).size).toBe(1);
    expect(disabled[0]).not.toBe(enabled[0]);
    await toggle.click();
    await expect(toggle).toHaveAttribute('aria-checked', 'true');
    expect(await problemColors()).toEqual(enabled);
  }
});
