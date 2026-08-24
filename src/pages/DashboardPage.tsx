import { useEffect, useState, type CSSProperties } from 'react';
import { AlertTriangle, ChevronDown, ChevronLeft, ChevronRight, ExternalLink, RefreshCw } from 'lucide-react';
import Heatmap from '../components/Heatmap';
import DifficultyHeatmap from '../components/DifficultyHeatmap';
import StatCards from '../components/StatCards';
import { currentYear, formatDateTime, hourInTimeZone, timeZoneLabel, today } from '../lib/date';
import { difficultyColor, METRICS, PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { TimeScope } from '../lib/ui';
import type { AccountConfig, Metric, Platform, Snapshot } from '../types';

interface Props {
  platform: Platform | null;
  platformAccounts: AccountConfig[];
  accountFilter: string;
  setAccountFilter: (value: string) => void;
  sourceFilter: string;
  setSourceFilter: (value: string) => void;
  timeScope: TimeScope;
  setTimeScope: (value: TimeScope) => void;
  range: { start: string; end: string };
  metric: Metric;
  setMetric: (value: Metric) => void;
  timeZone: string;
  snapshot: Snapshot;
  loading: boolean;
  syncing: string | null;
  syncTip: string;
  syncProgress: { done: number; total: number; added: number; failed: number } | null;
  onSync: () => void;
  onDay: (day: string) => void;
  onPlatform: (platform: Platform) => void;
}

function greeting(timeZone: string) {
  const hour = hourInTimeZone(timeZone);
  if (hour < 5) return { title: '凌晨好', message: '夜深了，保持清醒，也记得给大脑留一点休息。' };
  if (hour < 11) return { title: '早上好', message: '先从一道手感题开始，把今天的训练节奏带起来。' };
  if (hour < 14) return { title: '中午好', message: '午间适合补一道短题，或回看上午卡住的关键一步。' };
  if (hour < 18) return { title: '下午好', message: '今天的轨迹已经在这里了，看看还想补上哪一块。' };
  return { title: '晚上好', message: '复盘今天的提交，比单纯追求题数更接近真正的进步。' };
}

export default function DashboardPage(props: Props) {
  const { platform, platformAccounts, accountFilter, setAccountFilter, sourceFilter, setSourceFilter, timeScope, setTimeScope, range, metric, setMetric, timeZone, snapshot, loading, syncing, syncTip, syncProgress, onSync, onDay, onPlatform } = props;
  const welcome = greeting(timeZone); const title = platform ? PLATFORM_META[platform].name : welcome.title;
  const years = Array.from({ length: currentYear(timeZone) - 2009 }, (_, index) => currentYear(timeZone) - index);
  const label = timeScope === 'until' ? '至今（近一年）' : String(timeScope);
  const move = (delta: number) => { if (timeScope !== 'until') setTimeScope(Math.min(currentYear(timeZone), Math.max(2010, timeScope + delta))); };
  return <>
    <header className="topbar dashboard-head"><div><small>{platform ? `${PLATFORM_META[platform].short} · PLATFORM` : today(timeZone)}</small><h1>{title}</h1><p>{platform ? `${PLATFORM_META[platform].name} 的活动砖、难度足迹和逐题记录。` : welcome.message}</p></div><button className="primary sync-button" onClick={onSync} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />{syncProgress ? `${syncProgress.done}/${syncProgress.total}` : syncing ? '同步中' : platform ? `同步 ${PLATFORM_META[platform].short}` : '同步全部'}</button></header>
    {!!syncing && syncTip && <div className="tip-banner"><span>比赛小贴士</span><strong>{syncTip}</strong></div>}
    {syncProgress && <div className="sync-banner"><strong>正在同步 {syncProgress.done} / {syncProgress.total}</strong><span>新增 {syncProgress.added} 条 · {syncProgress.failed} 个平台失败</span><i><b style={{ width: `${syncProgress.total ? syncProgress.done / syncProgress.total * 100 : 0}%` }} /></i></div>}
    {!platform && <TodayProgress rows={snapshot.platforms} timeZone={timeZone} onSelect={onPlatform} />}
    <div className="section-title career-title"><small>CAREER · 不受下方时间范围影响</small><h2>生涯累计</h2></div><StatCards stats={snapshot.career} />
    <div className="toolbar">
      <label>时间范围<div className="year-control"><button onClick={() => move(-1)} disabled={timeScope === 'until' || timeScope <= 2010}><ChevronLeft size={15} /></button><div className="select-wrap"><select value={timeScope} onChange={(event) => setTimeScope(event.target.value === 'until' ? 'until' : Number(event.target.value))}><option value="until">至今（近一年）</option>{years.map((year) => <option value={year} key={year}>{year}</option>)}</select><ChevronDown size={14} /></div><button onClick={() => move(1)} disabled={timeScope === 'until' || timeScope >= currentYear(timeZone)}><ChevronRight size={15} /></button></div></label>
      <label>统计口径<div className="select-wrap"><select value={metric} onChange={(event) => setMetric(event.target.value as Metric)}>{METRICS.map((item) => <option value={item.value} key={item.value}>{item.label}</option>)}</select><ChevronDown size={14} /></div></label>
      {platform && platformAccounts.length > 1 && <label>账号<div className="select-wrap"><select value={accountFilter} onChange={(event) => setAccountFilter(event.target.value)}><option value="">全部账号</option>{platformAccounts.map((entry) => <option key={entry.account} value={entry.account}>{entry.account}</option>)}</select><ChevronDown size={14} /></div></label>}
      {platform === 'nowcoder' && <label>记录来源<div className="source-segments"><button className={!sourceFilter ? 'active' : ''} onClick={() => setSourceFilter('')}>总计</button><button className={sourceFilter === 'oj' ? 'active' : ''} onClick={() => setSourceFilter('oj')}>普通 OJ</button><button className={sourceFilter === 'daily' ? 'active' : ''} onClick={() => setSourceFilter('daily')}>每日一题</button></div></label>}
      <span className="toolbar-note">按 {timeZoneLabel(timeZone)} 统计</span>
    </div>
    {!snapshot.metric_available && <div className="warning"><AlertTriangle size={16} />当前平台没有这一口径的逐日数据。</div>}
    {snapshot.warnings.map((warning) => <div className="warning" key={warning}><AlertTriangle size={16} />{warning}</div>)}
    <div className="section-title"><small>CURRENT RANGE</small><h2>{label}的训练状态</h2></div><StatCards stats={snapshot.stats} />
    <section className="panel heat-panel"><div className="panel-head"><div><small>ACTIVITY · {range.start} — {range.end}</small><h2>{platform ? `${PLATFORM_META[platform].name} 活动砖` : '全部 OJ 活动砖'}</h2></div>{loading && <span className="muted">读取中…</span>}</div><Heatmap startDay={range.start} endDay={range.end} daily={snapshot.daily} onDay={onDay} /></section>
    {platform && <section className="panel heat-panel difficulty-footprint"><div className="panel-head"><div><small>DAILY PEAK DIFFICULTY</small><h2>难度足迹</h2><p>用当天 AC 题目的最高难度，为这一天留下颜色。</p></div></div><DifficultyHeatmap platform={platform} startDay={range.start} endDay={range.end} daily={snapshot.difficulty_daily} onDay={onDay} colorFor={difficultyColor} />{!snapshot.difficulty_daily.length && <div className="inline-empty">当前数据源没有逐题难度，活动砖仍可正常使用。</div>}</section>}
    {!platform && <section className="panel overview-platforms"><div className="panel-head"><div><small>PLATFORMS</small><h2>各 OJ 训练概况</h2></div></div><PlatformTable rows={snapshot.platforms} onSelect={onPlatform} /></section>}
    {platform === 'leetcode' && <LeetCodeSummary data={snapshot.difficulty} />}
    <section className="panel difficulty-panel"><div className="panel-head"><div><small>DIFFICULTY DISTRIBUTION</small><h2>难度分布</h2></div></div><DifficultyProfile data={snapshot.difficulty} preferred={platform || undefined} /></section>
    <section className="panel recent-panel"><div className="panel-head"><div><small>RECENT ACCEPTED · 不受时间范围影响</small><h2>最近 AC</h2></div></div><RecentList items={snapshot.recent} timeZone={timeZone} /></section>
  </>;
}

function TodayProgress({ rows, timeZone, onSelect }: { rows: Snapshot['platforms']; timeZone: string; onSelect: (platform: Platform) => void }) {
  const by = new Map(rows.map((row) => [row.platform, row])); const total = rows.reduce((sum, row) => sum + row.today_count, 0);
  return <section className="today-block"><div className="today-copy"><small>TODAY · {today(timeZone)}</small><h2>今日进度</h2><p>{total ? `今天六个平台共留下 ${total} 条活动记录。` : '今天还没有活动记录，第一块砖会从哪里亮起？'}</p></div><div className="today-oj-grid">{PLATFORM_ORDER.map((platform) => { const row = by.get(platform); return <button key={platform} onClick={() => onSelect(platform)}><span className="platform-monogram" style={{ color: PLATFORM_META[platform].accent }}>{PLATFORM_META[platform].short}</span><strong>{row?.today_count || 0}</strong><small>{PLATFORM_META[platform].name}</small></button>; })}</div></section>;
}

function PlatformTable({ rows, onSelect }: { rows: Snapshot['platforms']; onSelect: (platform: Platform) => void }) {
  if (!rows.length) return <div className="empty">还没有本地数据。先到设置页填写账号，然后同步。</div>;
  return <div className="platform-table">{rows.map((row) => <button key={row.platform} onClick={() => onSelect(row.platform)}><span className="platform-monogram" style={{ color: PLATFORM_META[row.platform].accent }}>{PLATFORM_META[row.platform].short}</span><div><strong>{PLATFORM_META[row.platform].name}</strong><small>{row.account || '未配置账号'}</small></div><span>{row.solved == null ? '暂无解题数' : `${row.solved.toLocaleString()} 题`}</span><small>{row.status === 'ok' ? '同步成功' : '查看状态'} · 缓存 {row.cached_records}</small><ChevronRight size={15} /></button>)}</div>;
}

function filledDifficulty(data: Snapshot['difficulty'], platform: Platform) {
  return data
    .filter((item) => item.platform === platform && item.count > 0)
    .sort((left, right) => left.order - right.order || left.label.localeCompare(right.label));
}

function DifficultyProfile({ data, preferred }: { data: Snapshot['difficulty']; preferred?: Platform }) {
  const available = PLATFORM_ORDER.filter((platform) => data.some((item) => item.platform === platform && item.count > 0));
  const [selected, setSelected] = useState<Platform | undefined>(preferred || available[0]);
  useEffect(() => { if (preferred && available.includes(preferred)) setSelected(preferred); else if (!selected || !available.includes(selected)) setSelected(available[0]); }, [preferred, available.join('|')]);
  const active = selected && available.includes(selected) ? selected : available[0];
  if (!active) return <div className="empty">生涯记录中暂时没有可靠的难度数据；同步源可用后会自动补齐。</div>;
  const shown = filledDifficulty(data, active); const max = Math.max(1, ...shown.map((item) => item.count)); const total = shown.reduce((sum, item) => sum + item.count, 0);
  return <><div className="difficulty-tabs">{available.map((platform) => <button className={platform === active ? 'active' : ''} onClick={() => setSelected(platform)} key={platform}>{PLATFORM_META[platform].short}<span>{PLATFORM_META[platform].name}</span></button>)}</div><div className={`histogram histogram-${active}`} style={{ '--bucket-count': shown.length } as CSSProperties}>{shown.map((item) => <div key={`${item.platform}-${item.label}`} title={`${item.label}：${item.count}`}><strong>{item.count}</strong><i style={{ height: `${Math.max(8, item.count / max * 100)}%`, background: difficultyColor(active, item.label, item.order) }} /><span>{item.label}</span></div>)}</div><div className="difficulty-summary"><span>生涯去重难度题数<strong>{total.toLocaleString()} 题</strong></span><span>分级方式<strong>{active === 'codeforces' ? '每 100 rating 一级，仅显示已完成难度' : active === 'luogu' ? 'Luogu 最新八级体系' : `${PLATFORM_META[active].name} 当前体系`}</strong></span></div></>;
}

function LeetCodeSummary({ data }: { data: Snapshot['difficulty'] }) {
  const rows = data.filter((item) => item.platform === 'leetcode' && item.count > 0); const total = rows.reduce((sum, item) => sum + item.count, 0);
  if (!rows.length) return null;
  return <section className="leetcode-strip"><div><small>LEETCODE PROGRESS</small><h2>题库进度结构</h2><p>把 Easy、Medium、Hard 独立呈现，便于观察刷题结构。</p></div><div>{rows.map((item) => <span key={item.label} style={{ '--difficulty': difficultyColor('leetcode', item.label, item.order) } as CSSProperties}><i /><small>{item.label}</small><strong>{item.count}</strong><em>{total ? Math.round(item.count / total * 100) : 0}%</em></span>)}</div></section>;
}

function RecentList({ items, timeZone }: { items: Snapshot['recent']; timeZone: string }) {
  if (!items.length) return <div className="empty">暂时没有可读取的逐题 AC。同步源不可用时会在上方给出具体说明，不会用活动计数伪造题目。</div>;
  return <div className="recent-list">{items.map((item) => <article key={`${item.platform}-${item.submission_id}`}><span className="platform-monogram" style={{ color: PLATFORM_META[item.platform].accent }}>{PLATFORM_META[item.platform].short}</span><div className="recent-title"><strong>{item.problem_id || item.problem_name}</strong><span>{item.problem_name}</span><small>{item.account}{item.source === 'daily' ? ' · 每日一题' : ''}</small></div><div className="recent-meta">{item.difficulty && <span><i style={{ background: difficultyColor(item.platform, item.difficulty) }} />{item.difficulty}</span>}<small>{item.source_day ? `来源日期 ${item.source_day}` : formatDateTime(item.epoch_second, timeZone)}</small></div>{item.problem_url ? <a className="problem-button" href={item.problem_url} target="_blank" rel="noreferrer">前往题目<ExternalLink size={14} /></a> : <span />}</article>)}</div>;
}
