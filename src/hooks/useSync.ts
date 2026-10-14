import { useEffect, useRef, useState } from 'react';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import { SYNC_TIPS, type AccountMap } from '../lib/ui';
import { api } from '../services/api';
import type { Platform } from '../types';

type SyncProgress = { done: number; total: number; partial: number; failed: number };

type UseSyncOptions = {
  accounts: AccountMap;
  accountsLoaded: boolean;
  autoSync: boolean;
  loadSnapshot: (showSolvedGains?: boolean) => Promise<void>;
  loadStatuses: () => Promise<void>;
  notify: (message: string) => void;
};

export function useSync(options: UseSyncOptions) {
  const { accounts, accountsLoaded, autoSync, loadSnapshot, loadStatuses, notify } = options;
  const [syncing, setSyncing] = useState<string | null>(null);
  const [syncTip, setSyncTip] = useState('');
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);
  const startupSyncStarted = useRef(false);

  const chooseTip = () => setSyncTip(SYNC_TIPS[Math.floor(Math.random() * SYNC_TIPS.length)]);

  const syncOne = async (platform: Platform, full = false) => {
    setSyncing(platform);
    chooseTip();
    try {
      const result = await api.syncPlatform(platform, full);
      notify(`${PLATFORM_META[platform].name}：${result.message}`);
      await Promise.all([loadSnapshot(true), loadStatuses()]);
    } catch (error) {
      notify(`${PLATFORM_META[platform].name}：${String(error)}`);
      await loadStatuses();
    } finally {
      setSyncing(null);
    }
  };

  const syncAll = async (sourceAccounts: AccountMap = accounts, automatic = false) => {
    const configured = PLATFORM_ORDER.filter((platform) => sourceAccounts[platform].some((entry) => entry.account.trim()));
    if (!configured.length && automatic) return;
    setSyncing('all');
    chooseTip();
    let done = 0;
    let partial = 0;
    let failed = 0;
    let changedPlatforms = 0;
    const problems: string[] = [];
    setSyncProgress({ done, total: configured.length, partial, failed });
    try {
      for (const platform of configured) {
        try {
          const result = await api.syncPlatform(platform);
          if (result.inserted > 0 || result.updated > 0) changedPlatforms += 1;
          if (result.partial || result.status === 'warning') {
            partial += 1;
            problems.push(`${PLATFORM_META[platform].name}：${result.message}`);
          } else if (result.status !== 'ok') {
            failed += 1;
            problems.push(`${PLATFORM_META[platform].name}：${result.message}`);
          }
        } catch (error) {
          failed += 1;
          problems.push(`${PLATFORM_META[platform].name}：${String(error)}`);
        }
        done += 1;
        setSyncProgress({ done, total: configured.length, partial, failed });
        await loadStatuses();
      }
      if (!automatic || changedPlatforms > 0 || partial > 0 || failed > 0) {
        notify(configured.length ? `同步完成：${configured.length - partial - failed} 个平台成功，${changedPlatforms} 个平台有新增或更新，部分可用 ${partial}，失败 ${failed}${problems.length ? `；${problems.join('；')}` : ''}` : '还没有配置账号，请先到设置页填写');
      }
      await Promise.all([loadSnapshot(true), loadStatuses()]);
    } finally {
      setSyncing(null);
      window.setTimeout(() => setSyncProgress(null), 2600);
    }
  };

  useEffect(() => {
    if (!accountsLoaded || !autoSync || startupSyncStarted.current) return;
    startupSyncStarted.current = true;
    void syncAll(accounts, true);
  }, [accountsLoaded, autoSync]);

  return { syncing, syncTip, syncProgress, syncOne, syncAll };
}
