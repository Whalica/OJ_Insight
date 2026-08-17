import { useCallback, useEffect, useMemo, useState } from 'react';
import { AlertTriangle, Check, ChevronDown, Download, RefreshCw, Save, Trash2 } from 'lucide-react';
import Sidebar from './components/Sidebar';
import Heatmap from './components/Heatmap';
import StatCards from './components/StatCards';
import DayDrawer from './components/DayDrawer';
import { api } from './lib/api';
import { exportHeatmap } from './lib/export';
import { currentYear, formatDateTime } from './lib/date';
import { METRICS, PLATFORM_META, PLATFORM_ORDER } from './lib/platforms';
import type { AccountConfig, DayDetail, Metric, Platform, Snapshot, SyncStatus } from './types';

type Page = 'overview' | 'export' | 'data' | 'settings' | Platform;

const emptySnapshot: Snapshot = {
  stats: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  daily: [], platforms: [], difficulty: [], recent: [], metric_available: true, warnings: [],
};

function yearRange(year: number) { return { start: `${year}-01-01`, end: `${year}-12-31` }; }

export default function App() {
  const [page, setPage] = useState<Page>('overview');
  const [year, setYear] = useState(currentYear());
  const [metric, setMetric] = useState<Metric>('activity');
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot);
  const [accounts, setAccounts] = useState<Record<string, AccountConfig>>({});
  const [statuses, setStatuses] = useState<SyncStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [toast, setToast] = useState<string>('');
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);

  const selectedPlatform: Platform | null = PLATFORM_ORDER.includes(page as Platform) ? page as Platform : null;
  const range = useMemo(() => yearRange(year), [year]);

  const notify = (message: string) => { setToast(message); window.setTimeout(() => setToast(''), 3200); };

  const loadAccounts = useCallback(async () => {
    const list = await api.getAccounts();
    const map: Record<string, AccountConfig> = {};
    for (const a of list) map[a.platform] = a;
    setAccounts(map);
  }, []);

  const loadStatuses = useCallback(async () => setStatuses(await api.getStatuses()), []);

  const loadSnapshot = useCallback(async () => {
    setLoading(true);
    try {
      const data = await api.snapshot(selectedPlatform, range.start, range.end, metric);
      setSnapshot(data);
    } catch (e) { notify(String(e)); }
    finally { setLoading(false); }
  }, [selectedPlatform, range.start, range.end, metric]);

  useEffect(() => { Promise.all([loadAccounts(), loadStatuses()]).catch((e) => notify(String(e))); }, [loadAccounts, loadStatuses]);
  useEffect(() => { loadSnapshot(); }, [loadSnapshot]);

  const syncOne = async (platform: Platform, full = false) => {
    setSyncing(platform);
    try {
      const result = await api.syncPlatform(platform, full);
      notify(`${PLATFORM_META[platform].name}: ${result.message}`);
      await Promise.all([loadSnapshot(), loadStatuses()]);
    } catch (e) { notify(`${PLATFORM_META[platform].name}: ${String(e)}`); await loadStatuses(); }
    finally { setSyncing(null); }
  };

  const syncAll = async () => {
    setSyncing('all');
    try {
      const results = await api.syncAll();
      notify(`同步完成：${results.filter((x) => x.status === 'ok').length}/${results.length}`);
      await Promise.all([loadSnapshot(), loadStatuses()]);
    } catch (e) { notify(String(e)); }
    finally { setSyncing(null); }
  };

  const openDay = async (day: string) => {
    setDayLoading(true); setDayDetail({ day, items: [], aggregates: [] });
    try { setDayDetail(await api.dayDetail(day, selectedPlatform)); }
    catch (e) { notify(String(e)); }
    finally { setDayLoading(false); }
  };

  return <div className="app-shell">
    <Sidebar page={page} onChange={setPage} />
    <main className="main">
      {page === 'settings' ? <SettingsPage accounts={accounts} onSaved={async () => { await loadAccounts(); notify('账号设置已保存'); }} /> :
       page === 'data' ? <DataPage statuses={statuses} syncing={syncing} onSync={syncOne} onSyncAll={syncAll} onCleared={async () => { await Promise.all([loadSnapshot(), loadStatuses()]); }} notify={notify} /> :
       page === 'export' ? <ExportPage accounts={accounts} metric={metric} /> :
       <DashboardPage platform={selectedPlatform} year={year} setYear={setYear} metric={metric} setMetric={setMetric} snapshot={snapshot} loading={loading} syncing={syncing} onSync={() => selectedPlatform ? syncOne(selectedPlatform) : syncAll()} onDay={openDay} />}
    </main>
    <DayDrawer detail={dayDetail} loading={dayLoading} onClose={() => setDayDetail(null)} />
    {toast && <div className="toast">{toast}</div>}
  </div>;
}

