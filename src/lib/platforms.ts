import type { Platform } from '../types';

export const PLATFORM_ORDER: Platform[] = ['codeforces', 'atcoder', 'luogu', 'nowcoder', 'qoj', 'leetcode'];

export const PLATFORM_META: Record<Platform, { name: string; short: string; accent: string; accountHint: string; secretHint?: string }> = {
  codeforces: { name: 'Codeforces', short: 'CF', accent: '#5aa6e8', accountHint: 'Handle' },
  atcoder: { name: 'AtCoder', short: 'ATC', accent: '#d7d9dc', accountHint: '用户名' },
  luogu: { name: '洛谷', short: 'LG', accent: '#4fa9df', accountHint: '用户名或 UID' },
  nowcoder: { name: '牛客', short: 'NC', accent: '#44c767', accountHint: '数字 User ID' },
  qoj: { name: 'QOJ', short: 'QOJ', accent: '#48d0c0', accountHint: '用户名', secretHint: '可选：UOJSESSID=...（QOJ 当前需登录查看完整提交）' },
  leetcode: { name: 'LeetCode', short: 'LC', accent: '#f3b23c', accountHint: '国际站用户名；中国站写 cn:用户名' },
};

export const METRICS = [
  { value: 'first_ac', label: '首次 AC' },
  { value: 'daily_unique', label: '当日去重 AC' },
  { value: 'accepted_submissions', label: 'AC 提交' },
  { value: 'activity', label: '平台活动' },
] as const;
