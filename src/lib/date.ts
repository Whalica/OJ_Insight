export function toDay(ts: number) {
  return new Date(ts * 1000).toISOString().slice(0, 10);
}

export const DEFAULT_TIME_ZONE = 'Asia/Shanghai';

export const TIME_ZONE_OPTIONS = [
  ['Asia/Shanghai', '中国标准时间 · UTC+8'],
  ['Asia/Tokyo', '东京 · UTC+9'],
  ['Asia/Singapore', '新加坡 · UTC+8'],
  ['Asia/Hong_Kong', '香港 · UTC+8'],
  ['Asia/Kolkata', '印度 · UTC+5:30'],
  ['Europe/London', '伦敦'],
  ['Europe/Paris', '巴黎'],
  ['America/New_York', '纽约'],
  ['America/Chicago', '芝加哥'],
  ['America/Denver', '丹佛'],
  ['America/Los_Angeles', '洛杉矶'],
  ['Australia/Sydney', '悉尼'],
  ['UTC', '协调世界时 · UTC'],
] as const;

export function isValidTimeZone(value: string) {
  try { new Intl.DateTimeFormat('en', { timeZone: value }).format(); return true; }
  catch { return false; }
}

export function initialTimeZone() {
  const saved = localStorage.getItem('oj-insight.time-zone') || '';
  return isValidTimeZone(saved) ? saved : DEFAULT_TIME_ZONE;
}

export function timeZoneLabel(timeZone: string) {
  const preset = TIME_ZONE_OPTIONS.find(([value]) => value === timeZone)?.[1];
  const zone = new Intl.DateTimeFormat('zh-CN', {
    timeZone, timeZoneName: 'shortOffset', hour: '2-digit', minute: '2-digit', hour12: false,
  }).formatToParts(new Date()).find((part) => part.type === 'timeZoneName')?.value;
  return preset ? `${preset}${zone && !preset.includes(zone) ? ` · ${zone}` : ''}` : `${timeZone}${zone ? ` · ${zone}` : ''}`;
}

export function formatDateTime(ts: number | null, timeZone = DEFAULT_TIME_ZONE) {
  if (!ts) return '从未';
  return new Intl.DateTimeFormat('zh-CN', {
    timeZone, year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false,
  }).format(new Date(ts * 1000));
}

export function formatTime(ts: number, timeZone = DEFAULT_TIME_ZONE) {
  return new Intl.DateTimeFormat('zh-CN', {
    timeZone, hour: '2-digit', minute: '2-digit', hour12: false,
  }).format(new Date(ts * 1000));
}

function dateKeyAt(value: Date | number, timeZone: string) {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone, year: 'numeric', month: '2-digit', day: '2-digit',
  }).formatToParts(value instanceof Date ? value : new Date(value));
  const get = (type: Intl.DateTimeFormatPartTypes) => parts.find((part) => part.type === type)?.value || '';
  return `${get('year')}-${get('month')}-${get('day')}`;
}

export function today(timeZone = DEFAULT_TIME_ZONE) {
  return dateKeyAt(new Date(), timeZone);
}

export function currentYear(timeZone = DEFAULT_TIME_ZONE) {
  return Number(today(timeZone).slice(0, 4));
}

export function hourInTimeZone(timeZone = DEFAULT_TIME_ZONE) {
  const hour = new Intl.DateTimeFormat('en-US', {
    timeZone, hour: '2-digit', hourCycle: 'h23',
  }).formatToParts(new Date()).find((part) => part.type === 'hour')?.value;
  return Number(hour || 0);
}

export function millisecondsUntilNextDay(timeZone = DEFAULT_TIME_ZONE) {
  const current = today(timeZone); const now = Date.now();
  let low = now; let high = now + 30 * 60 * 60 * 1000;
  while (high - low > 250) {
    const middle = Math.floor((low + high) / 2);
    if (dateKeyAt(middle, timeZone) === current) low = middle;
    else high = middle;
  }
  return Math.max(1000, high - now + 250);
}
