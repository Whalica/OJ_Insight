import { useEffect, useState } from 'react';
import { Clock3, Pause, Play, RotateCcw, Square, X } from 'lucide-react';
import { MarkdownPreview } from './MarkdownNote';

const STORAGE_KEY = 'oj-insight.study-assistant.v1';

type TimerStatus = 'idle' | 'running' | 'paused' | 'stopped';
interface AssistantState {
  elapsedMs: number;
  startedAt: number | null;
  status: TimerStatus;
  note: string;
}

const emptyState: AssistantState = { elapsedMs: 0, startedAt: null, status: 'idle', note: '' };

function loadState(): AssistantState {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) || 'null') as Partial<AssistantState> | null;
    if (!saved || !['idle', 'running', 'paused', 'stopped'].includes(saved.status || '')) return emptyState;
    return {
      elapsedMs: typeof saved.elapsedMs === 'number' && Number.isFinite(saved.elapsedMs) ? Math.max(0, saved.elapsedMs) : 0,
      startedAt: typeof saved.startedAt === 'number' && Number.isFinite(saved.startedAt) ? saved.startedAt : null,
      status: saved.status === 'running' && !saved.startedAt ? 'paused' : saved.status as TimerStatus,
      note: typeof saved.note === 'string' ? saved.note : '',
    };
  } catch {
    return emptyState;
  }
}

function saveState(value: AssistantState) {
  try { localStorage.setItem(STORAGE_KEY, JSON.stringify(value)); } catch { /* Storage may be unavailable. */ }
}

function formatElapsed(milliseconds: number) {
  const seconds = Math.floor(Math.max(0, milliseconds) / 1000);
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor(seconds / 60) % 60;
  const remainder = seconds % 60;
  return [hours, minutes, remainder].map((part) => String(part).padStart(2, '0')).join(':');
}

export default function StudyAssistant({ visible, expanded, onExpand, onClose }: {
  visible: boolean;
  expanded: boolean;
  onExpand: (value: boolean) => void;
  onClose: () => void;
}) {
  const [session, setSession] = useState<AssistantState>(loadState);
  const [now, setNow] = useState(Date.now);
  const [noteView, setNoteView] = useState<'edit' | 'preview'>('edit');
  useEffect(() => { saveState(session); }, [session]);
  useEffect(() => {
    if (session.status !== 'running') return;
    const interval = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(interval);
  }, [session.status]);

  const elapsed = session.elapsedMs + (session.status === 'running' && session.startedAt != null ? Math.max(0, now - session.startedAt) : 0);
  const update = (change: (current: AssistantState) => AssistantState) => {
    setNow(Date.now());
    setSession((current) => change(current));
  };
  const start = () => update((current) => ({
    ...current,
    elapsedMs: current.status === 'stopped' ? 0 : current.elapsedMs,
    startedAt: Date.now(),
    status: 'running',
  }));
  const pause = () => update((current) => ({
    ...current,
    elapsedMs: current.elapsedMs + (current.startedAt == null ? 0 : Math.max(0, Date.now() - current.startedAt)),
    startedAt: null,
    status: 'paused',
  }));
  const stop = () => update((current) => ({
    ...current,
    elapsedMs: current.elapsedMs + (current.startedAt == null ? 0 : Math.max(0, Date.now() - current.startedAt)),
    startedAt: null,
    status: 'stopped',
  }));
  const close = () => {
    if (session.status === 'running') {
      const paused = { ...session, elapsedMs: elapsed, startedAt: null, status: 'paused' as const };
      saveState(paused);
      setSession(paused);
    }
    onClose();
  };
  if (!visible) return null;

  return <aside className={`study-assistant ${expanded ? 'expanded' : 'collapsed'}`} aria-label="做题小助手">
    {expanded ? <div className="study-assistant-panel">
      <header><div><small>STUDY ASSISTANT</small><strong>做题小助手</strong></div><div className="study-assistant-window-actions"><button type="button" onClick={() => onExpand(false)} aria-label="收起小助手">−</button><button type="button" onClick={close} aria-label="关闭小助手"><X size={16} /></button></div></header>
      <div className="study-assistant-timer"><Clock3 size={20} /><time aria-label="已用时间">{formatElapsed(elapsed)}</time><span>{session.status === 'running' ? '计时中' : session.status === 'paused' ? '已暂停' : session.status === 'stopped' ? '已停止' : '准备开始'}</span></div>
      <div className="study-assistant-controls">
        {session.status === 'running' ? <button type="button" onClick={pause}><Pause size={15} />暂停</button> : <button type="button" className="primary" onClick={start}><Play size={15} />{session.status === 'paused' ? '继续' : '开始'}</button>}
        <button type="button" onClick={stop} disabled={session.status === 'idle' || session.status === 'stopped'}><Square size={14} />停止</button>
        <button type="button" onClick={() => update((current) => ({ ...current, elapsedMs: 0, startedAt: null, status: 'idle' }))} disabled={session.status === 'idle'}><RotateCcw size={14} />重置</button>
      </div>
      <div className="study-assistant-notes">
        <div className="study-assistant-note-head"><strong>Markdown 笔记</strong><span>自动保存在本机</span></div>
        <div className="study-assistant-note-tabs"><button type="button" className={noteView === 'edit' ? 'active' : ''} onClick={() => setNoteView('edit')}>编辑</button><button type="button" className={noteView === 'preview' ? 'active' : ''} onClick={() => setNoteView('preview')}>预览</button></div>
        {noteView === 'edit' ? <textarea aria-label="小助手 Markdown 笔记" value={session.note} onChange={(event) => setSession((current) => {
          const next = { ...current, note: event.target.value };
          saveState(next);
          return next;
        })} placeholder="记录思路、样例和待验证的想法。支持 Markdown、$行内公式$ 与 $$独立公式$$。" /> : <div className="study-assistant-preview"><MarkdownPreview text={session.note || '暂无笔记'} /></div>}
      </div>
    </div> : <button type="button" className="study-assistant-bubble" onClick={() => onExpand(true)} aria-label={`展开做题小助手，已计时 ${formatElapsed(elapsed)}`}><Clock3 size={17} /><time>{formatElapsed(elapsed)}</time><small>{session.status === 'running' ? '计时中' : session.status === 'paused' ? '已暂停' : session.status === 'stopped' ? '已停止' : '小助手'}</small></button>}
  </aside>;
}
