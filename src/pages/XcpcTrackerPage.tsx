import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react';
import { ChevronDown, ChevronLeft, ChevronRight, Filter, RefreshCw, Search, X } from 'lucide-react';
import { api } from '../lib/api';
import { type XcpcContest, type XcpcTier } from '../lib/xcpc';

type Series = 'all' | 'ICPC' | 'CCPC' | '省赛' | '其他';
type Progress = 'all' | 'todo' | 'doing' | 'done';

const tierLabels: Record<XcpcTier, string> = { gold: '金题', silver: '银题', bronze: '铜题', iron: '铁题' };
function contestProgress(contest: XcpcContest): Progress {
  const solved = contest.problems.filter((problem) => problem.solved).length;
  return solved === 0 ? 'todo' : contest.problems.length > 0 && solved === contest.problems.length ? 'done' : 'doing';
}

export default function XcpcTrackerPage({ syncing, onSync, notify }: { syncing: boolean; onSync: () => Promise<void>; notify: (message: string) => void }) {
  const [contests, setContests] = useState<XcpcContest[]>([]);
  const [catalogLoading, setCatalogLoading] = useState(true);
  const [catalogError, setCatalogError] = useState('');
  const [query, setQuery] = useState('');
  const [series, setSeries] = useState<Series>('all');
  const [stage, setStage] = useState('all');
  const [year, setYear] = useState('all');
  const [site, setSite] = useState('all');
  const [progress, setProgress] = useState<Progress>('all');
  const [showDifficulty, setShowDifficulty] = useState(() => localStorage.getItem('oj-insight.xcpc.show-difficulty') !== 'false');
  const [showProblemNames, setShowProblemNames] = useState(() => localStorage.getItem('oj-insight.xcpc.show-problem-names') === 'true');
  const [shortContestNames, setShortContestNames] = useState(() => localStorage.getItem('oj-insight.xcpc.short-contest-names') === 'true');
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [page, setPage] = useState(1);
  const filterRef = useRef<HTMLDivElement>(null);
  const pageSize = 15;

  const loadCatalog = async (forceRefresh = false) => {
    setCatalogLoading(true); setCatalogError('');
    try { setContests(await api.getXcpcContests(forceRefresh)); }
    catch (error) { setCatalogError(String(error)); }
    finally { setCatalogLoading(false); }
  };

  useEffect(() => { void loadCatalog(); }, []);

  useEffect(() => {
    const close = (event: MouseEvent) => { if (!filterRef.current?.contains(event.target as Node)) setFiltersOpen(false); };
    const escape = (event: KeyboardEvent) => { if (event.key === 'Escape') setFiltersOpen(false); };
    document.addEventListener('mousedown', close); document.addEventListener('keydown', escape);
    return () => { document.removeEventListener('mousedown', close); document.removeEventListener('keydown', escape); };
  }, []);

  const filtered = useMemo(() => {
    const keyword = query.trim().toLowerCase();
    return contests.filter((contest) => {
      const haystack = `${contest.name} ${contest.shortName} ${contest.site} ${contest.year}`.toLowerCase();
      return (!keyword || haystack.includes(keyword)) &&
        (series === 'all' || contest.series.includes(series)) &&
        (stage === 'all' || contest.stage === stage) &&
        (year === 'all' || contest.year === year) &&
        (site === 'all' || contest.site === site) &&
        (progress === 'all' || contestProgress(contest) === progress);
    });
  }, [contests, progress, query, series, site, stage, year]);

  const years = useMemo(() => [...new Set(contests.map((contest) => contest.year))].sort((a, b) => b.localeCompare(a)), [contests]);
  const sites = useMemo(() => [...new Set(contests.map((contest) => contest.site))].sort((a, b) => a.localeCompare(b, 'zh-CN')), [contests]);
  const stages = useMemo(() => [...new Set(contests.map((contest) => contest.stage))], [contests]);

  const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
  const visible = filtered.slice((Math.min(page, pageCount) - 1) * pageSize, Math.min(page, pageCount) * pageSize);
  const widestVisibleContest = Math.max(1, ...visible.map((contest) => contest.problems.length));
  const solvedContests = filtered.filter((contest) => contestProgress(contest) === 'done').length;
  const solvedProblems = filtered.reduce((sum, contest) => sum + contest.problems.filter((problem) => problem.solved).length, 0);
  const totalProblems = filtered.reduce((sum, contest) => sum + contest.problems.length, 0);
  const ratedContests = filtered.filter((contest) => contest.boardSource).length;
  const activeFilterCount = [stage, year, site, progress].filter((value) => value !== 'all').length;

  const updatePreference = (key: string, value: boolean, setter: (value: boolean) => void) => {
    localStorage.setItem(key, String(value)); setter(value);
  };
  const clearFilters = () => { setStage('all'); setYear('all'); setSite('all'); setProgress('all'); setPage(1); };

  return <>
    <header className="topbar xcpc-topbar">
      <div><small>XCPC · CONTEST TRACKER</small><h1>XCPC Tracker</h1><p>浏览 ICPC、CCPC 与省赛题集，追踪 QOJ 补题进度。</p></div>
      <div className="xcpc-top-actions">
        <button className="xcpc-action" disabled={catalogLoading} onClick={() => void loadCatalog(true)}><RefreshCw className={catalogLoading ? 'spin' : ''} size={13} />{catalogLoading ? '更新中' : '更新目录'}</button>
        <button className="xcpc-action primary" disabled={syncing} onClick={async () => { await onSync(); await loadCatalog(); }}><RefreshCw className={syncing ? 'spin' : ''} size={13} />{syncing ? '同步中' : '同步 QOJ'}</button>
      </div>
    </header>

    <section className="xcpc-summary" aria-label="XCPC 统计">
      <div><span>收录比赛</span><strong>{filtered.length} 场</strong><small>当前筛选</small></div>
      <div><span>完成比赛</span><strong>{solvedContests} 场</strong><small>全部题目 AC</small></div>
      <div><span>完成题目</span><strong>{solvedProblems} / {totalProblems}</strong><small>按比赛题目统计</small></div>
      <div><span>榜单覆盖</span><strong>{ratedContests} / {filtered.length}</strong><small>有公开榜单评级</small></div>
    </section>

    <section className="xcpc-search-row" ref={filterRef}>
      <label className="xcpc-search"><Search size={15} /><input aria-label="搜索比赛" value={query} onChange={(event) => { setQuery(event.target.value); setPage(1); }} placeholder="搜索比赛名称、简称、城市或年份…" />{query && <button aria-label="清除搜索" onClick={() => setQuery('')}><X size={13} /></button>}</label>
      <button className={`xcpc-filter-trigger ${filtersOpen ? 'active' : ''}`} aria-expanded={filtersOpen} onClick={() => setFiltersOpen((value) => !value)}><Filter size={14} />筛选{activeFilterCount > 0 && <b>{activeFilterCount}</b>}</button>
      {filtersOpen && <div className="xcpc-filter-popover">
        <header><strong>筛选条件</strong><span>选择后即时更新</span></header>
        <div className="xcpc-filter-grid">
          <label>阶段<span><select value={stage} onChange={(event) => { setStage(event.target.value); setPage(1); }}><option value="all">全部阶段</option>{stages.map((value) => <option key={value}>{value}</option>)}</select><ChevronDown size={13} /></span></label>
          <label>年份<span><select value={year} onChange={(event) => { setYear(event.target.value); setPage(1); }}><option value="all">全部年份</option>{years.map((value) => <option key={value}>{value}</option>)}</select><ChevronDown size={13} /></span></label>
          <label>省份 / 赛站<span><select value={site} onChange={(event) => { setSite(event.target.value); setPage(1); }}><option value="all">全部地区</option>{sites.map((value) => <option key={value}>{value}</option>)}</select><ChevronDown size={13} /></span></label>
          <label>进度<span><select value={progress} onChange={(event) => { setProgress(event.target.value as Progress); setPage(1); }}><option value="all">全部进度</option><option value="todo">未开始</option><option value="doing">进行中</option><option value="done">已完成</option></select><ChevronDown size={13} /></span></label>
        </div>
        <footer><button onClick={clearFilters}>清除筛选</button></footer>
      </div>}
    </section>

    <section className={`panel xcpc-panel ${showProblemNames ? 'show-problem-names' : 'compact-problems'} ${showDifficulty ? '' : 'hide-difficulty'}`} style={{ '--xcpc-problems-width': `${widestVisibleContest * 128}px` } as CSSProperties}>
      <div className="xcpc-toolbar">
        <div className="source-segments">{(['all', 'ICPC', 'CCPC', '省赛', '其他'] as Series[]).map((value) => <button key={value} className={series === value ? 'active' : ''} onClick={() => { setSeries(value); setPage(1); }}>{value === 'all' ? '全部' : value}</button>)}</div>
        <div className="xcpc-view-options">
          <span>共 <strong>{filtered.length}</strong> 场 · 时间倒序</span>
          <label><input type="checkbox" checked={showProblemNames} onChange={(event) => updatePreference('oj-insight.xcpc.show-problem-names', event.target.checked, setShowProblemNames)} /><i />题目名称</label>
          <label><input type="checkbox" checked={showDifficulty} onChange={(event) => updatePreference('oj-insight.xcpc.show-difficulty', event.target.checked, setShowDifficulty)} /><i />难度颜色</label>
          <div className="source-segments xcpc-name-mode"><button className={!shortContestNames ? 'active' : ''} onClick={() => updatePreference('oj-insight.xcpc.short-contest-names', false, setShortContestNames)}>全称</button><button className={shortContestNames ? 'active' : ''} onClick={() => updatePreference('oj-insight.xcpc.short-contest-names', true, setShortContestNames)}>简称</button></div>
        </div>
      </div>

      <div className="xcpc-table-scroll">
        {catalogLoading && !contests.length && <div className="empty">正在从 QOJ 载入赛事目录…</div>}
        {catalogError && <div className="empty">目录载入失败：{catalogError} <button onClick={() => void loadCatalog(true)}>重试</button></div>}
        {!catalogLoading && contests.length > 0 && !contests.some((contest) => contest.problems.length > 0) && <div className="empty">QOJ 当前只返回了比赛索引，没有返回题目链接。请在设置中填写 QOJ 的 UOJSESSID Cookie 后重新更新目录。</div>}
        <table className="xcpc-table">
          <thead><tr><th className="xcpc-contest-column">比赛</th><th className="xcpc-date-column">日期</th><th className="xcpc-progress-column">进度</th><th className="xcpc-problems-column">题目</th></tr></thead>
          <tbody>{visible.map((contest) => {
            const solved = contest.problems.filter((problem) => problem.solved).length;
            const percentage = contest.problems.length ? Math.round(solved / contest.problems.length * 100) : 0;
            return <tr key={contest.id} className={contest.problems.length > 0 && solved === contest.problems.length ? 'complete' : ''}>
              <td className="xcpc-contest-column"><button title={`${contest.name} · 在 QOJ 打开`} onClick={() => void api.openExternal(contest.url)}>{shortContestNames ? contest.shortName : contest.name}</button><div>{contest.series.map((value) => <span className="series" key={value}>{value}</span>)}<span>{contest.stage}</span><span>{contest.site}</span>{contest.boardSource && <span className="board">{contest.boardSource}</span>}</div></td>
              <td className="xcpc-date-column"><strong>{contest.date ? contest.date.slice(5) : '—'}</strong><small>{contest.year}</small></td>
              <td className="xcpc-progress-column"><strong>{solved} / {contest.problems.length}</strong><i><b style={{ width: `${percentage}%` }} /></i></td>
              <td className="xcpc-problems-cell"><div className="xcpc-problem-list">{contest.problems.map((problem) => {
                const rating = problem.tier ? ` · ${tierLabels[problem.tier]}` : '';
                const accepted = problem.acceptedTeams == null ? '' : ` · ${problem.acceptedTeams}${problem.totalTeams == null ? '' : `/${problem.totalTeams}`} 队通过`;
                const displayName = problem.name || '题目';
                return <div key={`${problem.index}-${problem.problemId}`} className={`xcpc-problem ${problem.solved ? 'solved' : ''} ${problem.tier ? `tier-${problem.tier}` : 'tier-unrated'}`} title={`${problem.index}. ${displayName}${rating}${accepted} · 点击打开 QOJ`}>
                  <button onClick={() => void api.openExternal(problem.url)}><strong>{showProblemNames ? `${problem.index}. ${displayName}` : problem.index}</strong>{showProblemNames && <small>{problem.acceptedTeams == null ? `QOJ #${problem.problemId}` : `${problem.acceptedTeams} 队通过`}</small>}</button>
                </div>;
              })}{contest.problems.length === 0 && <span className="xcpc-no-problems">暂无题目</span>}</div></td>
            </tr>;
          })}</tbody>
        </table>
        {!visible.length && <div className="empty">没有符合当前条件的比赛。</div>}
      </div>

      <footer className="xcpc-footer"><div className="xcpc-legend"><span className="gold"><i />金</span><span className="silver"><i />银</span><span className="bronze"><i />铜</span><span className="iron"><i />铁</span><em /><span className="solved"><i />已 AC</span>{!showProblemNames && <small>开启“题目名称”可查看题名与通过队数</small>}</div><div className="xcpc-pagination"><label>第 <select aria-label="跳转页码" value={Math.min(page, pageCount)} onChange={(event) => setPage(Number(event.target.value))}>{Array.from({ length: pageCount }, (_, index) => <option value={index + 1} key={index + 1}>{index + 1}</option>)}</select> / {pageCount} 页</label><button disabled={page <= 1} onClick={() => setPage((value) => Math.max(1, value - 1))}><ChevronLeft size={14} /></button><button disabled={page >= pageCount} onClick={() => setPage((value) => Math.min(pageCount, value + 1))}><ChevronRight size={14} /></button></div></footer>
    </section>
  </>;
}
