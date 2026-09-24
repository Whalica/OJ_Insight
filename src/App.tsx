import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Download, X } from 'lucide-react';
import Sidebar from './components/Sidebar';
import DayDrawer from './components/DayDrawer';
import DifficultyDrawer from './components/DifficultyDrawer';
import RelationshipNotice from './components/RelationshipNotice';
import DashboardPage from './pages/DashboardPage';
import AboutPage from './pages/AboutPage';
import DataPage from './pages/DataPage';
import ExportPage from './pages/ExportPage';
import ContestReviewPage from './pages/ContestReviewPage';
import SettingsPage from './pages/SettingsPage';
import RelationshipsPage from './pages/RelationshipsPage';
import XcpcTrackerPage from './pages/XcpcTrackerPage';
import ExternalTrackerPage, { type ExternalTracker } from './pages/ExternalTrackerPage';
import { api } from './services/api';
import { initialTimeZone, millisecondsUntilNextDay, today } from './lib/date';
import { PLATFORM_META, PLATFORM_ORDER } from './lib/platforms';
import { emptyAccounts, emptySnapshot, initialMetric, initialScope, recentHalfYearRange, scopeRange, SYNC_TIPS, type AccountMap, type TimeScope } from './lib/ui';
import { applyPreferences, loadPreferences, savePreferences, type Preferences } from './lib/preferences';
import { useUpdater } from './hooks/useUpdater';
import type { DayDetail, DifficultyDetail, Metric, Platform, Snapshot, SolvedGain, SyncStatus, WatchedAcEvent, WatchedBindingInput, WatchedPerson } from './types';

type Page = 'overview' | 'xcpc' | 'tracker-codeforces' | 'tracker-atcoder' | 'contest-review' | 'relationships' | 'export' | 'data' | 'settings' | 'about' | Platform;

