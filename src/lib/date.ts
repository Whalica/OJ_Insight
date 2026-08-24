export function toDay(ts: number) {
  return new Date(ts * 1000).toISOString().slice(0, 10);
}

export function formatDateTime(ts: number | null) {
  if (!ts) return '从未';
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', hour12: false,
  }).format(new Date(ts * 1000));
}

export function today() {
  return new Intl.DateTimeFormat('en-CA', {
    timeZone: 'Asia/Shanghai', year: 'numeric', month: '2-digit', day: '2-digit',
  }).format(new Date());
}

export function currentYear() {
  return Number(today().slice(0, 4));
}
