import { useCallback, useEffect, useMemo, useState } from 'react';
import { AlertTriangle, Check, ChevronDown, ChevronLeft, ChevronRight, Download, ExternalLink, Github, RefreshCw, Save, Trash2 } from 'lucide-react';
import Sidebar from './components/Sidebar';
import Heatmap from './components/Heatmap';
import StatCards from './components/StatCards';
import DayDrawer from './components/DayDrawer';
import { api } from './lib/api';
import { exportHeatmap } from './lib/export';
import { currentYear, formatDateTime, today } from './lib/date';
import { METRICS, PLATFORM_META, PLATFORM_ORDER } from './lib/platforms';
import type { AccountConfig, DayDetail, Metric, Platform, Snapshot, SyncStatus } from './types';

type Page = 'overview' | 'export' | 'data' | 'settings' | 'about' | Platform;
type TimeScope = number | 'until';

const emptySnapshot: Snapshot = {
  stats: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  career: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  daily: [], platforms: [], difficulty: [], recent: [], metric_available: true, warnings: [],
};

function yearRange(year: number) { return { start: `${year}-01-01`, end: `${year}-12-31` }; }
function scopeRange(scope: TimeScope) {
  if (scope !== 'until') return yearRange(scope);
  const end = new Date(`${today()}T00:00:00Z`); const start = new Date(end); start.setUTCDate(start.getUTCDate() - 364);
  return { start: start.toISOString().slice(0, 10), end: end.toISOString().slice(0, 10) };
}

export default function App() {
  const [page, setPage] = useState<Page>('overview');
  const [timeScope, setTimeScope] = useState<TimeScope>('until');
  const [metric, setMetric] = useState<Metric>('activity');
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot);
  const [accounts, setAccounts] = useState<Record<string, AccountConfig>>({});
  const [statuses, setStatuses] = useState<SyncStatus[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [syncProgress, setSyncProgress] = useState<{ done: number; total: number; added: number; failed: number } | null>(null);
  const [toast, setToast] = useState<string>('');
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);

  const selectedPlatform: Platform | null = PLATFORM_ORDER.includes(page as Platform) ? page as Platform : null;
  const range = useMemo(() => scopeRange(timeScope), [timeScope]);

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
      const platforms = PLATFORM_ORDER;
      let done=0,added=0,failed=0;setSyncProgress({done,total:platforms.length,added,failed});
      for (const platform of platforms) {
        if (accounts[platform]?.account?.trim()) {
          try { const result=await api.syncPlatform(platform); added+=result.inserted; } catch { failed+=1; }
        }
        done+=1;setSyncProgress({done,total:platforms.length,added,failed});await loadStatuses();
      }
      notify(`同步检查完成：${done}/${done} · 新增 ${added} · 失败 ${failed}`);
      await Promise.all([loadSnapshot(), loadStatuses()]);
    } catch (e) { notify(String(e)); }
    finally { setSyncing(null); window.setTimeout(()=>setSyncProgress(null),2500); }
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
       page === 'about' ? <AboutPage /> :
       <DashboardPage platform={selectedPlatform} timeScope={timeScope} setTimeScope={setTimeScope} range={range} metric={metric} setMetric={setMetric} snapshot={snapshot} loading={loading} syncing={syncing} syncProgress={syncProgress} onSync={() => selectedPlatform ? syncOne(selectedPlatform) : syncAll()} onDay={openDay} onPlatform={(p)=>setPage(p)} />}
    </main>
    <DayDrawer detail={dayDetail} loading={dayLoading} onClose={() => setDayDetail(null)} />
    {toast && <div className="toast">{toast}</div>}
  </div>;
}