export default function App() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem('oj-insight.sidebar-collapsed') === 'true');
  const [page, setPage] = useState<Page>(() => {
    const saved = loadPreferences();
    const last = localStorage.getItem('oj-insight.last-page') as Page | null;
    const valid = ['overview', 'xcpc', 'tracker-codeforces', 'tracker-atcoder', 'contest-review', 'relationships', 'export', 'data', 'settings', 'about', ...PLATFORM_ORDER].includes(last || '');
    return saved.startupPage === 'last' && last && valid ? last : 'overview';
  });
  const embeddedTracker = page.startsWith('tracker-') ? page.slice('tracker-'.length) as ExternalTracker : null;
  const [mountedTracker, setMountedTracker] = useState<ExternalTracker | null>(embeddedTracker);
  const [timeZone, setTimeZoneState] = useState(initialTimeZone);
  const [timeScope, setTimeScopeState] = useState<TimeScope>(() => initialScope(timeZone));
  const [metric, setMetricState] = useState<Metric>(initialMetric);
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot);
  const [solvedGains, setSolvedGains] = useState<SolvedGain[]>([]);
  const [accounts, setAccounts] = useState<AccountMap>(emptyAccounts);
  const [statuses, setStatuses] = useState<SyncStatus[]>([]);
  const [watchedPeople, setWatchedPeople] = useState<WatchedPerson[]>([]);
  const [watchedEvents, setWatchedEvents] = useState<WatchedAcEvent[]>([]);
  const [watchedNotifications, setWatchedNotifications] = useState<WatchedAcEvent[]>([]);
  const [watchedLoaded, setWatchedLoaded] = useState(false);
  const [watchedSyncing, setWatchedSyncing] = useState(false);
  const [autoWatch, setAutoWatchState] = useState(() => localStorage.getItem('oj-insight.relationship-auto-check') !== 'false');
  const watchedPeopleRef = useRef(watchedPeople);
  watchedPeopleRef.current = watchedPeople;
  const watchedSyncingRef = useRef(false);
  const syncingRef = useRef<string | null>(null);
  const [accountFilter, setAccountFilter] = useState('');
  const [sourceFilter, setSourceFilter] = useState('');
  const [selectedDay, setSelectedDay] = useState(() => today(timeZone));
  const [loading, setLoading] = useState(true);
  const [accountsLoaded, setAccountsLoaded] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);
  syncingRef.current = syncing;
  const [syncTip, setSyncTip] = useState('');
  const [syncProgress, setSyncProgress] = useState<{ done: number; total: number; added: number; partial: number; failed: number } | null>(null);
  const [toast, setToast] = useState('');
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);
  const [difficultyDetail, setDifficultyDetail] = useState<DifficultyDetail | null>(null);
  const [difficultyLoading, setDifficultyLoading] = useState(false);

  const selectedPlatform: Platform | null = PLATFORM_ORDER.includes(page as Platform) ? page as Platform : null;
  const range = useMemo(() => selectedPlatform === 'luogu' ? recentHalfYearRange(timeZone) : scopeRange(timeScope, timeZone), [selectedPlatform, timeScope, selectedDay, timeZone]);
  const setTimeScope = (value: TimeScope) => { localStorage.setItem('oj-insight.time-scope', String(value)); setTimeScopeState(value); };
  const setMetric = (value: Metric) => { localStorage.setItem('oj-insight.metric', value); setMetricState(value); };
  const setTimeZone = (value: string) => { localStorage.setItem('oj-insight.time-zone', value); setTimeZoneState(value); setSelectedDay(today(value)); };
  const updatePreferences = (patch: Partial<Preferences>) => setPreferences((current) => {
    const next = { ...current, ...patch };
    savePreferences(next);
    return next;
  });
  const notify = (message: string) => { setToast(message); window.setTimeout(() => setToast(''), 3600); };
  const {
    availableUpdate,
    installingUpdate,
    updateProgress,
    dismissUpdate,
    skipUpdate,
    installUpdate,
    openRelease,
  } = useUpdater({ preferences, syncing, notify, updatePreferences });

  useEffect(() => {
    let timer = 0;
    const schedule = () => {
      timer = window.setTimeout(() => { setSelectedDay(today(timeZone)); schedule(); }, millisecondsUntilNextDay(timeZone));
    };
    schedule(); return () => window.clearTimeout(timer);
  }, [timeZone]);

  useEffect(() => {
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const apply = () => applyPreferences(preferences, media.matches);
    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [preferences]);

  const loadAccounts = useCallback(async () => {
    const next = emptyAccounts();
    for (const entry of await api.getAccounts()) next[entry.platform].push(entry);
    setAccounts(next);
    setAccountsLoaded(true);
    return next;
  }, []);
  const loadStatuses = useCallback(async () => setStatuses(await api.getStatuses()), []);
  const loadWatched = useCallback(async () => {
    const [people, events] = await Promise.all([
      api.getWatchedPeople(),
      api.getWatchedEvents(preferences.watchedEventRetention),
    ]);
    const notifications = await api.getPendingWatchedNotifications();
    setWatchedPeople(people);
    setWatchedEvents(events);
    setWatchedNotifications(notifications);
    setWatchedLoaded(true);
  }, [preferences.watchedEventRetention]);
  const snapshotRequest = useRef(0);
  const snapshotValue = useRef<Snapshot>(emptySnapshot);
  const solvedGainTimer = useRef(0);
  const dayRequest = useRef(0);
  const difficultyRequest = useRef(0);
  const startupSyncStarted = useRef(false);
  const xcpcKnowledgeLoaded = useRef(false);
  const query = useRef({ selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone });
  query.current = { selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone };
  const closeDay = () => { dayRequest.current += 1; setDayDetail(null); setDayLoading(false); };
  const closeDifficulty = () => { difficultyRequest.current += 1; setDifficultyDetail(null); setDifficultyLoading(false); };
  const openDifficulty = async (platform: Platform, label: string, sourceOverride?: string) => {
    const request = ++difficultyRequest.current;
    setDifficultyDetail(null); setDifficultyLoading(true);
    try {
      const account = selectedPlatform === platform ? accountFilter || null : null;
      const source = sourceOverride || (selectedPlatform === 'nowcoder' && platform === 'nowcoder' ? sourceFilter || null : null);
      const detail = await api.difficultyDetail(platform, label, account, source);
      if (request === difficultyRequest.current) setDifficultyDetail(detail);
    } catch (error) { if (request === difficultyRequest.current) notify(String(error)); }
    finally { if (request === difficultyRequest.current) setDifficultyLoading(false); }
  };
  const loadSnapshot = useCallback(async (showSolvedGains = false) => {
    const request = ++snapshotRequest.current;
    const { selectedPlatform: platform, range: dates, metric: mode, accountFilter: account, sourceFilter: source, timeZone: zone } = query.current;
    setLoading(true);
    try {
      const result = await api.snapshot(platform, dates.start, dates.end, mode, platform ? account || null : null, platform === 'nowcoder' ? source || null : null, zone);
      if (request === snapshotRequest.current) {
        if (showSolvedGains) {
          const before = new Map(snapshotValue.current.platforms.map((row) => [row.platform, row.solved]));
          const gains = result.platforms.flatMap((row) => {
            const previous = before.get(row.platform);
            const amount = row.solved != null && previous != null ? row.solved - previous : 0;
            return amount > 0 ? [{ platform: row.platform, amount, id: Date.now() + Math.random(),
              offsetX: Math.round(Math.random() * 28 - 14), offsetY: Math.round(Math.random() * 20 - 12) }] : [];
          });
          if (gains.length) {
            window.clearTimeout(solvedGainTimer.current);
            setSolvedGains(gains);
            solvedGainTimer.current = window.setTimeout(() => setSolvedGains([]), 3400);
          }
        }
        snapshotValue.current = result;
        setSnapshot(result);
      }
    } catch (error) { if (request === snapshotRequest.current) notify(String(error)); }
    finally { if (request === snapshotRequest.current) setLoading(false); }
  }, [selectedPlatform, range.start, range.end, metric, accountFilter, sourceFilter, selectedDay, timeZone]);

  useEffect(() => { Promise.all([loadAccounts(), loadStatuses(), loadWatched()]).catch((error) => notify(String(error))); }, [loadAccounts, loadStatuses, loadWatched]);
  useEffect(() => {
    if (!accountsLoaded || xcpcKnowledgeLoaded.current || !accounts.qoj.some((entry) => entry.account.trim())) return;
    xcpcKnowledgeLoaded.current = true;
    api.getXcpcContests(false, false).then(() => loadSnapshot()).catch(() => undefined);
  }, [accountsLoaded, accounts.qoj, loadSnapshot]);
  useEffect(() => { setAccountFilter(''); setSourceFilter(''); }, [selectedPlatform]);
  useEffect(() => { if (selectedPlatform === 'luogu' && metric !== 'activity') setMetric('activity'); }, [selectedPlatform, metric]);
  useEffect(() => {
    closeDay(); closeDifficulty(); window.scrollTo({ top: 0, behavior: 'auto' }); localStorage.setItem('oj-insight.last-page', page);
    if (embeddedTracker) setMountedTracker(embeddedTracker);
  }, [page, embeddedTracker]);
  useEffect(() => { loadSnapshot(); closeDay(); return () => { snapshotRequest.current += 1; }; }, [loadSnapshot]);
  useEffect(() => () => window.clearTimeout(solvedGainTimer.current), []);

  const chooseTip = () => setSyncTip(SYNC_TIPS[Math.floor(Math.random() * SYNC_TIPS.length)]);
  const syncOne = async (platform: Platform, full = false) => {
    setSyncing(platform); chooseTip();
    try {
      const result = await api.syncPlatform(platform, full);
      notify(`${PLATFORM_META[platform].name}：${result.message}`);
      await Promise.all([loadSnapshot(true), loadStatuses()]);
    } catch (error) { notify(`${PLATFORM_META[platform].name}：${String(error)}`); await loadStatuses(); }
    finally { setSyncing(null); }
  };
  const syncAll = async (sourceAccounts: AccountMap = accounts, automatic = false) => {
    const configured = PLATFORM_ORDER.filter((platform) => sourceAccounts[platform].some((entry) => entry.account.trim()));
    if (!configured.length && automatic) return;
    setSyncing('all'); chooseTip();
    let done = 0; let added = 0; let partial = 0; let failed = 0;
    setSyncProgress({ done, total: configured.length, added, partial, failed });
    try {
      for (const platform of configured) {
        try { const result = await api.syncPlatform(platform); added += result.inserted; if (result.partial) partial += 1; else if (result.status !== 'ok' && result.status !== 'warning') failed += 1; } catch { failed += 1; }
        done += 1; setSyncProgress({ done, total: configured.length, added, partial, failed }); await loadStatuses();
      }
      if (!automatic || added > 0 || partial > 0 || failed > 0) {
        notify(configured.length ? `同步完成：新增 ${added} 条，部分可用 ${partial}，失败 ${failed}` : '还没有配置账号，请先到设置页填写');
      }
      await Promise.all([loadSnapshot(true), loadStatuses()]);
    } finally { setSyncing(null); window.setTimeout(() => setSyncProgress(null), 2600); }
  };
  const syncWatched = useCallback(async (personId: number | null = null, silent = false) => {
    if (watchedSyncingRef.current) return;
    if (syncingRef.current) {
      if (!silent) notify('请等待当前个人账号同步完成后再检查关注账号');
      return;
    }
    watchedSyncingRef.current = true;
    setWatchedSyncing(true);
    try {
      const result = personId == null ? await api.syncWatchedPeople() : await api.syncWatchedPerson(personId);
      if (result.events.length) {
        setWatchedEvents((current) => {
          const merged = new Map(current.map((event) => [event.id, event]));
          result.events.forEach((event) => merged.set(event.id, event));
          return [...merged.values()].sort((a, b) => b.createdAt - a.createdAt || b.id - a.id);
        });
      }
      await loadWatched();
      if (!silent) {
        const checkedPeople = personId == null
          ? new Set(watchedPeopleRef.current.map((person) => person.nickname.trim()
            ? `nickname:${person.nickname.trim().toLocaleLowerCase()}`
            : `account:${person.platform}:${person.account.trim().toLocaleLowerCase()}`)).size
          : 1;
        const checkedSummary = `检查 ${checkedPeople} 人（${result.checked} 个账号）`;
        if (result.failures.length) notify(`关注检查完成：${checkedSummary}；新 AC ${result.insertedEvents} 条；${result.failures.join('；')}`);
        else if (!result.checked) notify('还没有添加关注账号');
        else if (!result.insertedEvents) notify(`关注检查完成：${checkedSummary}，没有新的 AC`);
        else notify(`关注检查完成：${checkedSummary}，发现新 AC ${result.insertedEvents} 条`);
      }
    } catch (error) {
      if (!silent) notify('关注检查失败：' + String(error));
    } finally {
      watchedSyncingRef.current = false;
      setWatchedSyncing(false);
    }
  }, [loadWatched]);
  const saveWatched = async (nickname: string, relationship: string, bindings: WatchedBindingInput[]) => {
    await api.saveWatchedPeople(nickname, relationship, bindings);
    await loadWatched();
    notify(`关注已保存 ${bindings.length} 个平台；首次检查会先建立历史基线`);
  };
  const editWatched = async (personIds: number[], nickname: string, relationship: string, bindings: WatchedBindingInput[]) => {
    await api.editWatchedPerson(personIds, nickname, relationship, bindings);
    await loadWatched();
    notify(bindings.length ? `关注信息已更新，并添加 ${bindings.length} 个平台` : '关注信息已更新');
  };
  const deleteWatched = async (personId: number) => {
    await api.deleteWatchedPerson(personId);
    await loadWatched();
    notify('关注账号及其提醒记录已移除');
  };
  const dismissWatched = async (eventId: number) => {
    try {
      await api.dismissWatchedEvent(eventId);
      setWatchedEvents((current) => current.map((event) => event.id === eventId ? { ...event, dismissed: true } : event));
      setWatchedNotifications((current) => current.filter((event) => event.id !== eventId));
    } catch (error) {
      notify('关闭提醒失败：' + String(error));
    }
  };
  const setAutoWatch = (value: boolean) => {
    localStorage.setItem('oj-insight.relationship-auto-check', String(value));
    setAutoWatchState(value);
  };
  useEffect(() => {
    if (!watchedLoaded || !autoWatch) return;
    const startupDelay = window.setTimeout(() => { void syncWatched(null, true); }, preferences.autoSync ? 7000 : 1200);
    const interval = window.setInterval(() => { void syncWatched(null, true); }, 10 * 60 * 1000);
    return () => { window.clearTimeout(startupDelay); window.clearInterval(interval); };
  }, [watchedLoaded, autoWatch, preferences.autoSync, syncWatched]);
  useEffect(() => {
    if (!accountsLoaded || !preferences.autoSync || startupSyncStarted.current) return;
    startupSyncStarted.current = true;
    void syncAll(accounts, true);
  }, [accountsLoaded, preferences.autoSync]);
  const openDay = async (day: string) => {
    const request = ++dayRequest.current;
    setDayLoading(true); setDayDetail({ day, items: [], aggregates: [] });
    try { const result = await api.dayDetail(day, selectedPlatform, selectedPlatform ? accountFilter || null : null, selectedPlatform === 'nowcoder' ? sourceFilter || null : null, timeZone); if (request === dayRequest.current) setDayDetail(result); }
    catch (error) { notify(String(error)); }
    finally { if (request === dayRequest.current) setDayLoading(false); }
  };

  const toggleSidebar = () => setSidebarCollapsed((current) => {
    localStorage.setItem('oj-insight.sidebar-collapsed', String(!current));
    return !current;
  });
  return <div className={`app-shell ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
    <Sidebar page={page} onChange={setPage} collapsed={sidebarCollapsed} onToggle={toggleSidebar} />
    <main className={`main ${page.startsWith('tracker-') ? 'main-tracker' : ''}`}>
      {page === 'settings' ? <SettingsPage syncing={syncing} notify={notify} accounts={accounts} timeZone={timeZone} onTimeZone={setTimeZone} preferences={preferences} onPreferences={updatePreferences} onSaved={async () => { closeDay(); setAccountFilter(''); setSourceFilter(''); await Promise.all([loadAccounts(), loadSnapshot(), loadStatuses()]); notify('账号已保存，移除 ID 的本地记录已清理'); }} /> :
       page === 'contest-review' ? <ContestReviewPage accounts={accounts} notify={notify} onOpenSettings={() => setPage('settings')} /> :
      page === 'relationships' ? <RelationshipsPage people={watchedPeople} events={watchedEvents.slice(0, preferences.watchedEventRetention)} timeZone={timeZone} syncing={watchedSyncing || !!syncing} autoCheck={autoWatch} onAutoCheck={setAutoWatch} onSync={() => syncWatched()} onSyncPerson={(personId) => syncWatched(personId)} onSave={saveWatched} onEdit={editWatched} onDelete={deleteWatched} onDismiss={dismissWatched} notify={notify} /> :
       page === 'data' ? <DataPage statuses={statuses} syncing={syncing} timeZone={timeZone} onSync={syncOne} onSyncAll={syncAll} onCleared={async () => { closeDay(); await Promise.all([loadSnapshot(), loadStatuses()]); }} notify={notify} /> :
       page === 'export' ? <ExportPage accounts={accounts} metric={metric} timeZone={timeZone} /> :
       page === 'about' ? <AboutPage syncing={syncing} /> :
       page === 'xcpc' ? <XcpcTrackerPage syncing={syncing === 'qoj'} onSync={() => syncOne('qoj').then(() => undefined)} notify={notify} /> :
       embeddedTracker ? null :
      <DashboardPage platform={selectedPlatform} platformAccounts={selectedPlatform ? accounts[selectedPlatform] : []} accountFilter={accountFilter} setAccountFilter={setAccountFilter} sourceFilter={sourceFilter} setSourceFilter={setSourceFilter} timeScope={timeScope} setTimeScope={setTimeScope} range={range} metric={metric} setMetric={setMetric} timeZone={timeZone} snapshot={snapshot} solvedGains={solvedGains} loading={loading} syncing={syncing} syncTip={syncTip} syncProgress={syncProgress} onSync={() => selectedPlatform ? syncOne(selectedPlatform) : syncAll()} onDay={openDay} onDifficulty={openDifficulty} onPlatform={(platform) => setPage(platform)} onOpenSettings={() => setPage('settings')} />}
      {mountedTracker && <div className={`tracker-keepalive-layer ${embeddedTracker === mountedTracker ? 'active' : ''}`} aria-hidden={embeddedTracker !== mountedTracker}><ExternalTrackerPage tracker={mountedTracker} accounts={accounts[mountedTracker] || []} /></div>}
    </main>
    <DayDrawer detail={dayDetail} loading={dayLoading} timeZone={timeZone} onClose={closeDay} />
    <DifficultyDrawer detail={difficultyDetail} loading={difficultyLoading} timeZone={timeZone} onClose={closeDifficulty} />
    <RelationshipNotice events={watchedNotifications} timeZone={timeZone} onDismiss={(eventId) => { void dismissWatched(eventId); }} />
    {availableUpdate && <aside className="update-notice" aria-live="polite"><button className="update-dismiss" aria-label="稍后提醒" disabled={installingUpdate} onClick={dismissUpdate}><X size={15} /></button><small>UPDATE AVAILABLE</small><strong>OJ Insight v{availableUpdate.latestVersion}</strong><span>{installingUpdate ? `正在下载${updateProgress == null ? '…' : ` · ${updateProgress}%`}` : availableUpdate.installable === false ? '这个版本暂时需要从 Release 页面下载安装。' : syncing ? '当前正在同步数据，完成后即可安装更新。' : '新版本已经准备好，可以直接在应用内完成更新。'}</span>{installingUpdate && <i><b style={{ width: `${updateProgress || 4}%` }} /></i>}<div><button disabled={installingUpdate} onClick={skipUpdate}>跳过此版本</button><button className="primary" disabled={installingUpdate || (availableUpdate.installable !== false && !!syncing)} onClick={() => availableUpdate.installable === false ? openRelease() : installUpdate()}><Download size={14} />{availableUpdate.installable === false ? '手动下载' : installingUpdate ? '更新中' : syncing ? '等待同步' : '立即更新'}</button></div></aside>}
    {toast && <div className="toast">{toast}</div>}
  </div>;
}
