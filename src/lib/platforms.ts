import type { Platform } from '../types';

export const PLATFORM_ORDER: Platform[] = ['codeforces', 'atcoder', 'luogu', 'nowcoder', 'qoj', 'leetcode'];

export const PLATFORM_META: Record<Platform, { name: string; short: string; accent: string; accountHint: string; secretHint?: string }> = {
  codeforces: { name: 'Codeforces', short: 'CF', accent: '#5aa6e8', accountHint: 'Handle' },
  atcoder: { name: 'AtCoder', short: 'ATC', accent: '#d7d9dc', accountHint: '用户名' },
  luogu: { name: 'Luogu', short: 'LG', accent: '#2d9cdb', accountHint: '用户名或数字 UID' },
  nowcoder: { name: 'NowCoder', short: 'NC', accent: '#00b96b', accountHint: '个人主页 users/ 后的数字 User ID' },
  qoj: { name: 'QOJ', short: 'QOJ', accent: '#48d0c0', accountHint: '用户名', secretHint: '可选：UOJSESSID=...（QOJ 当前需登录查看完整提交）' },
  leetcode: { name: 'LeetCode', short: 'LC', accent: '#f3b23c', accountHint: '国际站用户名；中国站写 cn:用户名' },
};

export const METRICS = [
  { value: 'first_ac', label: '首次 AC' },
  { value: 'daily_unique', label: '当日去重 AC' },
  { value: 'accepted_submissions', label: 'AC 提交' },
  { value: 'activity', label: '平台活动' },
] as const;

const LUOGU_COLORS: Record<string, string> = {
  '入门': '#ef5350', '普及-': '#f39c12', '普及': '#f4c430', '普及+/提高-': '#52b65a',
  '提高': '#13b6a7', '提高+/省选-': '#3182ce', '省选/NOI-': '#8b5cf6', 'NOI/NOI+/CTS': '#30343b',
};

export function difficultyColor(platform: Platform, label: string, order = 0) {
  if (platform === 'luogu') return LUOGU_COLORS[label] || '#68737d';
  if (platform === 'codeforces') {
    const rating = Number(label) || order;
    if (rating < 1200) return '#808080'; if (rating < 1400) return '#43a047';
    if (rating < 1600) return '#03a89e'; if (rating < 1900) return '#3282d8';
    if (rating < 2100) return '#aa47bc'; if (rating < 2400) return '#ff8c00'; return '#e53935';
  }
  if (platform === 'atcoder') {
    const rating = order || Number(label.split('–')[0]);
    if (rating < 400) return '#808080'; if (rating < 800) return '#9a6b3f';
    if (rating < 1200) return '#2e9d46'; if (rating < 1600) return '#00a8a8';
    if (rating < 2000) return '#3778c2'; if (rating < 2400) return '#c4a000';
    if (rating < 2800) return '#ef7d00'; return '#d94141';
  }
  if (platform === 'leetcode') {
    if (/easy/i.test(label)) return '#00af9b'; if (/medium/i.test(label)) return '#f0ad1c'; return '#ef4743';
  }
  if (platform === 'nowcoder') {
    const score = Number(label) || order;
    if (score < 1100) return '#6b7280'; if (score < 1600) return '#22a06b';
    if (score < 2100) return '#2878c7'; if (score < 2600) return '#8250df'; return '#d64545';
  }
  return PLATFORM_META[platform].accent;
}
