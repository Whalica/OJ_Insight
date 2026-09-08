import { ExternalLink, X } from 'lucide-react';
import { formatDateTime } from '../lib/date';
import { difficultyColor, PLATFORM_META } from '../lib/platforms';
import type { DifficultyDetail } from '../types';

export default function DifficultyDrawer({ detail, loading, timeZone, onClose }: { detail: DifficultyDetail | null; loading: boolean; timeZone: string; onClose: () => void }) {
  if (!detail && !loading) return null;
  return <div className="drawer-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose(); }}>
    <aside className="drawer" role="dialog" aria-modal="true" aria-label="难度题目列表">
      <div className="drawer-head"><div><small>DIFFICULTY DETAIL</small><h3>{detail ? `${PLATFORM_META[detail.platform].name} · ${detail.label}` : '读取中…'}</h3></div><button className="icon-btn" aria-label="关闭难度题目列表" onClick={onClose}><X size={18} /></button></div>
      {loading && <div className="loading-block">正在读取本地题目记录…</div>}
      {detail && <>
        <div className="aggregate-note"><strong>{detail.count.toLocaleString()} 道去重题目</strong><span>按最近 AC 时间排序；相同账号下的同一道题只显示一次。</span></div>
        {detail.note && <div className="warning">{detail.note}</div>}
        <div className="submission-list">
          {!detail.items.length && <div className="empty">当前数据源没有可显示的逐题记录。</div>}
          {detail.items.map((item) => <article key={`${item.platform}-${item.account}-${item.problem_key}`}>
            <span className="oj-badge" style={{ borderColor: PLATFORM_META[item.platform].accent, color: PLATFORM_META[item.platform].accent }}>{PLATFORM_META[item.platform].short}</span>
            <div className="submission-main"><strong>{item.problem_id || item.problem_name}</strong><span>{item.problem_name}</span><small>{formatDateTime(item.epoch_second, timeZone)}{item.language ? ` · ${item.language}` : ''}{item.account ? ` · ${item.account}` : ''}</small>{item.difficulty && <em><i style={{ background: difficultyColor(item.platform, item.difficulty) }} />{detail.label}</em>}</div>
            {item.problem_url && <a className="drawer-problem-button" href={item.problem_url} target="_blank" rel="noreferrer">前往题目<ExternalLink size={14} /></a>}
          </article>)}
        </div>
      </>}
    </aside>
  </div>;
}
