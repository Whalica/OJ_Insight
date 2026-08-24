export interface StorageInfo {
  rootDir: string;
  dataDir: string;
  databasePath: string;
  exportDir: string;
  webviewDir: string;
}

import { invoke } from '@tauri-apps/api/core';
import type { AccountConfig, DayDetail, Metric, Platform, Snapshot, SyncResult, SyncStatus, UpdateInfo } from '../types';

export const api = {
  storageInfo: () => invoke<StorageInfo>('get_storage_info'),
  getAccounts: () => invoke<AccountConfig[]>('get_accounts'),
  saveAccount: (platform: Platform, account: string, secret: string) => invoke<void>('save_account', { platform, account, secret }),
  saveAccounts: (platform: Platform, accounts: AccountConfig[]) => invoke<void>('save_accounts', { platform, accounts }),
  getStatuses: () => invoke<SyncStatus[]>('get_sync_statuses'),
  syncPlatform: (platform: Platform, full = false) => invoke<SyncResult>('sync_platform', { platform, full }),
  syncAll: () => invoke<SyncResult[]>('sync_all'),
  clearPlatform: (platform: Platform) => invoke<void>('clear_platform_records', { platform }),
  clearAll: () => invoke<void>('clear_all_records'),
  snapshot: (platform: Platform | null, startDay: string | null, endDay: string | null, metric: Metric, account: string | null = null, source: string | null = null, timeZone = 'Asia/Shanghai') =>
    invoke<Snapshot>('get_snapshot', { platform, startDay, endDay, metric, account, source, timeZone }),
  dayDetail: (day: string, platform: Platform | null, account: string | null = null, source: string | null = null, timeZone = 'Asia/Shanghai') =>
    invoke<DayDetail>('get_day_detail', { day, platform, account, source, timeZone }),
  checkForUpdates: () => invoke<UpdateInfo>('check_for_updates'),
  openExternal: (url: string) => invoke<void>('open_external', { url }),
};
