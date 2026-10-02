import { invoke, isTauri } from '@tauri-apps/api/core';

export interface SolveMistake {
  id: string;
  at: number;
  reason: string;
  note: string;
  lostMinutes: number | null;
  elapsedMs?: number | null;
}

export interface SolveRecord {
  id: string;
  title: string;
  url: string;
  platform: string;
  status: 'draft' | 'finished';
  outcome: 'pending' | 'solved' | 'unfinished';
  createdAt: number;
  updatedAt: number;
  endedAt: number | null;
  elapsedMs: number;
  note: string;
  tags: string[];
  mistakes: SolveMistake[];
  sourceGroup?: string;
  sourceBatchId?: string;
}

const BROWSER_KEY = 'oj-insight.solve-journal.v1';
const SHADOW_PREFIX = 'oj-insight.solve-journal.pending.';
const LEGACY_KEY = 'oj-insight.study-assistant.v1';
const MIGRATED_KEY = 'oj-insight.study-assistant.migrated';
const ACTIVE_KEY = 'oj-insight.solve-journal.active-id';
export const JOURNAL_CHANGED = 'oj-insight.solve-journal.changed';

function browserRecords(): SolveRecord[] {
  try { return JSON.parse(localStorage.getItem(BROWSER_KEY) || '[]') as SolveRecord[]; } catch { return []; }
}

function shadowRecords(): SolveRecord[] {
  const records: SolveRecord[] = [];
  for (let index = 0; index < localStorage.length; index++) {
    const key = localStorage.key(index);
    if (!key?.startsWith(SHADOW_PREFIX)) continue;
    try { records.push(JSON.parse(localStorage.getItem(key) || '') as SolveRecord); } catch { /* Ignore invalid recovery data. */ }
  }
  return records;
}

function announce() { window.dispatchEvent(new Event(JOURNAL_CHANGED)); }

export function activeSolveDraftId() { return localStorage.getItem(ACTIVE_KEY); }
export function selectSolveDraft(id: string | null) { if (id) localStorage.setItem(ACTIVE_KEY, id); else localStorage.removeItem(ACTIVE_KEY); announce(); }

export function stageSolveRecord(record: SolveRecord): void {
  localStorage.setItem(`${SHADOW_PREFIX}${record.id}`, JSON.stringify(record));
  announce();
}

export function newSolveRecord(): SolveRecord {
  const now = Date.now();
  return { id: crypto.randomUUID(), title: '', url: '', platform: '', status: 'draft', outcome: 'pending', createdAt: now, updatedAt: now, endedAt: null, elapsedMs: 0, note: '', tags: [], mistakes: [] };
}

let saveQueue: Promise<unknown> = Promise.resolve();
export function saveSolveRecord(record: SolveRecord): Promise<void> {
  stageSolveRecord(record);
  const operation = async () => {
    if (isTauri()) await invoke('save_solve_record', { record });
    else {
      const records = browserRecords().filter((item) => item.id !== record.id);
      records.push(record);
      localStorage.setItem(BROWSER_KEY, JSON.stringify(records));
    }
    const key = `${SHADOW_PREFIX}${record.id}`;
    try {
      const pending = JSON.parse(localStorage.getItem(key) || 'null') as SolveRecord | null;
      if (pending?.updatedAt === record.updatedAt) localStorage.removeItem(key);
    } catch { /* Preserve recovery data if it cannot be read. */ }
    announce();
  };
  const next = saveQueue.catch(() => undefined).then(operation);
  saveQueue = next;
  return next;
}

