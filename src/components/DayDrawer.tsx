import { ExternalLink, X } from 'lucide-react';
import { PLATFORM_META } from '../lib/platforms';
import { difficultyColor } from '../lib/platforms';
import { formatTime } from '../lib/date';
import type { DayDetail } from '../types';

export default function DayDrawer({ detail, loading, timeZone, onClose }: { detail: DayDetail | null; loading: boolean; timeZone: string; onClose: () => void }) {
  if (!detail && !loading) return null;
  return <div className="drawer-backdrop" onMouseDown={(e) => { if (e.target === e.currentTarget) onClose(); }}>
    <aside className="drawer">
      <div className="drawer-head"><div><small>DAY DETAIL</small><h3>{detail?.day || '读取中…'}</h3></div><button className="icon-btn" onClick={onClose}><X size={18} /></button></div>
      {loading && <div className="loading-block">正在读取本地记录…</div>}
      {detail && <>
        {detail.aggregates.map((a, i) => <div className="aggregate-note" key={`${a.platform}-${i}`}><strong>{PLATFORM_META[a.platform].name}</strong><span>{a.count} · {a.note}</span></div>)}
        <div className="submission-list">
          {detail.items.length === 0 && detail.aggregates.length === 0 && <div className="empty">这一天没有可显示的记录。</div>}
          {detail.items.map((item) => <article key={`${item.platform}-${item.submission_id}`}>
            <span className="oj-badge" style={{ borderColor: PLATFORM_META[item.platform].accent, color: PLATFORM_META[item.platform].accent }}>{PLATFORM_META[item.platform].short}</span>
            <div className="submission-main"><strong>{item.problem_id || item.problem_name}</strong><span>{item.problem_name}</span><small>{item.source_day ? `来源日期 ${item.source_day}` : formatTime(item.epoch_second, timeZone)}{item.language ? ` · ${item.language}` : ''}{item.account ? ` · ${item.account}` : ''}</small>{item.difficulty && <em><i style={{ background: difficultyColor(item.platform, item.difficulty) }} />{item.difficulty}</em>}</div>
            {item.problem_url && <a className="drawer-problem-button" href={item.problem_url} target="_blank" rel="noreferrer">前往题目<ExternalLink size={14} /></a>}
          </article>)}
        </div>
      </>}
    </aside>
  </div>;
}
