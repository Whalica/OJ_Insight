import { useCallback, useEffect, useState } from 'react';
import { api } from '../services/api';
import type { ProblemSet, ProblemSetInput, TrainingMatch, TrainingMode } from '../types';

export function useTraining(notify: (message: string) => void) {
  const [sets, setSets] = useState<ProblemSet[]>([]);
  const [matches, setMatches] = useState<TrainingMatch[]>([]);
  const [loading, setLoading] = useState(true);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const [nextSets, nextMatches] = await Promise.all([api.listProblemSets(), api.listTrainingMatches()]);
      setSets(nextSets);
      setMatches(nextMatches);
    } catch (error) { notify(String(error)); }
    finally { setLoading(false); }
  }, [notify]);

  useEffect(() => { void reload(); }, [reload]);

  const saveSet = async (input: ProblemSetInput) => { const result = await api.saveProblemSet(input); await reload(); return result; };
  const deleteSet = async (id: number) => { await api.deleteProblemSet(id); await reload(); };
  const importSet = async (data: string) => { const result = await api.importProblemSet(data); await reload(); return result; };
  const startMatch = async (problemSetId: number, mode: TrainingMode, durationMinutes: number, targetMin: number, targetMax: number) => { const result = await api.startTrainingMatch(problemSetId, mode, durationMinutes, targetMin, targetMax); await reload(); return result; };
  const importMatch = async (data: string) => { const result = await api.importMatchManifest(data); await reload(); return result; };
  const refreshMatch = async (id: number) => { const result = await api.refreshTrainingMatch(id); await reload(); return result; };
  const finishMatch = async (id: number) => { const result = await api.finishTrainingMatch(id); await reload(); return result; };
  const deleteMatch = async (id: number) => { await api.deleteTrainingMatch(id); await reload(); };

  return { sets, matches, loading, reload, saveSet, deleteSet, importSet, startMatch, importMatch, refreshMatch, finishMatch, deleteMatch };
}