function DashboardPage({ platform, timeScope, setTimeScope, range, metric, setMetric, snapshot, loading, syncing, syncProgress, onSync, onDay, onPlatform }: {
  platform: Platform | null; timeScope: TimeScope; setTimeScope: (y: TimeScope) => void; range:{start:string;end:string}; metric: Metric; setMetric: (m: Metric) => void; snapshot: Snapshot; loading: boolean; syncing: string | null; syncProgress:{done:number;total:number;added:number;failed:number}|null; onSync: () => void; onDay: (day: string) => void; onPlatform:(p:Platform)=>void;
}) {
  const title = platform ? PLATFORM_META[platform].name : '总览';
  const years = Array.from({ length: currentYear() - 2009 }, (_, i) => currentYear()-i);
  const label=timeScope==='until'?'至今（最近一年）':String(timeScope);
  const move=(delta:number)=>{if(timeScope!=='until')setTimeScope(Math.min(currentYear(),Math.max(2010,timeScope+delta)));};
  return <>
    <header className="topbar"><div><small>{platform ? 'PLATFORM ANALYTICS' : 'UNIFIED ANALYTICS'}</small><h1>{title}</h1><p>{platform ? '查看单平台生涯、当前范围、活动图与难度数据' : '将多个 Online Judge 的训练轨迹汇总为一套统计。'}</p></div><button className="primary" onClick={onSync} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />{syncProgress?`${syncProgress.done}/${syncProgress.total}`:syncing?'同步中':'同步'}</button></header>
    {syncProgress&&<div className="sync-banner"><strong>正在同步 {syncProgress.done} / {syncProgress.total}</strong><span>新增 {syncProgress.added} 条 · {syncProgress.failed} 个平台失败</span><i><b style={{width:`${syncProgress.total?syncProgress.done/syncProgress.total*100:0}%`}}/></i></div>}
    <div className="toolbar">
      <label>时间范围<div className="year-control"><button onClick={()=>move(-1)} disabled={timeScope==='until'||timeScope<=2010}><ChevronLeft size={15}/></button><div className="select-wrap"><select value={timeScope} onChange={(e)=>setTimeScope(e.target.value==='until'?'until':Number(e.target.value))}><option value="until">至今（最近一年）</option>{years.map(y=><option value={y} key={y}>{y}</option>)}</select><ChevronDown size={14}/></div><button onClick={()=>move(1)} disabled={timeScope==='until'||timeScope>=currentYear()}><ChevronRight size={15}/></button></div></label>
      <label>Activity 口径<div className="select-wrap"><select value={metric} onChange={(e) => setMetric(e.target.value as Metric)}>{METRICS.map((m) => <option value={m.value} key={m.value}>{m.label}</option>)}</select><ChevronDown size={14} /></div></label>
      <span className="toolbar-note">日期统计基准：UTC+8</span>
    </div>
    {!snapshot.metric_available && <div className="warning"><AlertTriangle size={16} />当前平台没有该口径的逐日数据，已显示可用数据为空。</div>}
    {snapshot.warnings.map((w) => <div className="warning" key={w}><AlertTriangle size={16} />{w}</div>)}
    <div className="section-title"><small>CAREER · 不随时间范围变化</small><h2>{platform?`${title} 生涯统计`:'生涯统计'}</h2></div><StatCards stats={snapshot.career} />
    <div className="section-title"><small>CURRENT RANGE</small><h2>当前范围 · {label}</h2></div><StatCards stats={snapshot.stats} />
    <section className="panel heat-panel"><div className="panel-head"><div><small>ACTIVITY · {range.start} — {range.end}</small><h2>活动图 · {label}</h2></div>{loading && <span className="muted">读取中…</span>}</div><Heatmap startDay={range.start} endDay={range.end} daily={snapshot.daily} onDay={onDay} /></section>
    <div className="two-col">
      <section className="panel"><div className="panel-head"><div><small>PLATFORM SUMMARY</small><h2>{platform ? '本平台概况' : '平台概览'}</h2></div></div><PlatformTable rows={snapshot.platforms} onSelect={onPlatform} /></section>
      <section className="panel"><div className="panel-head"><div><small>DIFFICULTY PROFILE</small><h2>难度分布</h2></div></div><DifficultyProfile data={snapshot.difficulty} /></section>
    </div>
    <section className="panel"><div className="panel-head"><div><small>RECENT</small><h2>最近 AC</h2></div></div><RecentList items={snapshot.recent} /></section>
  </>;
}