function DashboardPage({ platform, year, setYear, metric, setMetric, snapshot, loading, syncing, onSync, onDay }: {
  platform: Platform | null; year: number; setYear: (y: number) => void; metric: Metric; setMetric: (m: Metric) => void; snapshot: Snapshot; loading: boolean; syncing: string | null; onSync: () => void; onDay: (day: string) => void;
}) {
  const title = platform ? PLATFORM_META[platform].name : '总览';
  const years = Array.from({ length: currentYear() - 2009 }, (_, i) => 2010 + i);
  return <>
    <header className="topbar"><div><small>{platform ? 'PLATFORM ANALYTICS' : 'UNIFIED ANALYTICS'}</small><h1>{title}</h1><p>{platform ? '单平台活动、解题与难度数据' : '将多个 Online Judge 的训练轨迹汇总为一套统计。'}</p></div><button className="primary" onClick={onSync} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />{syncing ? '同步中' : '同步'}</button></header>
    <div className="toolbar">
      <label>年份<div className="select-wrap"><select value={year} onChange={(e) => setYear(Number(e.target.value))}>{years.map((y) => <option key={y}>{y}</option>)}</select><ChevronDown size={14} /></div></label>
      <label>砖墙口径<div className="select-wrap"><select value={metric} onChange={(e) => setMetric(e.target.value as Metric)}>{METRICS.map((m) => <option value={m.value} key={m.value}>{m.label}</option>)}</select><ChevronDown size={14} /></div></label>
      <span className="toolbar-note">日期统计基准：UTC+8</span>
    </div>
    {!snapshot.metric_available && <div className="warning"><AlertTriangle size={16} />当前平台没有该口径的逐日数据，已显示可用数据为空。</div>}
    {snapshot.warnings.map((w) => <div className="warning" key={w}><AlertTriangle size={16} />{w}</div>)}
    <StatCards stats={snapshot.stats} />
    <section className="panel heat-panel"><div className="panel-head"><div><small>ACTIVITY</small><h2>{year} 砖墙</h2></div>{loading && <span className="muted">读取中…</span>}</div><Heatmap year={year} daily={snapshot.daily} onDay={onDay} /></section>
    <div className="two-col">
      <section className="panel"><div className="panel-head"><div><small>PLATFORMS</small><h2>{platform ? '数据概况' : '平台分布'}</h2></div></div><PlatformTable rows={snapshot.platforms} /></section>
      <section className="panel"><div className="panel-head"><div><small>DIFFICULTY</small><h2>难度分布</h2></div></div><DifficultyList data={snapshot.difficulty} /></section>
    </div>
    <section className="panel"><div className="panel-head"><div><small>RECENT</small><h2>最近 AC</h2></div></div><RecentList items={snapshot.recent} /></section>
  </>;
}

function PlatformTable({ rows }: { rows: Snapshot['platforms'] }) {
  if (!rows.length) return <div className="empty">还没有本地数据。先到「设置」填账号，然后同步。</div>;
  return <div className="platform-table">{rows.map((x) => <div key={x.platform}><span className="oj-dot" style={{ background: PLATFORM_META[x.platform].accent }} /><strong>{PLATFORM_META[x.platform].name}</strong><span>{x.solved == null ? '—' : x.solved.toLocaleString()} solved</span><small>{x.active_days} active days</small></div>)}</div>;
}

