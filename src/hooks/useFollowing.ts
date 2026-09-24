import { useCallback, useEffect, useRef, useState } from 'react';
import { api } from '../services/api';
import type { WatchedAcEvent, WatchedBindingInput, WatchedPerson } from '../types';

type UseFollowingOptions = {
  retention: number;
  autoSync: boolean;
  syncing: string | null;
  notify: (message: string) => void;
};

export function useFollowing({ retention, autoSync, syncing, notify }: UseFollowingOptions) {
  const [watchedPeople, setWatchedPeople] = useState<WatchedPerson[]>([]);
  const [watchedEvents, setWatchedEvents] = useState<WatchedAcEvent[]>([]);
  const [watchedNotifications, setWatchedNotifications] = useState<WatchedAcEvent[]>([]);
  const [watchedLoaded, setWatchedLoaded] = useState(false);
  const [watchedSyncing, setWatchedSyncing] = useState(false);
  const [autoWatch, setAutoWatchState] = useState(() => localStorage.getItem('oj-insight.relationship-auto-check') !== 'false');
  const watchedPeopleRef = useRef(watchedPeople);
  const watchedSyncingRef = useRef(false);
  const syncingRef = useRef(syncing);
  watchedPeopleRef.current = watchedPeople;
  syncingRef.current = syncing;

  const loadWatched = useCallback(async () => {
    const [people, events] = await Promise.all([
      api.getWatchedPeople(),
      api.getWatchedEvents(retention),
    ]);
    const notifications = await api.getPendingWatchedNotifications();
    setWatchedPeople(people);
    setWatchedEvents(events);
    setWatchedNotifications(notifications);
    setWatchedLoaded(true);
  }, [retention]);

  useEffect(() => {
    loadWatched().catch((error) => notify(String(error)));
  }, [loadWatched, notify]);

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
  }, [loadWatched, notify]);

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
    const startupDelay = window.setTimeout(() => { void syncWatched(null, true); }, autoSync ? 7000 : 1200);
    const interval = window.setInterval(() => { void syncWatched(null, true); }, 10 * 60 * 1000);
    return () => {
      window.clearTimeout(startupDelay);
      window.clearInterval(interval);
    };
  }, [watchedLoaded, autoWatch, autoSync, syncWatched]);

  return {
    watchedPeople,
    watchedEvents,
    watchedNotifications,
    watchedSyncing,
    autoWatch,
    syncWatched,
    saveWatched,
    editWatched,
    deleteWatched,
    dismissWatched,
    setAutoWatch,
  };
}
