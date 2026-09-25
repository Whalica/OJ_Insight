import { useEffect, useState, type CSSProperties } from 'react';
import { AlertTriangle, Check, ChevronDown, ChevronLeft, ChevronRight, ExternalLink, KeyRound, RefreshCw, Sparkles } from 'lucide-react';
import Heatmap from '../components/Heatmap';
import DifficultyHeatmap from '../components/DifficultyHeatmap';
import StatCards from '../components/StatCards';
import RatingOverview from '../components/RatingOverview';
import KnowledgeRadar from '../components/KnowledgeRadar';
import { api } from '../services/api';
import { currentYear, dayAtEpoch, formatDateTime, formatTime, hourInTimeZone, timeZoneLabel, today } from '../lib/date';
import { difficultyColor, METRICS, PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { TimeScope } from '../lib/ui';
import type { AccountConfig, Metric, Platform, Snapshot, SolvedGain } from '../types';

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
  solvedGains: SolvedGain[];
  loading: boolean;
  syncing: string | null;
  syncTip: string;
  syncProgress: { done: number; total: number; added: number; partial: number; failed: number } | null;
  onSync: () => void;
  onDay: (day: string) => void;
  onDifficulty: (platform: Platform, label: string, sourceOverride?: string) => void;
  onPlatform: (platform: Platform) => void;
  onOpenSettings: () => void;
}

function greeting(timeZone: string) {
  const hour = hourInTimeZone(timeZone);
  if (hour < 5) return { title: '凌晨好', message: '夜深了，保持清醒，也记得给大脑留一点休息。' };
  if (hour < 11) return { title: '早上好', message: '先从一道手感题开始，把今天的训练节奏带起来。' };
  if (hour < 14) return { title: '中午好', message: '午间适合补一道短题，或回看上午卡住的关键一步。' };
  if (hour < 18) return { title: '下午好', message: '今天的轨迹已经在这里了，看看还想补上哪一块。' };
  return { title: '晚上好', message: '复盘今天的提交，比单纯追求题数更接近真正的进步。' };
}

const CHECKIN_STORAGE_KEY = 'oj-insight.checkins.v1';
const CHECKIN_MESSAGES = [
  '今天也来看看自己啦。',
  '为今天留下一枚小小记录。',
  '看到这里，愿你今天也有好心情。',
  '数据会积累，成长不必着急。',
  '欢迎回来，慢慢来就很好。',
  '今天的 OJ 小角落也亮了一下。',
];

function loadCheckinDays() {
  try {
    const value = JSON.parse(localStorage.getItem(CHECKIN_STORAGE_KEY) || '[]');
    if (!Array.isArray(value)) return [];
    return [...new Set(value.filter((day): day is string => typeof day === 'string' && /^\d{4}-\d{2}-\d{2}$/.test(day)))].sort();
  } catch {
    return [];
  }
}