function PlatformTable({ rows,onSelect }: { rows: Snapshot['platforms'];onSelect:(p:Platform)=>void }) {
  if (!rows.length) return <div className="empty">还没有本地数据。先到「设置」填账号，然后同步。</div>;
  return <div className="platform-table">{rows.map((x) => <button key={x.platform} onClick={()=>onSelect(x.platform)}><span className="oj-dot" style={{ background: PLATFORM_META[x.platform].accent }} /><strong>{PLATFORM_META[x.platform].name}</strong><span>{x.solved == null ? '暂无解题数' : `已解 ${x.solved.toLocaleString()} 题`}</span><small>缓存 {x.cached_records} 条 · {syncStatusLabel(x.status)}</small></button>)}</div>;
}

function DifficultyProfile({ data }: { data: Snapshot['difficulty'] }) {
  const available=PLATFORM_ORDER.filter(p=>data.some(x=>x.platform===p));const[first]=available;const[selected,setSelected]=useState<Platform|undefined>(first);
  useEffect(()=>{if(!selected||!available.includes(selected))setSelected(first);},[first,selected,available.join('|')]);
  const active = selected && available.includes(selected) ? selected : first;
  if (!data.length || !active) return <div className="empty">当前范围没有可靠的难度数据。</div>;
  const shown=data.filter(x=>x.platform===active);
  if (!shown.length) return <div className="empty">当前范围没有可靠的难度数据。</div>;
  const max=Math.max(...shown.map(x=>x.count),1);const total=shown.reduce((a,b)=>a+b.count,0);
  const peak=shown.reduce((a,b)=>a.count>b.count?a:b);
  let accumulated=0;const median=shown.find(x=>(accumulated+=x.count)>=Math.ceil(total/2))?.label||'暂无';
  const highest=[...shown].reverse().find(x=>x.count>0)?.label||'暂无';
  return <><div className="difficulty-tabs">{available.map(p=><button className={p===active?'active':''} onClick={()=>setSelected(p)} key={p}>{PLATFORM_META[p].short}</button>)}</div><div className="histogram">{shown.map((x,i)=><div key={`${x.platform}-${x.label}-${i}`} title={`${x.label}：${x.count}`}><strong>{x.count}</strong><i style={{height:`${Math.max(8,x.count/max*70)}%`}}/><span>{x.label}</span></div>)}</div><div className="difficulty-summary"><span>中位难度<strong>{median}</strong></span><span>峰值区间<strong>{peak.label}</strong></span><span>最高难度<strong>{highest}</strong></span><span>有难度题目<strong>{total}</strong></span></div></>;
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
  return <><header className="topbar"><div><small>SETTINGS</small><h1>账号设置</h1><p>账号与可选登录凭据仅保存在本机 SQLite 数据库中。</p></div><button className="primary" onClick={saveAll}><Save size={16} />保存</button></header><section className="panel account-panel">{PLATFORM_ORDER.map((p) => <div className="account-row" key={p}><div className="account-name"><span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} /><div><strong>{PLATFORM_META[p].name}</strong><small>{PLATFORM_META[p].accountHint}</small></div></div><input value={form[p]?.account || ''} onChange={(e) => setForm({ ...form, [p]: { ...(form[p] || { secret: '' }), account: e.target.value } })} placeholder={PLATFORM_META[p].accountHint} />{p === 'qoj' ? <input type="password" value={form[p]?.secret || ''} onChange={(e) => setForm({ ...form, [p]: { ...(form[p] || { account: '' }), secret: e.target.value } })} placeholder={PLATFORM_META[p].secretHint} /> : <span className="account-spacer" />}</div>)}<div className="info-box"><strong>QOJ 登录说明</strong><p>可填写完整 <code>UOJSESSID=value</code>，也可只粘贴 Cookie 的 value，应用会自动补齐名称。日志会自动脱敏；未登录、过期、无提交与页面结构变化会分别提示。</p></div><div className="info-box"><strong>LeetCode 中国站</strong><p>国际站直接填用户名；中国站填写 <code>cn:用户名</code>。两站使用独立 GraphQL provider。</p></div></section></>;
}

