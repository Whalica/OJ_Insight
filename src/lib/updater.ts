import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';
import type { UpdateInfo } from '../types';

let pendingUpdate: Update | null = null;
let checking: Promise<UpdateInfo> | null = null;

function info(update: Update | null): UpdateInfo {
  if (!update) {
    return {
      currentVersion: '',
      latestVersion: '',
      releaseUrl: 'https://github.com/Whalica/OJ_Insight/releases',
      updateAvailable: false,
    };
  }
  return {
    currentVersion: update.currentVersion,
    latestVersion: update.version,
    releaseUrl: `https://github.com/Whalica/OJ_Insight/releases/tag/v${update.version}`,
    updateAvailable: true,
    notes: update.body || '',
    publishedAt: update.date || '',
  };
}

export function checkForAppUpdate() {
  if (pendingUpdate) return Promise.resolve(info(pendingUpdate));
  if (checking) return checking;
  checking = check({ timeout: 15_000 })
    .then(async (update) => {
      if (pendingUpdate && pendingUpdate !== update) await pendingUpdate.close();
      pendingUpdate = update;
      return info(update);
    })
    .finally(() => { checking = null; });
  return checking;
}

export async function installAppUpdate(onEvent?: (event: DownloadEvent) => void) {
  if (!pendingUpdate) await checkForAppUpdate();
  if (!pendingUpdate) throw new Error('当前没有可安装的新版本');
  await pendingUpdate.downloadAndInstall(onEvent, { timeout: 10 * 60_000 });
  await relaunch();
}

export async function discardAppUpdate() {
  if (pendingUpdate) await pendingUpdate.close();
  pendingUpdate = null;
}
