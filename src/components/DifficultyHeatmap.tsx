import { useMemo, useState } from 'react';
import type { DifficultyDayPoint, Platform } from '../types';

interface Props {
  platform: Platform;
  startDay: string;
  endDay: string;
  daily: DifficultyDayPoint[];
  onDay?: (day: string) => void;
  colorFor: (platform: Platform, label: string, order: number) => string;
}

const CELL = 12;
const GAP = 3;
const STEP = CELL + GAP;

function utc(day: string) { return new Date(`${day}T00:00:00Z`); }

export default function DifficultyHeatmap({ platform, startDay, endDay, daily, onDay, colorFor }: Props) {
  const [hover, setHover] = useState<{ day: string; label: string; x: number; y: number } | null>(null);
  const map = useMemo(() => new Map(daily.filter((x) => x.platform === platform).map((x) => [x.day, x])), [daily, platform]);
  const { days, weeks, months } = useMemo(() => {
    const start = utc(startDay); const end = utc(endDay); const gridStart = new Date(start);
    gridStart.setUTCDate(gridStart.getUTCDate() - gridStart.getUTCDay());
    const result: Array<{ day: string; dow: number; week: number; point?: DifficultyDayPoint }> = [];
    for (const d = new Date(gridStart); d <= end; d.setUTCDate(d.getUTCDate() + 1)) {
      const day = d.toISOString().slice(0, 10);
      if (day < startDay || day > endDay) continue;
      const diff = Math.floor((d.getTime() - gridStart.getTime()) / 86400000);
      result.push({ day, dow: d.getUTCDay(), week: Math.floor(diff / 7), point: map.get(day) });
    }
    const monthRows: Array<{ label: string; week: number }> = []; const seen = new Set<string>();
    for (const item of result) {
      const key = item.day.slice(0, 7);
      if (!seen.has(key)) { seen.add(key); monthRows.push({ label: `${Number(item.day.slice(5, 7))}月`, week: item.week }); }
    }
    return { days: result, weeks: Math.max(1, ...result.map((x) => x.week + 1)), months: monthRows };
  }, [startDay, endDay, map]);

  return <div className="heatmap-shell difficulty-map">
    <div className="heatmap-scroll"><div className="heatmap" style={{ width: 84 + weeks * STEP }}>
      <div className="month-labels">{months.map((m, i) => <span key={`${m.label}-${i}`} style={{ left: 46 + m.week * STEP }}>{m.label}</span>)}</div>
      <div className="weekday-labels"><span>一</span><span>三</span><span>五</span></div>
      <div className="cells" style={{ width: weeks * STEP, height: 7 * STEP }}>{days.map((item) => {
        const color = item.point ? colorFor(platform, item.point.label, item.point.order) : undefined;
        return <button key={item.day} className="heat-cell difficulty-cell" style={{ left: item.week * STEP, top: item.dow * STEP, width: CELL, height: CELL, background: color || 'var(--brick-empty)' }} aria-label={`${item.day}: ${item.point?.label || '无难度记录'}`} onClick={() => onDay?.(item.day)} onMouseEnter={(event) => setHover({ day: item.day, label: item.point?.label || '无难度记录', x: event.clientX, y: event.clientY })} onMouseMove={(event) => setHover((value) => value ? { ...value, x: event.clientX, y: event.clientY } : value)} onMouseLeave={() => setHover(null)} />;
      })}</div>
    </div></div>
    <div className="difficulty-map-note">每格显示当天 AC 题目的最高难度；点击可查看当天全部题目</div>
    {hover && <div className="heat-tooltip" style={{ left: hover.x + 14, top: hover.y + 14 }}><strong>{hover.day}</strong><span>{hover.label}</span></div>}
  </div>;
}
