import { useMemo, useState } from 'react';
import type { DailyPoint } from '../types';

interface Props {
  year: number;
  daily: DailyPoint[];
  onDay?: (day: string) => void;
}

const CELL = 12;
const GAP = 3;
const STEP = CELL + GAP;
const LEVELS = ['level-0', 'level-1', 'level-2', 'level-3', 'level-4'];

function makeDays(year: number, data: Map<string, number>) {
  const jan1 = new Date(Date.UTC(year, 0, 1));
  const firstSunday = new Date(jan1);
  firstSunday.setUTCDate(jan1.getUTCDate() - jan1.getUTCDay());
  const dec31 = new Date(Date.UTC(year, 11, 31));
  const out: Array<{ day: string; dow: number; week: number; count: number }> = [];
  for (const d = new Date(firstSunday); d <= dec31; d.setUTCDate(d.getUTCDate() + 1)) {
    if (d.getUTCFullYear() !== year) continue;
    const diff = Math.floor((d.getTime() - firstSunday.getTime()) / 86400000);
    const day = d.toISOString().slice(0, 10);
    out.push({ day, dow: d.getUTCDay(), week: Math.floor(diff / 7), count: data.get(day) || 0 });
  }
  return out;
}

export default function Heatmap({ year, daily, onDay }: Props) {
  const [hover, setHover] = useState<{ day: string; count: number; x: number; y: number } | null>(null);
  const map = useMemo(() => new Map(daily.map((x) => [x.day, x.count])), [daily]);
  const days = useMemo(() => makeDays(year, map), [year, map]);
  const max = Math.max(1, ...days.map((d) => d.count));
  const lvl = (n: number) => {
    if (!n) return 0;
    const x = n / max;
    if (x <= .2) return 1;
    if (x <= .45) return 2;
    if (x <= .72) return 3;
    return 4;
  };
  const months = useMemo(() => {
    const seen = new Set<number>();
    const arr: Array<{ month: string; week: number }> = [];
    for (const d of days) {
      const m = Number(d.day.slice(5, 7));
      if (!seen.has(m)) { seen.add(m); arr.push({ month: `${m}月`, week: d.week }); }
    }
    return arr;
  }, [days]);

  return (
    <div className="heatmap-shell">
      <div className="heatmap-scroll">
        <div className="heatmap" style={{ width: 84 + 54 * STEP }}>
          <div className="month-labels">
            {months.map((m) => <span key={m.month} style={{ left: 46 + m.week * STEP }}>{m.month}</span>)}
          </div>
          <div className="weekday-labels"><span>一</span><span>三</span><span>五</span></div>
          <div className="cells" style={{ width: 54 * STEP, height: 7 * STEP }}>
            {days.map((d) => (
              <button
                key={d.day}
                className={`heat-cell ${LEVELS[lvl(d.count)]}`}
                style={{ left: d.week * STEP, top: d.dow * STEP, width: CELL, height: CELL }}
                aria-label={`${d.day}: ${d.count}`}
                onClick={() => onDay?.(d.day)}
                onMouseEnter={(e) => setHover({ day: d.day, count: d.count, x: e.clientX, y: e.clientY })}
                onMouseMove={(e) => setHover((v) => v ? { ...v, x: e.clientX, y: e.clientY } : v)}
                onMouseLeave={() => setHover(null)}
              />
            ))}
          </div>
        </div>
      </div>
      <div className="heat-legend"><span>少</span>{LEVELS.map((x) => <i key={x} className={x} />)}<span>多</span></div>
      {hover && <div className="heat-tooltip" style={{ left: hover.x + 14, top: hover.y + 14 }}><strong>{hover.day}</strong><span>{hover.count} 条</span></div>}
    </div>
  );
}
