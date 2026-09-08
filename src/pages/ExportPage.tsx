import { useState } from 'react';
import { Download } from 'lucide-react';
import PersonalInfoExport from '../components/PersonalInfoExport';
import { api } from '../lib/api';
import { currentYear } from '../lib/date';
import { exportHeatmap } from '../lib/export';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import { scopeRange, type AccountMap } from '../lib/ui';
import type { Metric, Platform } from '../types';

export default function ExportPage({ accounts, metric, timeZone }: { accounts: AccountMap; metric: Metric; timeZone: string }) {
  const [kind, setKind] = useState<'heatmap' | 'personal'>('heatmap');
  const [from, setFrom] = useState(currentYear(timeZone) - 2);
  const [to, setTo] = useState(currentYear(timeZone));
  const [until, setUntil] = useState(false);
  const [scope, setScope] = useState<'all' | Platform>('all');
  const [format, setFormat] = useState<'png' | 'svg'>('png');
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState('');
  const years = Array.from({ length: currentYear(timeZone) - 2009 }, (_, index) => 2010 + index);
  const run = async () => {
    setBusy(true);
    try {
      const range = until ? scopeRange('until', timeZone) : { start: `${from}-01-01`, end: `${to}-12-31` };
      const snap = await api.snapshot(scope === 'all' ? null : scope, range.start, range.end, metric, null, null, timeZone);
      await exportHeatmap(`OJ Insight · ${scope === 'all' ? '所有 OJ' : PLATFORM_META[scope].name} · ${until ? '至今（近一年）' : `${from}–${to}`}`, snap.daily, until ? Number(range.start.slice(0, 4)) : from, until ? Number(range.end.slice(0, 4)) : to, format, range.start, range.end);
    } finally { setBusy(false); }
  };
  const flatAccounts = PLATFORM_ORDER.flatMap((platform) => accounts[platform]);

  return <>
    <header className="topbar"><div><small>EXPORT</small><h1>导出</h1><p>活动砖图片与个人信息使用各自独立的导出范围。</p></div></header>
    <div className="settings-tabs export-tabs"><button className={kind === 'heatmap' ? 'active' : ''} onClick={() => setKind('heatmap')}>活动砖图片</button><button className={kind === 'personal' ? 'active' : ''} onClick={() => setKind('personal')}>个人信息</button></div>
    {kind === 'heatmap' ? <div className="export-layout"><section className="panel export-form"><label>范围<div className="segmented"><button className={!until ? 'active' : ''} onClick={() => setUntil(false)}>年份区间</button><button className={until ? 'active' : ''} onClick={() => setUntil(true)}>至今</button></div></label>{!until && <label>年份<div className="range-row"><select value={from} onChange={(event) => setFrom(Number(event.target.value))}>{years.map((year) => <option key={year}>{year}</option>)}</select><span>—</span><select value={to} onChange={(event) => setTo(Number(event.target.value))}>{years.map((year) => <option key={year}>{year}</option>)}</select></div></label>}<label>平台<select value={scope} onChange={(event) => setScope(event.target.value as 'all' | Platform)}><option value="all">所有 OJ 合并</option>{PLATFORM_ORDER.map((platform) => <option key={platform} value={platform} disabled={!accounts[platform].length}>{PLATFORM_META[platform].name}</option>)}</select></label><label>格式<div className="segmented"><button className={format === 'png' ? 'active' : ''} onClick={() => setFormat('png')}>PNG</button><button className={format === 'svg' ? 'active' : ''} onClick={() => setFormat('svg')}>SVG</button></div></label><button className="primary export-btn" onClick={run} disabled={busy || (!until && from > to)}><Download size={16} />{busy ? '生成中…' : '选择位置并导出'}</button></section><section className="panel export-preview"><div className="mock-export"><div><strong>OJ Insight · 活动砖</strong><small>{until ? '至今' : `${from} — ${to}`}</small></div><div className="mock-grid">{Array.from({ length: 160 }).map((_, index) => <i key={index} className={`level-${(index * 17 + index * index) % 5}`} />)}</div><span>{scope === 'all' ? '所有 OJ' : PLATFORM_META[scope].name}</span></div></section></div> : <PersonalInfoExport accounts={flatAccounts} notify={(message) => { setNotice(message); window.setTimeout(() => setNotice(''), 3600); }} />}
    {notice && <div className="toast">{notice}</div>}
  </>;
}