function DataPage({ statuses, syncing, onSync, onSyncAll, onCleared, notify }: { statuses: SyncStatus[]; syncing: string | null; onSync: (p: Platform, full?: boolean) => void; onSyncAll: () => void; onCleared: () => void; notify: (s: string) => void }) {
  const by = new Map(statuses.map((x) => [x.platform, x]));
  const clearOne = async (p: Platform) => { if (!confirm(`清空 ${PLATFORM_META[p].name} 的全部本地记录？账号设置会保留。`)) return; await api.clearPlatform(p); notify('已清空'); onCleared(); };
  const clearAll = async () => { if (!confirm('清空所有 OJ 的本地记录？此操作不可撤销，账号设置会保留。')) return; await api.clearAll(); notify('所有本地记录已清空'); onCleared(); };
  return <><header className="topbar"><div><small>DATA SOURCES</small><h1>同步与本地数据</h1><p>缓存数据与本次同步结果彼此独立；单站失败不会删除旧数据。</p></div><button className="primary" onClick={onSyncAll} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />同步全部</button></header><section className="panel source-list">{PLATFORM_ORDER.map((p) => { const s = by.get(p); const ok = s?.status === 'ok'; return <article key={p}><div className="source-id"><span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} /><div><strong>{PLATFORM_META[p].name}</strong><small>{s?.account || '未配置账号'} · 本地缓存 {s?.cached_records||0} 条</small></div></div><div className={`source-state ${s?.status || 'idle'}`}>{ok ? <Check size={15} /> : s?.status === 'error' || s?.status === 'auth_required' ? <AlertTriangle size={15} /> : null}<div><strong>最近同步 · {syncStatusLabel(s?.status)}</strong><small>{s?.message || '尚未同步'} · 上次成功 {formatDateTime(s?.last_success || null)}</small></div></div><div className="source-actions"><button onClick={() => onSync(p)} disabled={!!syncing}><RefreshCw size={14} />增量</button><button onClick={() => onSync(p, true)} disabled={!!syncing}>重建</button><button className="danger-ghost" onClick={() => clearOne(p)}><Trash2 size={14} />清空</button></div></article>; })}</section><section className="danger-zone"><div><strong>清空全部数据</strong><p>删除六个平台的提交缓存、活动图统计和同步状态，账号设置保留。</p></div><button onClick={clearAll}><Trash2 size={15} />清空所有 OJ 记录</button></section></>;
}

function syncStatusLabel(status?: string) {
  return ({ idle:'未同步', syncing:'同步中', ok:'成功', error:'失败', auth_required:'需要重新登录' } as Record<string,string>)[status || 'idle'] || status || '未同步';
}

