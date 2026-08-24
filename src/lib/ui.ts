import { currentYear, today } from './date';
import { METRICS, PLATFORM_ORDER } from './platforms';
import type { AccountConfig, Metric, Platform, Snapshot } from '../types';

export type TimeScope = number | 'until';
export type AccountMap = Record<Platform, AccountConfig[]>;

export const emptySnapshot: Snapshot = {
  stats: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  career: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  daily: [], platforms: [], difficulty: [], difficulty_daily: [], recent: [], metric_available: true, warnings: [],
};

export const SYNC_TIPS = [
  '卡题时先写下已经确认的性质，常常比继续盯代码更快找到突破口。',
  '比赛前十分钟先扫完题面，把“会做但实现长”的题留到节奏稳定之后。',
  '同一道题的第二种解法，往往比再刷一道同标签题更能暴露理解盲区。',
  '提交前用极小值、极大值、全相等和单调数据各走一遍边界。',
  '补题时记录真正卡住你的那一步：建模、性质、算法选择，还是实现。',
  '连续 WA 后先离开代码，用一句话重新描述不变量，再回来检查实现。',
  '把复杂度写在草稿最上方；它会不断提醒你哪些状态和转移不能出现。',
];

export function emptyAccounts(): AccountMap {
  return Object.fromEntries(PLATFORM_ORDER.map((platform) => [platform, []])) as unknown as AccountMap;
}
export function scopeRange(scope: TimeScope, timeZone = 'Asia/Shanghai') {
  if (scope !== 'until') return { start: `${scope}-01-01`, end: `${scope}-12-31` };
  const end = new Date(`${today(timeZone)}T00:00:00Z`); const start = new Date(end); start.setUTCDate(start.getUTCDate() - 364);
  return { start: start.toISOString().slice(0, 10), end: end.toISOString().slice(0, 10) };
}
export function initialScope(timeZone = 'Asia/Shanghai'): TimeScope {
  const saved = localStorage.getItem('oj-insight.time-scope');
  if (saved === 'until') return 'until';
  const year = Number(saved); return year >= 2010 && year <= currentYear(timeZone) ? year : 'until';
}
export function initialMetric(): Metric {
  const saved = localStorage.getItem('oj-insight.metric') as Metric | null;
  return METRICS.some((item) => item.value === saved) ? saved! : 'activity';
}