function DifficultyList({ data }: { data: Snapshot['difficulty'] }) {
  if (!data.length) return <div className="empty">当前范围没有可靠的难度数据。</div>;
  const max = Math.max(...data.map((x) => x.count), 1);
  return <div className="difficulty-list">{data.map((x, i) => <div key={`${x.platform}-${x.label}-${i}`}><div><span className="oj-mini">{PLATFORM_META[x.platform].short}</span><strong>{x.label}</strong><em>{x.count}</em></div><i><b style={{ width: `${x.count / max * 100}%`, background: PLATFORM_META[x.platform].accent }} /></i></div>)}</div>;
}

function RecentList({ items }: { items: Snapshot['recent'] }) {
  if (!items.length) return <div className="empty">没有逐题记录。</div>;
  return <div className="recent-list">{items.map((x) => <a href={x.problem_url || '#'} target={x.problem_url ? '_blank' : undefined} rel="noreferrer" key={`${x.platform}-${x.submission_id}`}><span className="oj-badge" style={{ borderColor: PLATFORM_META[x.platform].accent, color: PLATFORM_META[x.platform].accent }}>{PLATFORM_META[x.platform].short}</span><div><strong>{x.problem_id || x.problem_name}</strong><span>{x.problem_name}</span></div><small>{new Date(x.epoch_second * 1000).toLocaleDateString('zh-CN')}{x.difficulty ? ` · ${x.difficulty}` : ''}</small></a>)}</div>;
}

function SettingsPage({ accounts, onSaved }: { accounts: Record<string, AccountConfig>; onSaved: () => void }) {
  const [form, setForm] = useState<Record<string, { account: string; secret: string }>>({});
  useEffect(() => {
    const next: Record<string, { account: string; secret: string }> = {};
    for (const p of PLATFORM_ORDER) next[p] = { account: accounts[p]?.account || '', secret: accounts[p]?.secret || '' };
    setForm(next);
  }, [accounts]);
  const saveAll = async () => { for (const p of PLATFORM_ORDER) await api.saveAccount(p, form[p]?.account || '', form[p]?.secret || ''); onSaved(); };
  return <><header className="topbar"><div><small>SETTINGS</small><h1>账号设置</h1><p>账号与可选登录凭据仅保存在本机 SQLite 数据库中。</p></div><button className="primary" onClick={saveAll}><Save size={16} />保存</button></header><section className="panel account-panel">{PLATFORM_ORDER.map((p) => <div className="account-row" key={p}><div className="account-name"><span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} /><div><strong>{PLATFORM_META[p].name}</strong><small>{PLATFORM_META[p].accountHint}</small></div></div><input value={form[p]?.account || ''} onChange={(e) => setForm({ ...form, [p]: { ...(form[p] || { secret: '' }), account: e.target.value } })} placeholder={PLATFORM_META[p].accountHint} />{p === 'qoj' ? <input type="password" value={form[p]?.secret || ''} onChange={(e) => setForm({ ...form, [p]: { ...(form[p] || { account: '' }), secret: e.target.value } })} placeholder={PLATFORM_META[p].secretHint} /> : <span className="account-spacer" />}</div>)}<div className="info-box"><strong>QOJ 登录说明</strong><p>QOJ 当前要求登录后才能查看完整提交列表。若要同步 QOJ 历史记录，可填写浏览器登录后的 <code>UOJSESSID=...</code> Cookie；留空时软件会明确标记为需要登录，不会把失败当成“0 条记录”。</p></div></section></>;
}

