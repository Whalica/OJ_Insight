import { Activity, CalendarDays, Flame, Layers3, Trophy } from 'lucide-react';
import type { SnapshotStats } from '../types';

export default function StatCards({ stats }: { stats: SnapshotStats }) {
  const cards = [
    { label: 'Solved', value: stats.solved, sub: '已解题目', icon: Trophy },
    { label: 'AC Submissions', value: stats.accepted_submissions, sub: 'AC 提交', icon: Layers3 },
    { label: 'Active Days', value: stats.active_days, sub: '活跃天数', icon: CalendarDays },
    { label: 'Longest Streak', value: stats.longest_streak, sub: `当前 ${stats.current_streak} 天`, icon: Flame },
    { label: 'Peak Day', value: stats.peak_count, sub: stats.peak_day || '—', icon: Activity },
  ];
  return <div className="stats-grid">{cards.map(({ label, value, sub, icon: Icon }) => <div className="stat-card" key={label}><div><span>{label}</span><strong>{value.toLocaleString()}</strong><small>{sub}</small></div><Icon size={19} /></div>)}</div>;
}