export async function listSolveRecords(): Promise<SolveRecord[]> {
  const records = isTauri() ? await invoke<SolveRecord[]>('list_solve_records') : browserRecords();
  const merged = new Map(records.map((record) => [record.id, record]));
  for (const shadow of shadowRecords()) {
    const saved = merged.get(shadow.id);
    if (!saved || shadow.updatedAt >= saved.updatedAt) {
      merged.set(shadow.id, shadow);
      if (!saved || shadow.updatedAt > saved.updatedAt) {
        try {
          if (isTauri()) await invoke('save_solve_record', { record: shadow });
          else localStorage.setItem(BROWSER_KEY, JSON.stringify([...browserRecords().filter((item) => item.id !== shadow.id), shadow]));
          const key = `${SHADOW_PREFIX}${shadow.id}`;
          const pending = JSON.parse(localStorage.getItem(key) || 'null') as SolveRecord | null;
          if (pending?.updatedAt === shadow.updatedAt) localStorage.removeItem(key);
        } catch { /* Keep recovery data until a later read succeeds. */ }
      }
    }
  }
  return [...merged.values()].sort((a, b) => b.updatedAt - a.updatedAt);
}

export async function deleteSolveRecord(id: string): Promise<void> {
  await saveQueue.catch(() => undefined);
  if (isTauri()) await invoke('delete_solve_record', { id });
  else localStorage.setItem(BROWSER_KEY, JSON.stringify(browserRecords().filter((item) => item.id !== id)));
  localStorage.removeItem(`${SHADOW_PREFIX}${id}`);
  if (activeSolveDraftId() === id) selectSolveDraft(null);
  announce();
}

export async function migrateLegacyAssistant(): Promise<void> {
  if (localStorage.getItem(MIGRATED_KEY)) return;
  let legacy: { elapsedMs?: number; startedAt?: number | null; status?: string; note?: string } | null = null;
  try { legacy = JSON.parse(localStorage.getItem(LEGACY_KEY) || 'null'); } catch { /* No usable old state. */ }
  if (legacy && (legacy.note?.trim() || legacy.elapsedMs || legacy.status === 'running')) {
    const record = newSolveRecord();
    record.id = 'legacy-study-assistant';
    record.title = '旧版小助手笔记';
    record.note = legacy.note || '';
    record.elapsedMs = Math.max(0, Number(legacy.elapsedMs) || 0) + (legacy.status === 'running' && legacy.startedAt ? Math.max(0, Date.now() - legacy.startedAt) : 0);
    await saveSolveRecord(record);
  }
  localStorage.setItem(MIGRATED_KEY, 'true');
}

export function exportSolveJson(records: SolveRecord[]): string {
  return JSON.stringify({ schema: 'com.ojinsight.solve-journal', schemaVersion: 1, exportedAt: new Date().toISOString(), records }, null, 2);
}

function csvCell(value: string | number | null): string {
  const text = value == null ? '' : String(value);
  return `"${text.replaceAll('"', '""')}"`;
}

export function exportSolveCsv(records: SolveRecord[]): string {
  const rows = [['ID', '题目', '平台', '链接', '状态', '完成情况', '开始时间', '结束时间', '用时分钟', '标签', '失误次数', '笔记']];
  for (const record of records) rows.push([record.id, record.title, record.platform, record.url, record.status, record.outcome, new Date(record.createdAt).toISOString(), record.endedAt ? new Date(record.endedAt).toISOString() : '', String(Math.round(record.elapsedMs / 60000 * 10) / 10), record.tags.join(' / '), String(record.mistakes.length), record.note]);
  return '\uFEFF' + rows.map((row) => row.map(csvCell).join(',')).join('\r\n');
}

export function exportMistakesCsv(records: SolveRecord[]): string {
  const rows = [['记录 ID', '题目', '平台', '失误时间', '失误原因', '备注', '发生时计时秒数']];
  for (const record of records) for (const mistake of record.mistakes) rows.push([record.id, record.title, record.platform, new Date(mistake.at).toISOString(), mistake.reason, mistake.note, mistake.elapsedMs == null ? '' : String(Math.floor(mistake.elapsedMs / 1000))]);
  return '\uFEFF' + rows.map((row) => row.map(csvCell).join(',')).join('\r\n');
}
