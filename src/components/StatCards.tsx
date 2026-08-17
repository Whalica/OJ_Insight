import { Activity, CalendarDays, Flame, Layers3, Trophy } from 'lucide-react';
import type { SnapshotStats } from '../types';

export default function StatCards({ stats }: { stats: SnapshotStats }) {
  const cards = [
    { label: '解题数', value: stats.solved, sub: '至少 AC 一次的平台内不同题', icon: Trophy },
    { label: 'AC 提交', value: stats.accepted_submissions, sub: '可获取逐题记录的 AC 提交', icon: Layers3 },
    { label: '活跃天数', value: stats.active_days, sub: '有活动记录的日期数', icon: CalendarDays },
    { label: '最长连续', value: stats.longest_streak, sub: `当前连续 ${stats.current_streak} 天`, icon: Flame },
    { label: '峰值日', value: stats.peak_count, sub: stats.peak_day || '—', icon: Activity },
  ];
  return <div className="stats-grid">{cards.map(({ label, value, sub, icon: Icon }) => <div className="stat-card" key={label} title={sub}><div><span>{label}</span><strong>{value.toLocaleString()}</strong><small>{sub}</small></div><Icon size={19} /></div>)}</div>;
}
