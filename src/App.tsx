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
import ProblemSetsPage from './pages/ProblemSetsPage';
import TrainingPage from './pages/TrainingPage';
import ExternalTrackerPage, { type ExternalTracker } from './pages/ExternalTrackerPage';
import { api } from './services/api';
import { initialTimeZone, millisecondsUntilNextDay, today } from './lib/date';
import type { Page } from './lib/navigation';
import { PLATFORM_ORDER } from './lib/platforms';
import { initialMetric, initialScope, recentHalfYearRange, scopeRange, type TimeScope } from './lib/ui';
import { applyPreferences, loadPreferences, savePreferences, type Preferences } from './lib/preferences';
import { useAccounts } from './hooks/useAccounts';
import { useFollowing } from './hooks/useFollowing';
import { useSnapshot } from './hooks/useSnapshot';
import { useSync } from './hooks/useSync';
import { useUpdater } from './hooks/useUpdater';
import type { Metric, Platform } from './types';

export default function App() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(() => localStorage.getItem('oj-insight.sidebar-collapsed') === 'true');
  const [page, setPage] = useState<Page>(() => {
    const saved = loadPreferences();
    const last = localStorage.getItem('oj-insight.last-page') as Page | null;
    const valid = ['overview', 'xcpc', 'tracker-codeforces', 'tracker-atcoder', 'contest-review', 'problem-sets', 'training', 'relationships', 'export', 'data', 'settings', 'about', ...PLATFORM_ORDER].includes(last || '');
    return saved.startupPage === 'last' && last && valid ? last : 'overview';
  });
  const embeddedTracker = page.startsWith('tracker-') ? page.slice('tracker-'.length) as ExternalTracker : null;
  const [mountedTracker, setMountedTracker] = useState<ExternalTracker | null>(embeddedTracker);
  const [timeZone, setTimeZoneState] = useState(initialTimeZone);
  const [timeScope, setTimeScopeState] = useState<TimeScope>(() => initialScope(timeZone));
  const [metric, setMetricState] = useState<Metric>(initialMetric);
  const [accountFilter, setAccountFilter] = useState('');
  const [sourceFilter, setSourceFilter] = useState('');
  const [selectedDay, setSelectedDay] = useState(() => today(timeZone));
  const [toast, setToast] = useState('');

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
  const notify = useCallback((message: string) => {
    setToast(message);
    window.setTimeout(() => setToast(''), 3600);
  }, []);
  const { accounts, statuses, accountsLoaded, loadAccounts, loadStatuses } = useAccounts();
  const {
    snapshot, solvedGains, loading,
    dayDetail, dayLoading, difficultyDetail, difficultyLoading,
    loadSnapshot, openDay, closeDay, openDifficulty, closeDifficulty,
  } = useSnapshot({ selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone, notify });
  const { syncing, syncTip, syncProgress, syncOne, syncAll } = useSync({
    accounts,
    accountsLoaded,
    autoSync: preferences.autoSync,
    loadSnapshot,
    loadStatuses,
    notify,
  });
  const {
    watchedPeople, watchedEvents, watchedNotifications, watchedSyncing, autoWatch,
    syncWatched, saveWatched, editWatched, deleteWatched, dismissWatched, setAutoWatch,
  } = useFollowing({
    retention: preferences.watchedEventRetention,
    autoSync: preferences.autoSync,
    syncing,
    notify,
  });
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

  const xcpcKnowledgeLoaded = useRef(false);
  useEffect(() => {
    Promise.all([loadAccounts(), loadStatuses()]).catch((error) => notify(String(error)));
  }, [loadAccounts, loadStatuses, notify]);
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
  const toggleSidebar = () => setSidebarCollapsed((current) => {
    localStorage.setItem('oj-insight.sidebar-collapsed', String(!current));
    return !current;
  });
  return <div className={`app-shell ${sidebarCollapsed ? 'sidebar-collapsed' : ''}`}>
    <Sidebar page={page} onChange={setPage} collapsed={sidebarCollapsed} onToggle={toggleSidebar} />
    <main className={`main ${page.startsWith('tracker-') ? 'main-tracker' : ''}`}>
      {page === 'training' ? <TrainingPage notify={notify} onOpenProblemSets={() => setPage('problem-sets')} /> :
       page === 'problem-sets' ? <ProblemSetsPage notify={notify} onTrain={(setId) => { localStorage.setItem('oj-insight.training-set-id', String(setId)); setPage('training'); }} /> :
       page === 'settings' ? <SettingsPage syncing={syncing} notify={notify} accounts={accounts} timeZone={timeZone} onTimeZone={setTimeZone} preferences={preferences} onPreferences={updatePreferences} onSaved={async () => { closeDay(); setAccountFilter(''); setSourceFilter(''); await Promise.all([loadAccounts(), loadSnapshot(), loadStatuses()]); notify('账号已保存，移除 ID 的本地记录已清理'); }} /> :
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
