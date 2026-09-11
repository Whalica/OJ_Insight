import type { Platform } from '../types';

export const PLATFORM_ORDER: Platform[] = ['codeforces', 'atcoder', 'luogu', 'nowcoder', 'qoj', 'leetcode'];

export const PLATFORM_META: Record<Platform, { name: string; short: string; accent: string; accountHint: string; secretHint?: string }> = {
  codeforces: { name: 'Codeforces', short: 'CF', accent: '#5aa6e8', accountHint: 'Handle' },
  atcoder: { name: 'AtCoder', short: 'ATC', accent: 'var(--atcoder-accent)', accountHint: '用户名' },
  luogu: { name: 'Luogu', short: 'LG', accent: '#2d9cdb', accountHint: '用户名或数字 UID' },
  nowcoder: { name: 'NowCoder', short: 'NC', accent: '#00b96b', accountHint: '个人主页 users/ 后的数字 User ID', secretHint: '可选：牛客网页 Cookie（用于同步 Tracker 完成记录）' },
  qoj: { name: 'QOJ', short: 'QOJ', accent: '#48d0c0', accountHint: '用户名', secretHint: '可选：UOJSESSID=...（QOJ 当前需登录查看完整提交）' },
  leetcode: { name: 'LeetCode', short: 'LC', accent: '#f3b23c', accountHint: '国际站用户名；中国站写 cn:用户名', secretHint: '可选：对应站点 Cookie（中国站活动与最近 AC 兜底）' },
};

export const METRICS = [
  { value: 'first_ac', label: '首次 AC' },
  { value: 'daily_unique', label: '当日去重 AC' },
  { value: 'accepted_submissions', label: 'AC 提交' },
  { value: 'activity', label: '平台活动' },
] as const;

const LUOGU_COLORS: Record<string, string> = {
  '入门': '#FE4C61', '普及-': '#F39C11', '普及': '#FFC116', '普及+/提高-': '#52C41A',
  '提高': '#00B5AD', '提高+/省选-': '#3498DB', '省选/NOI-': '#9D3DCF', 'NOI/NOI+/CTS': '#0E1D69',
};

export function difficultyColor(platform: Platform, label: string, order = 0) {
  if (label === '未评级') return 'var(--unrated-difficulty)';
  if (platform === 'luogu') return LUOGU_COLORS[label] || '#68737d';
  if (platform === 'codeforces') {
    const rating = Number(label) || order;
    if (rating < 1200) return '#9AA4AD'; if (rating < 1400) return '#43B95C';
    if (rating < 1600) return '#20B8B0'; if (rating < 1900) return '#4C8DDB';
    if (rating < 2100) return '#A45BD4'; if (rating < 2400) return '#F29A2E'; return '#E85757';
  }
  if (platform === 'atcoder') {
    const parsed = Number(label.split(/[–-]/)[0]);
    const rating = Number.isFinite(parsed) ? parsed : order * 400;
    if (rating < 400) return 'var(--atcoder-difficulty-gray)'; if (rating < 800) return '#A36F48';
    if (rating < 1200) return '#43B95C'; if (rating < 1600) return '#20B8B0';
    if (rating < 2000) return '#4C8DDB'; if (rating < 2400) return '#D9B72C';
    if (rating < 2800) return '#F29A2E'; return '#E85757';
  }
  if (platform === 'leetcode') {
    if (/easy/i.test(label)) return '#00B8A3'; if (/medium/i.test(label)) return '#FFC01E'; return '#FF375F';
  }
  if (platform === 'nowcoder') {
    const score = Number(label) || order;
    if (score < 1100) return '#6b7280'; if (score < 1600) return '#22a06b';
    if (score < 2100) return '#2878c7'; if (score < 2600) return '#8250df'; return '#d64545';
  }
  if (platform === 'qoj') {
    if (/金|gold/i.test(label)) return '#e5b94e';
    if (/银|silver/i.test(label)) return '#aab5c1';
    if (/铜|bronze/i.test(label)) return '#bd7a4e';
    if (/铁|iron/i.test(label)) return '#6f7b87';
  }
  return PLATFORM_META[platform].accent;
}
