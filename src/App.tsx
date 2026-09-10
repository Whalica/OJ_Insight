import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Download, X } from 'lucide-react';
import Sidebar from './components/Sidebar';
import DayDrawer from './components/DayDrawer';
import DifficultyDrawer from './components/DifficultyDrawer';
import DashboardPage from './pages/DashboardPage';
import AboutPage from './pages/AboutPage';
import DataPage from './pages/DataPage';
import ExportPage from './pages/ExportPage';
import SettingsPage from './pages/SettingsPage';
import XcpcTrackerPage from './pages/XcpcTrackerPage';
import { api } from './lib/api';
import { initialTimeZone, millisecondsUntilNextDay, today } from './lib/date';
import { PLATFORM_META, PLATFORM_ORDER } from './lib/platforms';
import { emptyAccounts, emptySnapshot, initialMetric, initialScope, scopeRange, SYNC_TIPS, type AccountMap, type TimeScope } from './lib/ui';
import { applyPreferences, loadPreferences, savePreferences, type Preferences } from './lib/preferences';
import { checkForAppUpdate, discardAppUpdate, installAppUpdate } from './lib/updater';
import type { DayDetail, DifficultyDetail, Metric, Platform, Snapshot, SyncStatus, UpdateInfo } from './types';

type Page = 'overview' | 'xcpc' | 'export' | 'data' | 'settings' | 'about' | Platform;

