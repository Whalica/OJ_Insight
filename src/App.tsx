import { useCallback, useEffect, useMemo, useState } from 'react';
import Sidebar from './components/Sidebar';
import DayDrawer from './components/DayDrawer';
import DashboardPage from './pages/DashboardPage';
import { AboutPage, DataPage, ExportPage, SettingsPage } from './pages/UtilityPages';
import { api } from './lib/api';
import { initialTimeZone, millisecondsUntilNextDay, today } from './lib/date';
import { PLATFORM_META, PLATFORM_ORDER } from './lib/platforms';
import { emptyAccounts, emptySnapshot, initialMetric, initialScope, scopeRange, SYNC_TIPS, type AccountMap, type TimeScope } from './lib/ui';
import type { DayDetail, Metric, Platform, Snapshot, SyncStatus } from './types';

type Page = 'overview' | 'export' | 'data' | 'settings' | 'about' | Platform;

export default function App() {
  const [page, setPage] = useState<Page>('overview');
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
  const [syncing, setSyncing] = useState<string | null>(null);
  const [syncTip, setSyncTip] = useState('');
  const [syncProgress, setSyncProgress] = useState<{ done: number; total: number; added: number; failed: number } | null>(null);
  const [toast, setToast] = useState('');
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);

  const selectedPlatform: Platform | null = PLATFORM_ORDER.includes(page as Platform) ? page as Platform : null;
  const range = useMemo(() => scopeRange(timeScope, timeZone), [timeScope, selectedDay, timeZone]);
  const setTimeScope = (value: TimeScope) => { localStorage.setItem('oj-insight.time-scope', String(value)); setTimeScopeState(value); };
  const setMetric = (value: Metric) => { localStorage.setItem('oj-insight.metric', value); setMetricState(value); };
  const setTimeZone = (value: string) => { localStorage.setItem('oj-insight.time-zone', value); setTimeZoneState(value); setSelectedDay(today(value)); };
  const notify = (message: string) => { setToast(message); window.setTimeout(() => setToast(''), 3600); };

  useEffect(() => {
    let timer = 0;
    const schedule = () => {
      timer = window.setTimeout(() => { setSelectedDay(today(timeZone)); schedule(); }, millisecondsUntilNextDay(timeZone));
    };
    schedule(); return () => window.clearTimeout(timer);
  }, [timeZone]);

  const loadAccounts = useCallback(async () => {
    const next = emptyAccounts();
    for (const entry of await api.getAccounts()) next[entry.platform].push(entry);
    setAccounts(next);
  }, []);
  const loadStatuses = useCallback(async () => setStatuses(await api.getStatuses()), []);
  const loadSnapshot = useCallback(async () => {
    setLoading(true);
    try {
      setSnapshot(await api.snapshot(selectedPlatform, range.start, range.end, metric, selectedPlatform ? accountFilter || null : null, selectedPlatform === 'nowcoder' ? sourceFilter || null : null, timeZone));
    } catch (error) { notify(String(error)); }
    finally { setLoading(false); }
  }, [selectedPlatform, range.start, range.end, metric, accountFilter, sourceFilter, selectedDay, timeZone]);

  useEffect(() => { Promise.all([loadAccounts(), loadStatuses()]).catch((error) => notify(String(error))); }, [loadAccounts, loadStatuses]);
  useEffect(() => { setAccountFilter(''); setSourceFilter(''); }, [selectedPlatform]);
  useEffect(() => { window.scrollTo({ top: 0, behavior: 'auto' }); }, [page]);
  useEffect(() => { loadSnapshot(); }, [loadSnapshot]);

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
  const syncAll = async () => {
    setSyncing('all'); chooseTip();
    const configured = PLATFORM_ORDER.filter((platform) => accounts[platform].some((entry) => entry.account.trim()));
    let done = 0; let added = 0; let failed = 0;
    setSyncProgress({ done, total: configured.length, added, failed });
    try {
      for (const platform of configured) {
        try { const result = await api.syncPlatform(platform); added += result.inserted; } catch { failed += 1; }
        done += 1; setSyncProgress({ done, total: configured.length, added, failed }); await loadStatuses();
      }
      notify(configured.length ? `同步完成：新增 ${added} 条，${failed} 个平台失败` : '还没有配置账号，请先到设置页填写');
      await Promise.all([loadSnapshot(), loadStatuses()]);
    } finally { setSyncing(null); window.setTimeout(() => setSyncProgress(null), 2600); }
  };
  const openDay = async (day: string) => {
    setDayLoading(true); setDayDetail({ day, items: [], aggregates: [] });
    try { setDayDetail(await api.dayDetail(day, selectedPlatform, selectedPlatform ? accountFilter || null : null, selectedPlatform === 'nowcoder' ? sourceFilter || null : null, timeZone)); }
    catch (error) { notify(String(error)); }
    finally { setDayLoading(false); }
  };

  return <div className="app-shell">
    <Sidebar page={page} onChange={setPage} onIssue={() => api.openExternal('https://github.com/Whalica/OJ_Insight/issues')} />
    <main className="main">
      {page === 'settings' ? <SettingsPage accounts={accounts} timeZone={timeZone} onTimeZone={setTimeZone} onSaved={async () => { await loadAccounts(); notify('账号设置已保存'); }} /> :
       page === 'data' ? <DataPage statuses={statuses} syncing={syncing} timeZone={timeZone} onSync={syncOne} onSyncAll={syncAll} onCleared={async () => { await Promise.all([loadSnapshot(), loadStatuses()]); }} notify={notify} /> :
       page === 'export' ? <ExportPage accounts={accounts} metric={metric} timeZone={timeZone} /> :
       page === 'about' ? <AboutPage /> :
       <DashboardPage platform={selectedPlatform} platformAccounts={selectedPlatform ? accounts[selectedPlatform] : []} accountFilter={accountFilter} setAccountFilter={setAccountFilter} sourceFilter={sourceFilter} setSourceFilter={setSourceFilter} timeScope={timeScope} setTimeScope={setTimeScope} range={range} metric={metric} setMetric={setMetric} timeZone={timeZone} snapshot={snapshot} loading={loading} syncing={syncing} syncTip={syncTip} syncProgress={syncProgress} onSync={() => selectedPlatform ? syncOne(selectedPlatform) : syncAll()} onDay={openDay} onPlatform={(platform) => setPage(platform)} />}
    </main>
    <DayDrawer detail={dayDetail} loading={dayLoading} timeZone={timeZone} onClose={() => setDayDetail(null)} />
    {toast && <div className="toast">{toast}</div>}
  </div>;
}
