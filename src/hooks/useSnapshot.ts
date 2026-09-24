import { useCallback, useEffect, useRef, useState } from 'react';
import { emptySnapshot } from '../lib/ui';
import { api } from '../services/api';
import type { DayDetail, DifficultyDetail, Metric, Platform, Snapshot, SolvedGain } from '../types';

type DateRange = { start: string; end: string };

type UseSnapshotOptions = {
  selectedPlatform: Platform | null;
  range: DateRange;
  metric: Metric;
  accountFilter: string;
  sourceFilter: string;
  timeZone: string;
  notify: (message: string) => void;
};

export function useSnapshot(options: UseSnapshotOptions) {
  const { selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone, notify } = options;
  const [snapshot, setSnapshot] = useState<Snapshot>(emptySnapshot);
  const [solvedGains, setSolvedGains] = useState<SolvedGain[]>([]);
  const [loading, setLoading] = useState(true);
  const [dayDetail, setDayDetail] = useState<DayDetail | null>(null);
  const [dayLoading, setDayLoading] = useState(false);
  const [difficultyDetail, setDifficultyDetail] = useState<DifficultyDetail | null>(null);
  const [difficultyLoading, setDifficultyLoading] = useState(false);
  const snapshotRequest = useRef(0);
  const snapshotValue = useRef<Snapshot>(emptySnapshot);
  const solvedGainTimer = useRef(0);
  const dayRequest = useRef(0);
  const difficultyRequest = useRef(0);
  const query = useRef({ selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone });
  query.current = { selectedPlatform, range, metric, accountFilter, sourceFilter, timeZone };

  const closeDay = useCallback(() => {
    dayRequest.current += 1;
    setDayDetail(null);
    setDayLoading(false);
  }, []);

  const closeDifficulty = useCallback(() => {
    difficultyRequest.current += 1;
    setDifficultyDetail(null);
    setDifficultyLoading(false);
  }, []);

  const openDifficulty = async (platform: Platform, label: string, sourceOverride?: string) => {
    const request = ++difficultyRequest.current;
    setDifficultyDetail(null);
    setDifficultyLoading(true);
    try {
      const account = selectedPlatform === platform ? accountFilter || null : null;
      const source = sourceOverride || (selectedPlatform === 'nowcoder' && platform === 'nowcoder' ? sourceFilter || null : null);
      const detail = await api.difficultyDetail(platform, label, account, source);
      if (request === difficultyRequest.current) setDifficultyDetail(detail);
    } catch (error) {
      if (request === difficultyRequest.current) notify(String(error));
    } finally {
      if (request === difficultyRequest.current) setDifficultyLoading(false);
    }
  };

  const loadSnapshot = useCallback(async (showSolvedGains = false) => {
    const request = ++snapshotRequest.current;
    const current = query.current;
    setLoading(true);
    try {
      const result = await api.snapshot(
        current.selectedPlatform,
        current.range.start,
        current.range.end,
        current.metric,
        current.selectedPlatform ? current.accountFilter || null : null,
        current.selectedPlatform === 'nowcoder' ? current.sourceFilter || null : null,
        current.timeZone,
      );
      if (request !== snapshotRequest.current) return;
      if (showSolvedGains) {
        const before = new Map(snapshotValue.current.platforms.map((row) => [row.platform, row.solved]));
        const gains = result.platforms.flatMap((row) => {
          const previous = before.get(row.platform);
          const amount = row.solved != null && previous != null ? row.solved - previous : 0;
          return amount > 0 ? [{
            platform: row.platform,
            amount,
            id: Date.now() + Math.random(),
            offsetX: Math.round(Math.random() * 28 - 14),
            offsetY: Math.round(Math.random() * 20 - 12),
          }] : [];
        });
        if (gains.length) {
          window.clearTimeout(solvedGainTimer.current);
          setSolvedGains(gains);
          solvedGainTimer.current = window.setTimeout(() => setSolvedGains([]), 3400);
        }
      }
      snapshotValue.current = result;
      setSnapshot(result);
    } catch (error) {
      if (request === snapshotRequest.current) notify(String(error));
    } finally {
      if (request === snapshotRequest.current) setLoading(false);
    }
  }, [selectedPlatform, range.start, range.end, metric, accountFilter, sourceFilter, timeZone, notify]);

  const openDay = async (day: string) => {
    const request = ++dayRequest.current;
    setDayLoading(true);
    setDayDetail({ day, items: [], aggregates: [] });
    try {
      const result = await api.dayDetail(
        day,
        selectedPlatform,
        selectedPlatform ? accountFilter || null : null,
        selectedPlatform === 'nowcoder' ? sourceFilter || null : null,
        timeZone,
      );
      if (request === dayRequest.current) setDayDetail(result);
    } catch (error) {
      notify(String(error));
    } finally {
      if (request === dayRequest.current) setDayLoading(false);
    }
  };

  useEffect(() => {
    void loadSnapshot();
    closeDay();
    return () => { snapshotRequest.current += 1; };
  }, [loadSnapshot, closeDay]);

  useEffect(() => () => window.clearTimeout(solvedGainTimer.current), []);

  return {
    snapshot,
    solvedGains,
    loading,
    dayDetail,
    dayLoading,
    difficultyDetail,
    difficultyLoading,
    loadSnapshot,
    openDay,
    closeDay,
    openDifficulty,
    closeDifficulty,
  };
}