export default function DashboardPage(props: Props) {
  const { platform, platformAccounts, accountFilter, setAccountFilter, sourceFilter, setSourceFilter, timeScope, setTimeScope, range, metric, setMetric, timeZone, snapshot, solvedGains, loading, syncing, syncTip, syncProgress, onSync, onDay, onDifficulty, onPlatform, onOpenSettings } = props;
  const welcome = greeting(timeZone); const title = platform ? PLATFORM_META[platform].name : welcome.title;
  const years = Array.from({ length: currentYear(timeZone) - 2009 }, (_, index) => currentYear(timeZone) - index);
  const luoguLimited = platform === 'luogu';
  const label = luoguLimited ? '近半年' : timeScope === 'until' ? '至今（近一年）' : String(timeScope);
  const move = (delta: number) => { if (timeScope !== 'until') setTimeScope(Math.min(currentYear(timeZone), Math.max(2010, timeScope + delta))); };
  const credentialPlatform = platform && ['codeforces', 'nowcoder', 'qoj', 'leetcode'].includes(platform) ? platform : null;
  const openCredentialHelp = () => onOpenSettings();
  return <>
    <header className="topbar dashboard-head"><div><small>{platform ? `${PLATFORM_META[platform].short} · PLATFORM` : today(timeZone)}</small><h1>{title}</h1><p>{platform ? luoguLimited ? '洛谷公开活动砖与题库难度概况。' : `${PLATFORM_META[platform].name} 的活动砖、难度足迹和逐题记录。` : welcome.message}</p></div><div className="topbar-actions">{(!platform || credentialPlatform) && <button className="credential-help" onClick={openCredentialHelp}><KeyRound size={15} />{!platform ? '配置 API / Cookie' : platform === 'codeforces' ? '配置 API' : '配置 Cookie'}</button>}<button className="primary sync-button" onClick={onSync} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />{syncProgress ? `${syncProgress.done}/${syncProgress.total}` : syncing ? '同步中' : platform ? `同步 ${PLATFORM_META[platform].short}` : '同步全部'}</button></div></header>
    {!!syncing && syncTip && <div className="tip-banner"><span>比赛小贴士</span><strong>{syncTip}</strong></div>}
    {syncProgress && <div className="sync-banner"><strong>正在同步 {syncProgress.done} / {syncProgress.total}</strong><span>新增 {syncProgress.added} 条 · 部分可用 {syncProgress.partial} · 失败 {syncProgress.failed}</span><i><b style={{ width: `${syncProgress.total ? syncProgress.done / syncProgress.total * 100 : 0}%` }} /></i></div>}
    {!platform && <TodayProgress snapshot={snapshot} timeZone={timeZone} solvedGains={solvedGains} onSelect={onPlatform} onSync={onSync} syncing={!!syncing} />}
    <div className="section-title career-title"><small>CAREER · 不受下方时间范围影响</small><h2>生涯累计</h2></div><StatCards stats={snapshot.career} />
    {platform !== 'luogu' && <RatingOverview ratings={snapshot.ratings} timeZone={timeZone} selectedPlatform={platform} />}
    {(!platform || platform === 'codeforces' || platform === 'leetcode' || platform === 'qoj') && <KnowledgeRadar data={snapshot.knowledge || []} selectedPlatform={platform} />}
    <div className="toolbar">
      <label>时间范围{luoguLimited ? <div className="range-fixed">近半年</div> : <div className="year-control"><button onClick={() => move(-1)} disabled={timeScope === 'until' || timeScope <= 2010}><ChevronLeft size={15} /></button><div className="select-wrap"><select value={timeScope} onChange={(event) => setTimeScope(event.target.value === 'until' ? 'until' : Number(event.target.value))}><option value="until">至今（近一年）</option>{years.map((year) => <option value={year} key={year}>{year}</option>)}</select><ChevronDown size={14} /></div><button onClick={() => move(1)} disabled={timeScope === 'until' || timeScope >= currentYear(timeZone)}><ChevronRight size={15} /></button></div>}</label>
      <label>统计口径{luoguLimited ? <div className="range-fixed">活动次数</div> : <div className="select-wrap"><select value={metric} onChange={(event) => setMetric(event.target.value as Metric)}>{METRICS.map((item) => <option value={item.value} key={item.value}>{item.label}</option>)}</select><ChevronDown size={14} /></div>}</label>
      {platform && platformAccounts.length > 1 && <label>账号<div className="select-wrap"><select value={accountFilter} onChange={(event) => setAccountFilter(event.target.value)}><option value="">全部账号</option>{platformAccounts.map((entry) => <option key={entry.account} value={entry.account}>{entry.account}</option>)}</select><ChevronDown size={14} /></div></label>}
      {platform === 'nowcoder' && <label>记录来源<div className="source-segments"><button className={!sourceFilter ? 'active' : ''} onClick={() => setSourceFilter('')}>总计</button><button className={sourceFilter === 'oj' ? 'active' : ''} onClick={() => setSourceFilter('oj')}>普通 OJ</button><button className={sourceFilter === 'daily' ? 'active' : ''} onClick={() => setSourceFilter('daily')}>每日一题</button></div></label>}
      <span className="toolbar-note">按 {timeZoneLabel(timeZone)} 统计</span>
    </div>
    {luoguLimited && <div className="warning"><AlertTriangle size={16} />洛谷仅使用公开个人页 dailyCounts，固定展示近半年活动；不抓取提交记录，因此没有当天逐题明细。</div>}
    {!snapshot.metric_available && <div className="warning"><AlertTriangle size={16} />当前平台没有这一口径的逐日数据。</div>}
    {snapshot.warnings.map((warning) => <div className="warning" key={warning}><AlertTriangle size={16} />{warning}</div>)}
    <div className="section-title"><small>CURRENT RANGE</small><h2>{label}的训练状态</h2></div><StatCards stats={snapshot.stats} />
    <section className={`panel heat-panel ${platform === 'luogu' ? 'luogu-heat-panel' : ''}`}><div className="panel-head"><div><small>ACTIVITY · {range.start} — {range.end}</small><h2>{platform ? `${PLATFORM_META[platform].name} 活动砖` : '全部 OJ 活动砖'}</h2></div>{loading && <span className="muted">读取中…</span>}</div><Heatmap startDay={range.start} endDay={range.end} daily={snapshot.daily} onDay={onDay} /></section>
    {platform && platform !== 'luogu' && <section className="panel heat-panel difficulty-footprint"><div className="panel-head"><div><small>DAILY PEAK DIFFICULTY</small><h2>难度足迹</h2><p>用当天 AC 题目的最高难度，为这一天留下颜色。</p></div></div><DifficultyHeatmap platform={platform} startDay={range.start} endDay={range.end} daily={snapshot.difficulty_daily} onDay={onDay} colorFor={difficultyColor} />{!snapshot.difficulty_daily.length && <div className="inline-empty">当前数据源没有逐题难度，活动砖仍可正常使用。</div>}</section>}
    {!platform && <section className="panel overview-platforms"><div className="panel-head"><div><small>PLATFORMS</small><h2>各 OJ 训练概况</h2></div></div><PlatformTable rows={snapshot.platforms} onSelect={onPlatform} /></section>}
    {platform === 'leetcode' && <LeetCodeSummary data={snapshot.difficulty} />}
    <section className="panel difficulty-panel"><div className="panel-head"><div><small>DIFFICULTY DISTRIBUTION</small><h2>{platform === 'nowcoder' ? sourceFilter === 'daily' ? '每日一题难度' : 'Tracker 题目 Rating' : '难度分布'}</h2></div><span className="muted">点击柱形查看该难度的全部题目</span></div><DifficultyProfile data={snapshot.difficulty} preferred={platform || undefined} onDifficulty={(item, label) => onDifficulty(item, label, item === 'nowcoder' ? sourceFilter || 'oj' : undefined)} /></section>
    {platform === 'nowcoder' && !sourceFilter && <section className="panel difficulty-panel nowcoder-daily-difficulty"><div className="panel-head"><div><small>DAILY PROBLEM DIFFICULTY · 独立 1–5 级</small><h2>每日一题难度</h2><p>每日一题使用独立难度等级，不与 Tracker 题目 Rating 混合。</p></div><span className="muted">点击柱形查看对应每日题</span></div><DifficultyProfile data={snapshot.nowcoder_daily_difficulty} preferred="nowcoder" hideTabs onDifficulty={(item, label) => onDifficulty(item, label, 'daily')} /></section>}
    {platform !== 'luogu' && <section className="panel recent-panel"><div className="panel-head"><div><small>RECENT ACCEPTED · 不受时间范围影响</small><h2>最近 AC</h2></div></div><RecentList items={snapshot.recent} timeZone={timeZone} /></section>}
  </>;
}