export default function App() {
  const [preferences, setPreferences] = useState<Preferences>(loadPreferences);
  const [page, setPage] = useState<Page>(() => {
    const saved = loadPreferences();
    const last = localStorage.getItem('oj-insight.last-page') as Page | null;
    const valid = ['overview', 'xcpc', 'export', 'data', 'settings', 'about', ...PLATFORM_ORDER].includes(last || '');
    return saved.startupPage === 'last' && last && valid ? last : 'overview';
  });
  const [timeZone, setTimeZoneState] = useState(initialTimeZone);
  const [timeScope, setTimeScopeState] = useState<TimeScope>(() => initialScope(timeZone));
  const [metric, setMetricState] = useState<Metric>(initialMetric);
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot);
  const [accounts, setAccounts] = useState<AccountMap>(emptyAccounts);
  const [statuses, setStatuses] = useState<SyncStatus[]>([]);
  const [accountFilter, setAccountFilter] = useState('');
  const [sourceFilter, setSourceFilter] = useState('');
  const [selectedDay, setSelectedDay] = useState(() => today(timeZone));
  const [loading, setLoading] = useState(true);
  const [accountsLoaded, setAccountsLoaded] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [syncTip, setSyncTip] = useState('');
  const [syncProgress, setSyncProgress] = useState<{ done: number; total: number; added: number; partial: number; failed: number } | null>(null);
  const [toast, setToast] = useState('');
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);
  const [difficultyDetail, setDifficultyDetail] = useState<DifficultyDetail | null>(null);
  const [difficultyLoading, setDifficultyLoading] = useState(false);
  const [availableUpdate, setAvailableUpdate] = useState<UpdateInfo | null>(null);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);

  const selectedPlatform: Platform | null = PLATFORM_ORDER.includes(page as Platform) ? page as Platform : null;
  const range = useMemo(() => scopeRange(timeScope, timeZone), [timeScope, selectedDay, timeZone]);
  const setTimeScope = (value: TimeScope) => { localStorage.setItem('oj-insight.time-scope', String(value)); setTimeScopeState(value); };
  const setMetric = (value: Metric) => { localStorage.setItem('oj-insight.metric', value); setMetricState(value); };
  const setTimeZone = (value: string) => { localStorage.setItem('oj-insight.time-zone', value); setTimeZoneState(value); setSelectedDay(today(value)); };
  const updatePreferences = (patch: Partial<Preferences>) => setPreferences((current) => {
    const next = { ...current, ...patch };
    savePreferences(next);
    return next;
  });
  const notify = (message: string) => { setToast(message); window.setTimeout(() => setToast(''), 3600); };

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
  const snapshotRequest = useRef(0);
  const dayRequest = useRef(0);
  const difficultyRequest = useRef(0);
  const startupSyncStarted = useRef(false);
  const startupUpdateStarted = useRef(false);
  const query = useRef({ selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone });
  query.current = { selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone };
  const closeDay = () => { dayRequest.current += 1; setDayDetail(null); setDayLoading(false); };
  const closeDifficulty = () => { difficultyRequest.current += 1; setDifficultyDetail(null); setDifficultyLoading(false); };
  const openDifficulty = async (platform: Platform, label: string) => {
    const request = ++difficultyRequest.current;
    setDifficultyDetail(null); setDifficultyLoading(true);
    try {
      const account = selectedPlatform === platform ? accountFilter || null : null;
      const source = selectedPlatform === 'nowcoder' && platform === 'nowcoder' ? sourceFilter || null : null;
      const detail = await api.difficultyDetail(platform, label, account, source);
      if (request === difficultyRequest.current) setDifficultyDetail(detail);
    } catch (error) { if (request === difficultyRequest.current) notify(String(error)); }
    finally { if (request === difficultyRequest.current) setDifficultyLoading(false); }
  };
  const loadSnapshot = useCallback(async () => {
    const request = ++snapshotRequest.current;
    const { selectedPlatform: platform, range: dates, metric: mode, accountFilter: account, sourceFilter: source, timeZone: zone } = query.current;
    setLoading(true);
    try {
      const result = await api.snapshot(platform, dates.start, dates.end, mode, platform ? account || null : null, platform === 'nowcoder' ? source || null : null, zone);
      if (request === snapshotRequest.current) setSnapshot(result);
    } catch (error) { if (request === snapshotRequest.current) notify(String(error)); }
    finally { if (request === snapshotRequest.current) setLoading(false); }
  }, [selectedPlatform, range.start, range.end, metric, accountFilter, sourceFilter, selectedDay, timeZone]);

  useEffect(() => { Promise.all([loadAccounts(), loadStatuses()]).catch((error) => notify(String(error))); }, [loadAccounts, loadStatuses]);
  useEffect(() => { setAccountFilter(''); setSourceFilter(''); }, [selectedPlatform]);
  useEffect(() => { closeDay(); closeDifficulty(); window.scrollTo({ top: 0, behavior: 'auto' }); localStorage.setItem('oj-insight.last-page', page); }, [page]);
  useEffect(() => { loadSnapshot(); closeDay(); return () => { snapshotRequest.current += 1; }; }, [loadSnapshot]);

  const chooseTip = () => setSyncTip(SYNC_TIPS[Math.floor(Math.random() * SYNC_TIPS.length)]);
  const syncOne = async (platform: Platform, full = false) => {
    setSyncing(platform); chooseTip();
    try {
      const result = await api.syncPlatform(platform, full);
      notify(`${PLATFORM_META[platform].name}：${result.message}`);
      await Promise.all([loadSnapshot(), loadStatuses()]);
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
      await Promise.all([loadSnapshot(), loadStatuses()]);
    } finally { setSyncing(null); window.setTimeout(() => setSyncProgress(null), 2600); }
  };
  useEffect(() => {
    if (!accountsLoaded || !preferences.autoSync || startupSyncStarted.current) return;
    startupSyncStarted.current = true;
    void syncAll(accounts, true);
  }, [accountsLoaded, preferences.autoSync]);
  useEffect(() => {
    if (!preferences.autoCheckUpdates || startupUpdateStarted.current) return;
    startupUpdateStarted.current = true;
    const timer = window.setTimeout(() => {
      checkForAppUpdate().then((result) => {
        if (result.updateAvailable && result.latestVersion !== preferences.skippedUpdateVersion) setAvailableUpdate(result);
      }).catch(() => undefined);
    }, 800);
    return () => window.clearTimeout(timer);
  }, [preferences.autoCheckUpdates, preferences.skippedUpdateVersion]);
  const installUpdate = async () => {
    if (syncing) { notify('请等待当前同步完成后再安装更新'); return; }
    setInstallingUpdate(true); setUpdateProgress(0);
    let downloaded = 0; let total = 0;
    try {
      await installAppUpdate((event) => {
        if (event.event === 'Started') total = event.data.contentLength || 0;
        if (event.event === 'Progress') downloaded += event.data.chunkLength;
        if (event.event === 'Progress') setUpdateProgress(total ? Math.min(100, Math.round(downloaded / total * 100)) : null);
        if (event.event === 'Finished') setUpdateProgress(100);
      });
    } catch (error) {
      notify(`更新失败：${String(error)}`); setInstallingUpdate(false); setUpdateProgress(null);
    }
  };
  const openDay = async (day: string) => {
    const request = ++dayRequest.current;
    setDayLoading(true); setDayDetail({ day, items: [], aggregates: [] });
    try { const result = await api.dayDetail(day, selectedPlatform, selectedPlatform ? accountFilter || null : null, selectedPlatform === 'nowcoder' ? sourceFilter || null : null, timeZone); if (request === dayRequest.current) setDayDetail(result); }
    catch (error) { notify(String(error)); }
    finally { if (request === dayRequest.current) setDayLoading(false); }
  };

  return <div className="app-shell">
    <Sidebar page={page} onChange={setPage} />
    <main className="main">
      {page === 'settings' ? <SettingsPage syncing={syncing} notify={notify} accounts={accounts} timeZone={timeZone} onTimeZone={setTimeZone} preferences={preferences} onPreferences={updatePreferences} onSaved={async () => { closeDay(); setAccountFilter(''); setSourceFilter(''); await Promise.all([loadAccounts(), loadSnapshot(), loadStatuses()]); notify('账号已保存，移除 ID 的本地记录已清理'); }} /> :
       page === 'data' ? <DataPage statuses={statuses} syncing={syncing} timeZone={timeZone} onSync={syncOne} onSyncAll={syncAll} onCleared={async () => { closeDay(); await Promise.all([loadSnapshot(), loadStatuses()]); }} notify={notify} /> :
       page === 'export' ? <ExportPage accounts={accounts} metric={metric} timeZone={timeZone} /> :
       page === 'about' ? <AboutPage syncing={syncing} /> :
       page === 'xcpc' ? <XcpcTrackerPage syncing={syncing === 'qoj'} onSync={() => syncOne('qoj').then(() => undefined)} notify={notify} /> :
      <DashboardPage platform={selectedPlatform} platformAccounts={selectedPlatform ? accounts[selectedPlatform] : []} accountFilter={accountFilter} setAccountFilter={setAccountFilter} sourceFilter={sourceFilter} setSourceFilter={setSourceFilter} timeScope={timeScope} setTimeScope={setTimeScope} range={range} metric={metric} setMetric={setMetric} timeZone={timeZone} snapshot={snapshot} loading={loading} syncing={syncing} syncTip={syncTip} syncProgress={syncProgress} onSync={() => selectedPlatform ? syncOne(selectedPlatform) : syncAll()} onDay={openDay} onDifficulty={openDifficulty} onPlatform={(platform) => setPage(platform)} />}
    </main>
    <DayDrawer detail={dayDetail} loading={dayLoading} timeZone={timeZone} onClose={closeDay} />
    <DifficultyDrawer detail={difficultyDetail} loading={difficultyLoading} timeZone={timeZone} onClose={closeDifficulty} />
    {availableUpdate && <aside className="update-notice" aria-live="polite"><button className="update-dismiss" aria-label="稍后提醒" disabled={installingUpdate} onClick={() => setAvailableUpdate(null)}><X size={15} /></button><small>UPDATE AVAILABLE</small><strong>OJ Insight v{availableUpdate.latestVersion}</strong><span>{installingUpdate ? `正在下载${updateProgress == null ? '…' : ` · ${updateProgress}%`}` : availableUpdate.installable === false ? '这个版本暂时需要从 Release 页面下载安装。' : syncing ? '当前正在同步数据，完成后即可安装更新。' : '新版本已经准备好，可以直接在应用内完成更新。'}</span>{installingUpdate && <i><b style={{ width: `${updateProgress || 4}%` }} /></i>}<div><button disabled={installingUpdate} onClick={() => { updatePreferences({ skippedUpdateVersion: availableUpdate.latestVersion }); setAvailableUpdate(null); void discardAppUpdate(); }}>跳过此版本</button><button className="primary" disabled={installingUpdate || (availableUpdate.installable !== false && !!syncing)} onClick={() => availableUpdate.installable === false ? api.openExternal(availableUpdate.releaseUrl) : installUpdate()}><Download size={14} />{availableUpdate.installable === false ? '手动下载' : installingUpdate ? '更新中' : syncing ? '等待同步' : '立即更新'}</button></div></aside>}
    {toast && <div className="toast">{toast}</div>}
  </div>;
}
