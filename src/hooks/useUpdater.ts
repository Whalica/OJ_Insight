import { useEffect, useRef, useState } from 'react';
import type { Preferences } from '../lib/preferences';
import { api } from '../services/api';
import { cancelAppUpdate, checkForAppUpdate, discardAppUpdate, installAppUpdate } from '../services/updater';
import type { UpdateInfo } from '../types';

type UseUpdaterOptions = {
  preferences: Preferences;
  syncing: string | null;
  notify: (message: string) => void;
  updatePreferences: (patch: Partial<Preferences>) => void;
};

export function useUpdater({ preferences, syncing, notify, updatePreferences }: UseUpdaterOptions) {
  const [availableUpdate, setAvailableUpdate] = useState<UpdateInfo | null>(null);
  const [installingUpdate, setInstallingUpdate] = useState(false);
  const [updateProgress, setUpdateProgress] = useState<number | null>(null);
  const [updateStage, setUpdateStage] = useState<'checking' | 'downloading' | 'verifying' | 'installing' | null>(null);
  const startupUpdateStarted = useRef(false);

  useEffect(() => {
    if (!preferences.autoCheckUpdates || startupUpdateStarted.current) return;
    startupUpdateStarted.current = true;
    const timer = window.setTimeout(() => {
      checkForAppUpdate().then((result) => {
        if (result.updateAvailable && result.latestVersion !== preferences.skippedUpdateVersion) {
          setAvailableUpdate(result);
        }
      }).catch(() => undefined);
    }, 800);
    return () => window.clearTimeout(timer);
  }, [preferences.autoCheckUpdates, preferences.skippedUpdateVersion]);

  const checkUpdate = async () => {
    const result = await checkForAppUpdate();
    if (result.updateAvailable) setAvailableUpdate(result);
    return result;
  };
  const installUpdate = async () => {
    if (installingUpdate) return;
    if (syncing) {
      notify('请等待当前同步完成后再安装更新');
      return;
    }
    setInstallingUpdate(true);
    setUpdateStage('checking');
    setUpdateProgress(0);
    let downloaded = 0;
    let total = 0;
    try {
      await installAppUpdate((event) => {
        if (event.event === 'Started') { total = event.data.contentLength || 0; setUpdateStage('downloading'); }
        if (event.event === 'Progress') { downloaded += event.data.chunkLength; setUpdateStage('downloading'); }
        if (event.event === 'Progress') {
          setUpdateProgress(total ? Math.min(100, Math.round(downloaded / total * 100)) : null);
        }
        if (event.event === 'Finished') { setUpdateProgress(100); setUpdateStage('verifying'); }
        if (event.event === 'Installing') setUpdateStage('installing');
      });
    } catch (error) {
      notify(String(error).includes('更新下载已取消') ? '已取消更新下载' : `更新失败：${String(error)}`);
      setInstallingUpdate(false);
      setUpdateProgress(null);
      setUpdateStage(null);
    }
  };

  const cancelUpdate = async () => {
    if (!installingUpdate || updateStage === 'installing') return;
    try { await cancelAppUpdate(); }
    catch (error) { notify(`取消下载失败：${String(error)}`); }
  };

  const dismissUpdate = () => setAvailableUpdate(null);
  const skipUpdate = () => {
    if (!availableUpdate) return;
    updatePreferences({ skippedUpdateVersion: availableUpdate.latestVersion });
    setAvailableUpdate(null);
    void discardAppUpdate();
  };
  const openRelease = () => {
    if (availableUpdate) void api.openExternal(availableUpdate.releaseUrl);
  };

  return {
    availableUpdate,
    installingUpdate,
    updateProgress,
    updateStage,
    checkUpdate,
    cancelUpdate,
    dismissUpdate,
    skipUpdate,
    installUpdate,
    openRelease,
  };
}