function TodayProgress({ snapshot, timeZone, solvedGains, onSelect, onSync, syncing }: { snapshot: Snapshot; timeZone: string; solvedGains: SolvedGain[]; onSelect: (platform: Platform) => void; onSync: () => void; syncing: boolean }) {
  const rows = snapshot.platforms;
  const by = new Map(rows.map((row) => [row.platform, row]));
  const currentDay = today(timeZone);
  const todayProblemMap = new Map<string, Snapshot['recent'][number]>();
  for (const item of snapshot.today_problems || snapshot.recent) {
    if (item.source_day !== currentDay && dayAtEpoch(item.epoch_second, timeZone) !== currentDay) continue;
    const key = `${item.platform}:${item.problem_key}`;
    if (!todayProblemMap.has(key)) todayProblemMap.set(key, item);
  }
  const todayProblems = [...todayProblemMap.values()];
  const uniqueByPlatform = new Map<Platform, number>();
  for (const item of todayProblems) uniqueByPlatform.set(item.platform, (uniqueByPlatform.get(item.platform) || 0) + 1);
  const displayTodayCount = (platform: Platform) => {
    const row = by.get(platform);
    return row?.today_count ?? uniqueByPlatform.get(platform) ?? 0;
  };
  const total = PLATFORM_ORDER.reduce((sum, platform) => sum + displayTodayCount(platform), 0);
  const solvedTotal = rows.reduce((sum, row) => sum + (row.solved || 0), 0);
  const milestone = solvedTotal < 10 ? 10 : Math.ceil((solvedTotal + 1) / 25) * 25;
  const encouragement = total > 0
    ? `今天新增的 ${total} 项进度已经留下来了。逐题平台的重复 AC 只会计作同一道题。`
    : solvedTotal > 0 ? `各平台已经累计 ${solvedTotal.toLocaleString()} 题，距离下一个小里程碑还有 ${milestone - solvedTotal} 题。今天休息也不会抹掉这些积累。` : '第一题不需要很难，也不必很快。只要开始，它就会成为以后回头能看见的一小步。';
  const todayRatings = snapshot.ratings.flatMap((summary) => summary.history
    .filter((point) => dayAtEpoch(point.epoch_second, timeZone) === currentDay)
    .map((point) => ({ ...point, platform: summary.platform, account: summary.display_name || summary.account })));
  const contestActivity = new Map<string, { platform: Platform; contestId: string; count: number; url: string }>();
  for (const item of todayProblems) {
    const match = item.problem_url.match(/codeforces\.com\/(?:contest|gym)\/(\d+)/i) || item.problem_url.match(/atcoder\.jp\/contests\/([^/]+)/i);
    if (!match) continue;
    const key = `${item.platform}:${match[1]}`;
    const current = contestActivity.get(key);
    contestActivity.set(key, { platform: item.platform, contestId: match[1], count: (current?.count || 0) + 1, url: item.problem_url });
  }
  const finishedContests = [...contestActivity.values()].flatMap((contest) => {
    const rating = todayRatings.find((item) => item.platform === contest.platform && item.contest_id === contest.contestId);
    return rating ? [{ ...contest, rating }] : [];
  });
  const [checkinDays, setCheckinDays] = useState<string[]>(loadCheckinDays);
  const [error, setError] = useState('');
  const [justChecked, setJustChecked] = useState(false);
  const checkedToday = checkinDays.includes(currentDay);
  const checkinIndex = checkinDays.indexOf(currentDay);
  const feedback = checkedToday ? CHECKIN_MESSAGES[Math.max(0, checkinIndex) % CHECKIN_MESSAGES.length] : '';
  useEffect(() => { setError(''); setJustChecked(false); }, [currentDay]);

  const checkIn = () => {
    if (checkedToday) return;
    const next = [...new Set([...checkinDays, currentDay])].sort();
    try {
      localStorage.setItem(CHECKIN_STORAGE_KEY, JSON.stringify(next));
      setCheckinDays(next);
      setJustChecked(true);
      window.setTimeout(() => setJustChecked(false), 900);
    } catch {
      setError('这次打卡没有保存成功，请稍后再试。');
    }
  };

  return <section className="today-block">
    <div className="today-copy"><small>TODAY · {currentDay}</small><h2>今日进度</h2><p>{total ? `今天六个平台共留下 ${total} 条活动记录。` : '今天还没有活动记录，第一块砖会从哪里亮起？'}</p>
      <div className={`today-checkin ${justChecked ? 'celebrate' : ''}`}>
        <div className="today-checkin-actions"><button className={checkedToday ? 'checked' : ''} onClick={checkIn} disabled={checkedToday} aria-pressed={checkedToday}><Check size={16} />{checkedToday ? '今天已打卡' : '今日打卡'}</button></div>
        <span className="today-checkin-count">累计打卡 <strong>{checkinDays.length}</strong> 天</span>{feedback && <span className="today-checkin-feedback" role="status"><Sparkles size={14} />{feedback}</span>}{error && <span className="today-checkin-error" role="alert">{error}</span>}
      </div>
      <div className="today-encouragement"><Sparkles size={15} /><span>{encouragement}</span></div>
    </div>
    <div className="today-oj-grid">{PLATFORM_ORDER.map((platform) => { const gain = solvedGains.find((item) => item.platform === platform); return <button key={platform} onClick={() => onSelect(platform)}><span className="platform-monogram" style={{ color: PLATFORM_META[platform].accent }}>{PLATFORM_META[platform].short}</span><strong className="today-progress-count">{displayTodayCount(platform)}{gain && <GainBubble gain={gain} />}</strong><small>{PLATFORM_META[platform].name}</small></button>; })}</div>
    <div className="today-detail">
      <header><div><small>TODAY'S ACTIVITY</small><strong>今日题目与比赛</strong></div><button onClick={onSync} disabled={syncing}><RefreshCw size={14} className={syncing ? 'spin' : ''} />{syncing ? '刷新中' : '比赛结束后刷新'}</button></header>
      <div className="today-detail-grid">
        <div className="today-problems"><span>题目 / AC</span>{todayProblems.length ? todayProblems.slice(0, 6).map((item) => <button key={`${item.platform}-${item.account}-${item.submission_id}`} onClick={() => item.problem_url && void api.openExternal(item.problem_url)}><i style={{ background: PLATFORM_META[item.platform].accent }} /><div><strong>{item.problem_id || item.problem_name}</strong><small>{item.problem_name} · {item.language || '语言未知'}</small></div><span className="today-problem-meta">{item.difficulty && <b><i style={{ background: difficultyColor(item.platform, item.difficulty) }} />{item.difficulty}</b>}<time>{item.source_day ? '每日一题' : formatTime(item.epoch_second, timeZone)}</time></span></button>) : <em>同步后会在这里显示今天完成的题目。</em>}</div>
        <div className="today-contests"><span>今日已结束比赛</span>{finishedContests.map((contest) => <button key={`${contest.platform}-${contest.contestId}`} onClick={() => void api.openExternal(contest.url)}><span className="platform-monogram" style={{ color: PLATFORM_META[contest.platform].accent }}>{PLATFORM_META[contest.platform].short}</span><div><strong>{contest.rating.contest_name}</strong><small>{contest.count} 道 AC{contest.rating.rank == null ? '' : ` · 排名 ${contest.rating.rank}`}</small></div><b className={contest.rating.new_rating < contest.rating.old_rating ? 'negative' : 'positive'}>{contest.rating.old_rating} → {contest.rating.new_rating}</b></button>)}{!finishedContests.length && <em>今天没有已结束且确认参赛的比赛；赛后同步即可更新过题与 Rating。</em>}</div>
      </div>
    </div>
  </section>;
}