function DataPage({ statuses, syncing, onSync, onSyncAll, onCleared, notify }: { statuses: SyncStatus[]; syncing: string | null; onSync: (p: Platform, full?: boolean) => void; onSyncAll: () => void; onCleared: () => void; notify: (s: string) => void }) {
  const by = new Map(statuses.map((x) => [x.platform, x]));
  const clearOne = async (p: Platform) => { if (!confirm(`清空 ${PLATFORM_META[p].name} 的全部本地记录？账号设置会保留。`)) return; await api.clearPlatform(p); notify('已清空'); onCleared(); };
  const clearAll = async () => { if (!confirm('清空所有 OJ 的本地记录？此操作不可撤销，账号设置会保留。')) return; await api.clearAll(); notify('所有本地记录已清空'); onCleared(); };
  return <><header className="topbar"><div><small>DATA SOURCES</small><h1>同步与本地数据</h1><p>远端数据先进入 SQLite，日常使用优先读取本地缓存。</p></div><button className="primary" onClick={onSyncAll} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />同步全部</button></header><section className="panel source-list">{PLATFORM_ORDER.map((p) => { const s = by.get(p); const ok = s?.status === 'ok'; return <article key={p}><div className="source-id"><span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} /><div><strong>{PLATFORM_META[p].name}</strong><small>{s?.account || '未配置账号'}</small></div></div><div className={`source-state ${s?.status || 'idle'}`}>{ok ? <Check size={15} /> : s?.status === 'error' || s?.status === 'auth_required' ? <AlertTriangle size={15} /> : null}<div><strong>{s?.status || 'idle'}</strong><small>{s?.message || '尚未同步'} · {formatDateTime(s?.last_success || null)}</small></div></div><div className="source-actions"><button onClick={() => onSync(p)} disabled={!!syncing}><RefreshCw size={14} />增量</button><button onClick={() => onSync(p, true)} disabled={!!syncing}>重建</button><button className="danger-ghost" onClick={() => clearOne(p)}><Trash2 size={14} />清空</button></div></article>; })}</section><section className="danger-zone"><div><strong>清空全部数据</strong><p>删除六个平台的提交缓存、砖墙统计和同步状态，账号设置保留。</p></div><button onClick={clearAll}><Trash2 size={15} />清空所有 OJ 记录</button></section></>;
}

function ExportPage({ accounts, metric }: { accounts: Record<string, AccountConfig>; metric: Metric }) {
  const [from, setFrom] = useState(currentYear() - 2); const [to, setTo] = useState(currentYear()); const [scope, setScope] = useState<'all' | Platform>('all'); const [format, setFormat] = useState<'png' | 'svg'>('png'); const [busy, setBusy] = useState(false);
  const years = Array.from({ length: currentYear() - 2009 }, (_, i) => 2010 + i);
  const run = async () => { setBusy(true); try { const snap = await api.snapshot(scope === 'all' ? null : scope, `${from}-01-01`, `${to}-12-31`, metric); await exportHeatmap(`OJ Insight · ${scope === 'all' ? 'All OJs' : PLATFORM_META[scope].name} · ${from}–${to}`, snap.daily, from, to, format); } finally { setBusy(false); } };
  return <><header className="topbar"><div><small>EXPORT STUDIO</small><h1>导出总图</h1><p>按指定年份区间导出统一砖墙，支持 PNG 与无损 SVG。</p></div></header><div className="export-layout"><section className="panel export-form"><label>范围<div className="range-row"><select value={from} onChange={(e) => setFrom(Number(e.target.value))}>{years.map((y) => <option key={y}>{y}</option>)}</select><span>—</span><select value={to} onChange={(e) => setTo(Number(e.target.value))}>{years.map((y) => <option key={y}>{y}</option>)}</select></div></label><label>平台<select value={scope} onChange={(e) => setScope(e.target.value as 'all' | Platform)}><option value="all">所有 OJ 合并</option>{PLATFORM_ORDER.map((p) => <option key={p} value={p} disabled={!accounts[p]?.account}>{PLATFORM_META[p].name}</option>)}</select></label><label>格式<div className="segmented"><button className={format === 'png' ? 'active' : ''} onClick={() => setFormat('png')}>PNG</button><button className={format === 'svg' ? 'active' : ''} onClick={() => setFormat('svg')}>SVG</button></div></label><button className="primary export-btn" onClick={run} disabled={busy || from > to}><Download size={16} />{busy ? '生成中…' : '选择保存位置并导出'}</button></section><section className="panel export-preview"><div className="mock-export"><div><strong>OJ Insight</strong><small>{from} — {to}</small></div><div className="mock-grid">{Array.from({ length: 160 }).map((_, i) => <i key={i} className={`level-${(i * 17 + i * i) % 5}`} />)}</div><span>{scope === 'all' ? 'All OJs' : PLATFORM_META[scope].name}</span></div></section></div></>;
}
