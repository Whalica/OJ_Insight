import { useEffect, useRef, useState, type PointerEvent } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Clock3, Flag, Pause, Play, RotateCcw, X } from 'lucide-react';
import { MarkdownPreview } from './MarkdownNote';
import { parseProblemUrl } from '../lib/problemIdentity';
import { PLATFORM_META, TRAINING_PLATFORM_ORDER } from '../lib/platforms';
import { activeSolveDraftId, JOURNAL_CHANGED, listSolveRecords, migrateLegacyAssistant, newSolveRecord, saveSolveRecord, selectSolveDraft, stageSolveRecord, type SolveRecord } from '../services/solveJournal';
import { useI18n } from '../lib/i18n';

const resizeDirections = ['North', 'NorthEast', 'East', 'SouthEast', 'South', 'SouthWest', 'West', 'NorthWest'] as const;
const commonTags = ['DP', '贪心', '图论', '搜索', '数学', '数据结构', '字符串', '二分'];
const reasons = ['粗心', '思路偏差', '实现错误', '知识盲点', '时间分配', '其他'];
const tagsIn = (note: string) => [...new Set([...note.matchAll(/(?:^|\s)#([^\s#]+)/gu)].map((match) => match[1]))];
const hasWork = (record: SolveRecord, running: boolean) => !!(record.title.trim() || record.url.trim() || record.note.trim() || record.elapsedMs || record.mistakes.length || running);
function formatElapsed(ms: number) { const s = Math.floor(Math.max(0, ms) / 1000); return [Math.floor(s / 3600), Math.floor(s / 60) % 60, s % 60].map((n) => String(n).padStart(2, '0')).join(':'); }

export default function StudyAssistant({ visible, expanded, detached = false, onExpand, onClose }: { visible: boolean; expanded: boolean; detached?: boolean; onExpand: (value: boolean) => void; onClose: () => void }) {
  const { t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
  const [record, setRecord] = useState<SolveRecord | null>(null);
  const [startedAt, setStartedAt] = useState<number | null>(null);
  const [now, setNow] = useState(Date.now);
  const [stopped, setStopped] = useState(false);
  const [message, setMessage] = useState('');
  const [reason, setReason] = useState(reasons[0]);
  const [mistakeNote, setMistakeNote] = useState('');
  const [outcome, setOutcome] = useState<'solved' | 'unfinished'>('unfinished');
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const latest = useRef<SolveRecord | null>(null);
  const runningAt = useRef<number | null>(null);
  const timer = useRef<number | null>(null);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const dragOrigin = useRef<{ x: number; y: number } | null>(null);
  const wasDragged = useRef(false);

  useEffect(() => { let active = true; void (async () => { await migrateLegacyAssistant(); const rows = await listSolveRecords(); if (!active) return; const draft = rows.find((row) => row.id === activeSolveDraftId() && row.status === 'draft') || rows.find((row) => row.status === 'draft') || newSolveRecord(); latest.current = draft; setRecord(draft); selectSolveDraft(draft.id); })().catch((error) => setMessage(tRef.current('读取失败：{error}', { error: String(error) }))); return () => { active = false; }; }, []);
  useEffect(() => { if (startedAt == null) return; const id = window.setInterval(() => setNow(Date.now()), 1000); return () => window.clearInterval(id); }, [startedAt]);
  useEffect(() => () => { if (timer.current != null) window.clearTimeout(timer.current); }, []);
  useEffect(() => {
    const id = window.setInterval(() => {
      const current = latest.current;
      const started = runningAt.current;
      if (!current || started == null) return;
      const time = Date.now();
      runningAt.current = time;
      setStartedAt(time);
      setNow(time);
      change({ elapsedMs: current.elapsedMs + Math.max(0, time - started) }, true);
    }, 15_000);
    return () => window.clearInterval(id);
  }, []);
  useEffect(() => {
    const flushOnUnload = () => {
      const current = latest.current;
      if (!current || !hasWork(current, runningAt.current != null)) return;
      const time = Date.now();
      stageSolveRecord({ ...current, elapsedMs: current.elapsedMs + (runningAt.current == null ? 0 : Math.max(0, time - runningAt.current)), updatedAt: Math.max(time, current.updatedAt + 1) });
    };
    window.addEventListener('beforeunload', flushOnUnload);
    return () => window.removeEventListener('beforeunload', flushOnUnload);
  }, []);
  useEffect(() => {
    let switching = false;
    const switchDraft = () => { const id = activeSolveDraftId(); if (id === latest.current?.id || switching) return; switching = true; void (async () => {
      if (!id) {
        const rows = await listSolveRecords();
        if (latest.current && !rows.some((item) => item.id === latest.current?.id)) {
          if (timer.current != null) window.clearTimeout(timer.current);
          runningAt.current = null; setStartedAt(null);
          const fresh = newSolveRecord(); latest.current = fresh; setRecord(fresh); selectSolveDraft(fresh.id);
        }
        return;
      }
      if (timer.current != null) window.clearTimeout(timer.current);
      timer.current = null;
      const previous = latest.current;
      if (previous && hasWork(previous, runningAt.current != null)) {
        const time = Date.now();
        await saveSolveRecord({ ...previous, elapsedMs: previous.elapsedMs + (runningAt.current == null ? 0 : Math.max(0, time - runningAt.current)), updatedAt: time });
      }
      runningAt.current = null; setStartedAt(null);
      const next = (await listSolveRecords()).find((item) => item.id === id && item.status === 'draft');
      if (next) { latest.current = next; setRecord(next); setMessage(tRef.current('已切换到所选草稿')); }
    })().catch((error) => setMessage(tRef.current('切换草稿失败：{error}', { error: String(error) }))).finally(() => { switching = false; }); };
    window.addEventListener('storage', switchDraft);
    window.addEventListener(JOURNAL_CHANGED, switchDraft);
    return () => { window.removeEventListener('storage', switchDraft); window.removeEventListener(JOURNAL_CHANGED, switchDraft); };
  }, []);

  const elapsed = (record?.elapsedMs || 0) + (startedAt == null ? 0 : Math.max(0, now - startedAt));
  const persist = async (value: SolveRecord) => { if (timer.current != null) window.clearTimeout(timer.current); timer.current = null; try { await saveSolveRecord(value); } catch (error) { setMessage(t('保存失败：{error}', { error: String(error) })); } };
  const change = (patch: Partial<SolveRecord>, immediate = false) => {
    if (!latest.current) return;
    const next = { ...latest.current, ...patch, updatedAt: Math.max(Date.now(), latest.current.updatedAt + 1) };
    latest.current = next; setRecord(next); stageSolveRecord(next);
    if (timer.current != null) window.clearTimeout(timer.current);
    if (immediate) void persist(next); else timer.current = window.setTimeout(() => { if (latest.current) void persist(latest.current); }, 450);
  };
  const start = () => { if (!latest.current) return; if (stopped && elapsed > 0 && !window.confirm(t('重新开始计时会将本次用时清零，继续吗？'))) return; const time = Date.now(); if (stopped) change({ elapsedMs: 0 }, true); setStopped(false); runningAt.current = time; setStartedAt(time); setNow(time); change({}, true); };
  const pause = () => { const time = Date.now(); const startTime = runningAt.current; runningAt.current = null; setStartedAt(null); setStopped(false); setNow(time); if (startTime != null) change({ elapsedMs: (latest.current?.elapsedMs || 0) + Math.max(0, time - startTime) }, true); };
  const stop = () => { const time = Date.now(); const startTime = runningAt.current; runningAt.current = null; setStartedAt(null); setStopped(true); setNow(time); if (startTime != null) change({ elapsedMs: (latest.current?.elapsedMs || 0) + Math.max(0, time - startTime) }, true); };
  const close = async () => { const current = latest.current; if (current && hasWork(current, runningAt.current != null)) { const time = Date.now(); const next = { ...current, elapsedMs: current.elapsedMs + (runningAt.current == null ? 0 : Math.max(0, time - runningAt.current)), updatedAt: time }; try { await saveSolveRecord(next); } catch (error) { setMessage(t('关闭前保存失败：{error}', { error: String(error) })); return; } } onClose(); };
  const finish = async () => {
    const current = latest.current; if (!current) return;
    if (!current.title.trim() && !current.url.trim() && !current.note.trim() && !current.elapsedMs && runningAt.current == null && !current.mistakes.length) { setMessage(t('请先填写题目、记录笔记或开始计时')); return; }
    const time = Date.now();
    const next: SolveRecord = { ...current, title: current.title.trim() || current.url.trim() || '未命名练习', status: 'finished', outcome, elapsedMs: current.elapsedMs + (runningAt.current == null ? 0 : Math.max(0, time - runningAt.current)), updatedAt: time, endedAt: time };
    if (timer.current != null) window.clearTimeout(timer.current);
    try { await saveSolveRecord(next); runningAt.current = null; setStartedAt(null); setStopped(false); const fresh = newSolveRecord(); latest.current = fresh; setRecord(fresh); selectSolveDraft(fresh.id); setMessage(t('已保存到训练中心 · 解题手记')); } catch (error) { setMessage(t('归档失败：{error}', { error: String(error) })); }
  };
  const addMistake = () => {
    const current = latest.current;
    if (!current) return;
    const time = Date.now();
    const elapsedMs = current.elapsedMs + (runningAt.current == null ? 0 : Math.max(0, time - runningAt.current));
    if (runningAt.current != null) { runningAt.current = time; setStartedAt(time); }
    setNow(time);
    change({ elapsedMs, mistakes: [...current.mistakes, { id: crypto.randomUUID(), at: time, reason, note: mistakeNote.trim(), lostMinutes: null, elapsedMs }] }, true);
    setMistakeNote('');
  };
  const insertTag = (tag: string) => { const input = textarea.current; const current = latest.current; if (!input || !current) return; const caret = input.selectionStart; const match = current.note.slice(0, caret).match(/(?:^|\s)#([^\s#]*)$/u); const from = match ? caret - match[1].length - 1 : caret; const insertion = `${match ? '' : ' '}#${tag} `; const note = current.note.slice(0, from) + insertion + current.note.slice(caret); change({ note, tags: tagsIn(note) }); setSuggestions([]); window.requestAnimationFrame(() => { input.focus(); input.setSelectionRange(from + insertion.length, from + insertion.length); }); };
  const beginDrag = (event: PointerEvent<HTMLElement>) => { if (!detached || event.button !== 0 || (event.target as HTMLElement).closest('button') && expanded) return; dragOrigin.current = { x: event.screenX, y: event.screenY }; wasDragged.current = false; };
  const drag = (event: PointerEvent<HTMLElement>) => { const origin = dragOrigin.current; if (!origin || Math.hypot(event.screenX - origin.x, event.screenY - origin.y) < 6) return; dragOrigin.current = null; wasDragged.current = true; void invoke('drag_study_assistant'); };
  if (!visible) return null;

  return <aside className={`study-assistant ${expanded ? 'expanded' : 'collapsed'} ${detached ? 'detached' : ''}`} aria-label={t('做题小助手')}>
    {expanded ? <div className="study-assistant-panel">
      {detached && resizeDirections.map((direction) => <div key={direction} className={`study-assistant-resize study-assistant-resize-${direction.toLowerCase()}`} aria-label={t('调整小助手窗口大小：{direction}', { direction })} onPointerDown={(event) => { if (event.button === 0) { event.preventDefault(); event.stopPropagation(); void getCurrentWindow().startResizeDragging(direction); } }} />)}
      <header onPointerDown={beginDrag} onPointerMove={drag}><div><small lang="en">STUDY ASSISTANT</small><strong>{t('做题小助手')}</strong></div><div className="study-assistant-window-actions"><button type="button" onClick={() => onExpand(false)} aria-label={t('收起小助手')}>−</button><button type="button" onClick={() => void close()} aria-label={t('关闭小助手')}><X size={16} /></button></div></header>
      <div className="study-assistant-body">
        <div className="study-assistant-problem"><input aria-label={t('题目名称')} placeholder={t('题目名称（可稍后填写）')} value={record?.title || ''} onChange={(event) => change({ title: event.target.value })} /><input aria-label={t('题目链接')} placeholder={t('题目链接（可选）')} value={record?.url || ''} onChange={(event) => { const url = event.target.value; change({ url, platform: parseProblemUrl(url)?.platform || record?.platform || '' }); }} /><select aria-label={t('题目平台')} value={record?.platform || ''} onChange={(event) => change({ platform: event.target.value })}><option value="">{t('未指定平台')}</option>{TRAINING_PLATFORM_ORDER.map((platform) => <option key={platform} value={platform}>{t(PLATFORM_META[platform].name)}</option>)}</select></div>
        <div className="study-assistant-timer"><Clock3 size={20} /><time aria-label={t('已用时间')}>{formatElapsed(elapsed)}</time><span>{startedAt != null ? t('计时中') : stopped ? t('已停止') : elapsed > 0 ? t('已暂停') : t('准备开始')}</span></div>
        <div className="study-assistant-controls">{startedAt != null ? <button type="button" onClick={pause}><Pause size={15} />{t('暂停')}</button> : <button type="button" className="primary" onClick={start}><Play size={15} />{stopped ? t('重新开始') : elapsed > 0 ? t('继续') : t('开始')}</button>}<button type="button" onClick={stop} disabled={startedAt == null && (elapsed === 0 || stopped)}>{t('停止')}</button><button type="button" onClick={() => { if (elapsed > 0 && !window.confirm(t('重置本次计时？笔记和失误记录会保留。'))) return; runningAt.current = null; setStartedAt(null); setStopped(false); change({ elapsedMs: 0 }, true); }} disabled={elapsed === 0}><RotateCcw size={14} />{t('重置')}</button></div>
        <div className="study-assistant-mistakes"><select aria-label={t('失误原因')} value={reason} onChange={(event) => setReason(event.target.value)}>{reasons.map((item) => <option key={item} value={item}>{t(item)}</option>)}</select><input aria-label={t('失误备注')} placeholder={t('备注（可选）')} value={mistakeNote} onChange={(event) => setMistakeNote(event.target.value)} /><button type="button" onClick={addMistake}>{t('+ 记一次失误')}</button><small>{t('已记 {count} 次', { count: record?.mistakes.length || 0 })}</small></div>
        <div className="study-assistant-finish"><select aria-label={t('完成情况')} value={outcome} onChange={(event) => setOutcome(event.target.value as 'solved' | 'unfinished')}><option value="solved">{t('已完成')}</option><option value="unfinished">{t('未完成 / 待复习')}</option></select><button type="button" onClick={() => void finish()}><Flag size={14} />{t('结束本次做题')}</button></div>
        {message && <p className="study-assistant-status" role="status">{message}</p>}
        <div className="study-assistant-notes"><div className="study-assistant-note-head"><strong>{t('Markdown 笔记')}</strong><span>{t('自动保存草稿 · 输入 # 补全标签')}</span></div><div className="study-assistant-note-columns"><div className="study-assistant-note-pane"><span>{t('编辑')}</span><textarea ref={textarea} aria-label={t('小助手 Markdown 笔记')} value={record?.note || ''} onChange={(event) => { const note = event.target.value; change({ note, tags: tagsIn(note) }); const prefix = note.slice(0, event.target.selectionStart).match(/(?:^|\s)#([^\s#]*)$/u)?.[1]; setSuggestions(prefix == null ? [] : commonTags.filter((tag) => tag.toLowerCase().includes(prefix.toLowerCase())).slice(0, 6)); }} onKeyDown={(event) => { if (suggestions.length && (event.key === 'Tab' || event.key === 'Enter')) { event.preventDefault(); insertTag(suggestions[0]); } }} placeholder={t('记录思路、样例和待验证的想法。支持 Markdown、$行内公式$ 与 $$独立公式$$。')} />{suggestions.length > 0 && <div className="study-assistant-tags">{suggestions.map((tag) => <button type="button" key={tag} onClick={() => insertTag(tag)}>#{tag}</button>)}</div>}</div><div className="study-assistant-note-pane"><span>{t('预览')}</span><div className="study-assistant-preview"><MarkdownPreview text={record?.note || t('暂无笔记')} /></div></div></div></div>
      </div>
    </div> : <button type="button" className="study-assistant-bubble" onPointerDown={beginDrag} onPointerMove={drag} onClick={() => { if (wasDragged.current) { wasDragged.current = false; return; } onExpand(true); }} aria-label={t('展开做题小助手，已计时 {elapsed}', { elapsed: formatElapsed(elapsed) })}><Clock3 size={17} /><time>{formatElapsed(elapsed)}</time><small>{startedAt != null ? t('计时中') : stopped ? t('已停止') : elapsed > 0 ? t('已暂停') : t('小助手')}</small></button>}
  </aside>;
}