function GainBubble({ gain }: { gain: SolvedGain }) {
  return <span key={gain.id} className="solved-gain" style={{ '--gain-x': `${gain.offsetX}px`, '--gain-y': `${gain.offsetY}px` } as CSSProperties} aria-hidden="true">+{gain.amount}</span>;
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

function DifficultyProfile({ data, preferred, onDifficulty, hideTabs = false }: { data: Snapshot['difficulty']; preferred?: Platform; onDifficulty: (platform: Platform, label: string) => void; hideTabs?: boolean }) {
  const available = PLATFORM_ORDER.filter((platform) => data.some((item) => item.platform === platform && item.count > 0));
  const [selected, setSelected] = useState<Platform | undefined>(preferred || available[0]);
  useEffect(() => { if (preferred && available.includes(preferred)) setSelected(preferred); else if (!selected || !available.includes(selected)) setSelected(available[0]); }, [preferred, available.join('|')]);
  const active = selected && available.includes(selected) ? selected : available[0];
  if (!active) return <div className="empty">生涯记录中暂时没有可靠的难度数据；同步源可用后会自动补齐。</div>;
  const shown = filledDifficulty(data, active); const max = Math.max(1, ...shown.map((item) => item.count)); const total = shown.reduce((sum, item) => sum + item.count, 0);
  const gap = shown.length >= 14 ? 5 : shown.length >= 8 ? 9 : 12;
  const barFill = shown.length >= 14 ? 78 : shown.length >= 8 ? 72 : shown.length >= 5 ? 68 : 58;
  const chartStyle = {
    '--bucket-count': shown.length,
    '--histogram-gap': `${gap}px`,
    '--histogram-bar-fill': `${barFill}%`,
  } as CSSProperties;
  return <>{!hideTabs && <div className="difficulty-tabs">{available.map((platform) => <button className={platform === active ? 'active' : ''} onClick={() => setSelected(platform)} key={platform}>{PLATFORM_META[platform].short}<span>{PLATFORM_META[platform].name}</span></button>)}</div>}<div className={`histogram histogram-${active}${hideTabs ? ' histogram-daily' : ''}`} style={chartStyle}>{shown.map((item) => <button className={`histogram-bucket ${item.label === '未评级' ? 'unrated' : ''}`} key={`${item.platform}-${item.label}`} title={`${item.label}：${item.count}，点击查看题目`} aria-label={`${PLATFORM_META[active].name} ${item.label} 难度，${item.count} 道题，点击查看`} onClick={() => onDifficulty(active, item.label)}><span className="histogram-bar" style={{ '--bar-height': `${Math.max(7, item.count / max * 100)}%`, '--bar-color': difficultyColor(active, item.label, item.order) } as CSSProperties}><strong>{item.count}</strong><i /></span><span>{item.label}</span></button>)}</div><div className="difficulty-summary"><span>生涯去重难度题数<strong>{total.toLocaleString()} 题</strong></span><span>分级方式<strong>{active === 'codeforces' ? '每 100 rating 一级，含未评级' : active === 'luogu' ? 'Luogu 最新八级体系，含未评级' : active === 'nowcoder' && hideTabs ? '每日一题独立 1–5 级' : `${PLATFORM_META[active].name} 当前体系，含未评级`}</strong></span></div></>;
}

function LeetCodeSummary({ data }: { data: Snapshot['difficulty'] }) {
  const rows = data.filter((item) => item.platform === 'leetcode' && item.count > 0); const total = rows.reduce((sum, item) => sum + item.count, 0);
  if (!rows.length) return null;
  return <section className="leetcode-strip"><div><small>LEETCODE PROGRESS</small><h2>题库进度结构</h2><p>把 Easy、Medium、Hard 独立呈现，便于观察刷题结构。</p></div><div>{rows.map((item) => <span key={item.label} style={{ '--difficulty': difficultyColor('leetcode', item.label, item.order) } as CSSProperties}><i /><small>{item.label}</small><strong>{item.count}</strong><em>{total ? Math.round(item.count / total * 100) : 0}%</em></span>)}</div></section>;
}

function RecentList({ items, timeZone }: { items: Snapshot['recent']; timeZone: string }) {
  if (!items.length) return <div className="empty">暂时没有可读取的逐题 AC。同步源不可用时会在上方给出具体说明，不会用活动计数伪造题目。</div>;
  return <div className="recent-list">{items.map((item) => <article key={`${item.platform}-${item.account}-${item.submission_id}`}><span className="platform-monogram" style={{ color: PLATFORM_META[item.platform].accent }}>{PLATFORM_META[item.platform].short}</span><div className="recent-title"><strong>{item.problem_id || item.problem_name}</strong><span>{item.problem_name}</span><small>{item.account}{item.source === 'daily' ? ' · 每日一题' : ''}</small>{item.tags?.length > 0 && <div className="problem-tags">{item.tags.slice(0, 3).map((tag) => <em key={tag}>{tag}</em>)}</div>}</div><div className="recent-meta">{item.difficulty && <span><i style={{ background: difficultyColor(item.platform, item.difficulty) }} />{item.difficulty}</span>}<small>{item.source_day ? `来源日期 ${item.source_day}` : formatDateTime(item.epoch_second, timeZone)}</small></div>{item.problem_url ? <a className="problem-button" href={item.problem_url} target="_blank" rel="noreferrer">前往题目<ExternalLink size={14} /></a> : <span />}</article>)}</div>;
}