function ExportPage({ accounts, metric }: { accounts: Record<string, AccountConfig>; metric: Metric }) {
  const [from, setFrom] = useState(currentYear() - 2); const [to, setTo] = useState(currentYear()); const [until,setUntil]=useState(false); const [scope, setScope] = useState<'all' | Platform>('all'); const [format, setFormat] = useState<'png' | 'svg'>('png'); const [busy, setBusy] = useState(false);
  const years = Array.from({ length: currentYear() - 2009 }, (_, i) => 2010 + i);
  const run = async () => { setBusy(true); try { const r=until?scopeRange('until'):{start:`${from}-01-01`,end:`${to}-12-31`}; const snap = await api.snapshot(scope === 'all' ? null : scope,r.start,r.end,metric); await exportHeatmap(`OJ Insight · ${scope === 'all' ? '所有 OJ' : PLATFORM_META[scope].name} · ${until?'至今（最近一年）':`${from}–${to}`}`, snap.daily, until?Number(r.start.slice(0,4)):from,until?Number(r.end.slice(0,4)):to, format,r.start,r.end); } finally { setBusy(false); } };
  return <><header className="topbar"><div><small>EXPORT STUDIO</small><h1>活动图导出</h1><p>指定年份区间或最近一年，支持所有 OJ 合并/单 OJ、PNG/SVG。</p></div></header><div className="export-layout"><section className="panel export-form"><label>范围<div className="segmented"><button className={!until?'active':''} onClick={()=>setUntil(false)}>年份区间</button><button className={until?'active':''} onClick={()=>setUntil(true)}>至今（最近一年）</button></div></label>{!until&&<label>年份<div className="range-row"><select value={from} onChange={(e) => setFrom(Number(e.target.value))}>{years.map((y) => <option key={y}>{y}</option>)}</select><span>—</span><select value={to} onChange={(e) => setTo(Number(e.target.value))}>{years.map((y) => <option key={y}>{y}</option>)}</select></div></label>}<label>平台<select value={scope} onChange={(e) => setScope(e.target.value as 'all' | Platform)}><option value="all">所有 OJ 合并</option>{PLATFORM_ORDER.map((p) => <option key={p} value={p} disabled={!accounts[p]?.account}>{PLATFORM_META[p].name}</option>)}</select></label><label>格式<div className="segmented"><button className={format === 'png' ? 'active' : ''} onClick={() => setFormat('png')}>PNG</button><button className={format === 'svg' ? 'active' : ''} onClick={() => setFormat('svg')}>SVG</button></div></label><button className="primary export-btn" onClick={run} disabled={busy || (!until&&from > to)}><Download size={16} />{busy ? '生成中…' : '选择保存位置并导出'}</button></section><section className="panel export-preview"><div className="mock-export"><div><strong>OJ Insight · 活动图</strong><small>{until?'至今（最近一年）':`${from} — ${to}`}</small></div><div className="mock-grid">{Array.from({ length: 160 }).map((_, i) => <i key={i} className={`level-${(i * 17 + i * i) % 5}`} />)}</div><span>{scope === 'all' ? '所有 OJ' : PLATFORM_META[scope].name}</span></div></section></div></>;
}

function AboutPage(){
  const[update,setUpdate]=useState<Awaited<ReturnType<typeof api.checkForUpdates>>|null>(null);const[checking,setChecking]=useState(false);const[error,setError]=useState('');
  const check=async()=>{setChecking(true);setError('');try{setUpdate(await api.checkForUpdates());}catch(e){setError(String(e));}finally{setChecking(false);}};
  return <><header className="topbar"><div><small>ABOUT</small><h1>关于 OJ Insight</h1><p>统一整理与呈现多个 Online Judge 的训练数据。</p></div></header><section className="panel about-card"><div className="about-mark">OI</div><h2>OJ Insight</h2><p>版本 0.2.0</p>{update?<div className={update.updateAvailable?'update-state available':'update-state'}><strong>{update.updateAvailable?`发现新版本 · v${update.latestVersion}`:'当前已是最新版本'}</strong>{update.updateAvailable&&<button onClick={()=>api.openExternal(update.releaseUrl)}>查看发布页 <ExternalLink size={14}/></button>}</div>:<button className="primary" onClick={check} disabled={checking}><RefreshCw size={15} className={checking?'spin':''}/>{checking?'正在检查…':'检查更新'}</button>}{error&&<div className="warning"><AlertTriangle size={15}/>{error}</div>}<div className="about-links"><button onClick={()=>api.openExternal('https://github.com/Whalica/OJ_Insight')}><Github size={16}/>GitHub 仓库<ExternalLink size={13}/></button><button onClick={()=>api.openExternal('https://github.com/Whalica/OJ_Insight/issues/new')}><AlertTriangle size={16}/>报告问题<ExternalLink size={13}/></button></div><small>便携数据目录：data/ · exports/ · webview/ · logs/</small></section></>;
}
