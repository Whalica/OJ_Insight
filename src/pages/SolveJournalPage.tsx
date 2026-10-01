import { useCallback, useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { BookOpenText, Download, FileUp, Play, Search, Trash2 } from 'lucide-react';
import { MarkdownPreview } from '../components/MarkdownNote';
import { JOURNAL_CHANGED, deleteSolveRecord, exportMistakesCsv, exportSolveCsv, exportSolveJson, listSolveRecords, migrateLegacyAssistant, saveSolveRecord, selectSolveDraft, type SolveRecord } from '../services/solveJournal';
import { saveTrainingFile } from '../services/training';
import { api, type CompanionStatus } from '../services/api';

function duration(ms: number) { const minutes = Math.round(ms / 6000) / 10; return `${minutes} 分钟`; }
function date(value: number | null) { return value == null ? '—' : new Date(value).toLocaleString(); }
function localDay(value: number) { const day = new Date(value); return `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, '0')}-${String(day.getDate()).padStart(2, '0')}`; }

export default function SolveJournalPage({ notify, onOpenAssistant }: { notify: (message: string) => void; onOpenAssistant: () => void }) {
  const [records, setRecords] = useState<SolveRecord[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<'all' | 'draft' | 'finished'>('all');
  const [platform, setPlatform] = useState('');
  const [day, setDay] = useState('');
  const [editing, setEditing] = useState(false);
  const [note, setNote] = useState('');
  const [title, setTitle] = useState('');
  const [saving, setSaving] = useState(false);
  const [companionStatus, setCompanionStatus] = useState<CompanionStatus | null>(null);
  const [companionPort, setCompanionPort] = useState('10046');
  const [companionSaving, setCompanionSaving] = useState(false);
  const importRef = useRef<HTMLInputElement>(null);
  const refresh = useCallback(() => { void migrateLegacyAssistant().then(listSolveRecords).then(setRecords).catch((error) => notify(`读取解题手记失败：${error}`)); }, [notify]);
  useEffect(() => { refresh(); window.addEventListener(JOURNAL_CHANGED, refresh); window.addEventListener('storage', refresh); window.addEventListener('focus', refresh); return () => { window.removeEventListener(JOURNAL_CHANGED, refresh); window.removeEventListener('storage', refresh); window.removeEventListener('focus', refresh); }; }, [refresh]);
  useEffect(() => { if (!isTauri()) return; void api.getCompanionStatus().then((status) => { setCompanionStatus(status); setCompanionPort(String(status.port)); }).catch((error) => notify(`读取 Competitive Companion 状态失败：${error}`)); }, [notify]);
  const selected = records.find((item) => item.id === selectedId) || null;
  const visible = records.filter((item) => (filter === 'all' || item.status === filter) && (!platform || item.platform === platform) && (!day || localDay(item.createdAt) === day) && `${item.title} ${item.url} ${item.platform} ${item.sourceGroup || ''} ${item.tags.join(' ')} ${item.note}`.toLowerCase().includes(query.toLowerCase()));
  const saveCompanionPort = async () => { const port = Number(companionPort); if (!Number.isInteger(port) || port < 1024 || port > 65535) return notify('端口须为 1024–65535 的整数'); setCompanionSaving(true); try { const status = await api.setCompanionPort(port); setCompanionStatus(status); notify(`Competitive Companion 接收端已监听 ${status.port}`); } catch (error) { notify(`设置接收端口失败：${error}`); } finally { setCompanionSaving(false); } };
  const open = (item: SolveRecord) => { setSelectedId(item.id); setTitle(item.title); setNote(item.note); setEditing(false); };
  const saveEdit = async () => { if (!selected) return; setSaving(true); try { await saveSolveRecord({ ...selected, title: title.trim() || '未命名练习', note, tags: [...new Set([...note.matchAll(/(?:^|\s)#([^\s#]+)/gu)].map((match) => match[1]))], updatedAt: Date.now() }); setEditing(false); notify('手记已保存'); } catch (error) { notify(`保存失败：${error}`); } finally { setSaving(false); } };
  const remove = async () => { if (!selected || !window.confirm(`删除“${selected.title || '未命名练习'}”这次记录？此操作无法撤销。`)) return; try { await deleteSolveRecord(selected.id); setSelectedId(null); notify('记录已删除'); } catch (error) { notify(`删除失败：${error}`); } };
  const exportFile = async (kind: 'json' | 'csv' | 'mistakes') => { try { const data = kind === 'json' ? exportSolveJson(records) : kind === 'csv' ? exportSolveCsv(records) : exportMistakesCsv(records); const extension = kind === 'json' ? 'json' : 'csv'; const saved = await saveTrainingFile(`OJ-Insight-${kind === 'mistakes' ? '失误明细' : '解题手记'}-${Date.now()}.${extension}`, data, extension); if (saved) notify(`已导出到 ${saved.path}`); } catch (error) { notify(`导出失败：${error}`); } };
  const importFile = async (file?: File) => {
    if (!file) return;
    try {
      const value = JSON.parse(await file.text()) as { schema?: string; schemaVersion?: number; records?: SolveRecord[] };
      if (value.schema !== 'com.ojinsight.solve-journal' || value.schemaVersion !== 1 || !Array.isArray(value.records)) throw new Error('请选择 OJ Insight 解题手记 JSON');
      if (value.records.length > 10000 || value.records.some((item) => !item || typeof item.id !== 'string' || !['draft', 'finished'].includes(item.status) || !Array.isArray(item.mistakes) || !Array.isArray(item.tags))) throw new Error('文件包含无效记录');
      const existing = new Set(records.map((item) => item.id));
      let count = 0;
      for (const item of value.records) {
        if (existing.has(item.id)) continue;
        await saveSolveRecord(item); count++;
      }
      refresh(); notify(`已导入 ${count} 条；同 ID 的现有记录已保留`);
    } catch (error) { notify(`导入失败：${error}`); } finally { if (importRef.current) importRef.current.value = ''; }
  };
  return <>
    <header className="topbar"><div><small>SOLVE JOURNAL</small><h1>解题手记</h1><p>保存每次练习的用时、思路、标签和失误，随时回看与复盘。</p></div><div className="compact-actions"><button onClick={() => void exportFile('json')} disabled={!records.length}><Download size={14} />导出 JSON</button><button onClick={() => void exportFile('csv')} disabled={!records.length}><Download size={14} />导出 CSV</button><button onClick={() => void exportFile('mistakes')} disabled={!records.some((item) => item.mistakes.length)}><Download size={14} />失误明细 CSV</button><button onClick={() => importRef.current?.click()}><FileUp size={14} />导入 JSON</button><input ref={importRef} hidden type="file" accept="application/json,.json" onChange={(event) => void importFile(event.target.files?.[0])} /></div></header>
    <section className="panel journal-intro"><BookOpenText size={25} /><div><h2>开始一次练习</h2><p>打开悬浮小助手，边做题边计时和记笔记；结束后自动收录到这里。关闭窗口会保留草稿。</p></div><button className="primary" onClick={onOpenAssistant}><Play size={15} />打开做题小助手</button></section>
    <section className="panel journal-companion"><div><small>COMPETITIVE COMPANION</small><h2>浏览器一键收题</h2><p>在扩展设置的 Custom Ports 中添加 OJI 端口。点击扩展后，运行中的 OJI 会自动建立草稿；计时仍由你手动开始。CPH 可同时接收。</p></div>{isTauri() ? <div className="journal-companion-controls"><label>OJI 端口<input aria-label="Competitive Companion 端口" type="number" min="1024" max="65535" value={companionPort} onChange={(event) => setCompanionPort(event.target.value)} /></label><button onClick={() => void saveCompanionPort()} disabled={companionSaving}>{companionSaving ? '设置中…' : companionStatus?.listening ? '应用端口' : '重试监听'}</button><span role="status" className={companionStatus?.listening ? 'ok' : 'error'}>{companionStatus?.listening ? `正在监听 127.0.0.1:${companionStatus.port}` : companionStatus?.error || '检查监听状态…'}</span></div> : <span className="journal-companion-browser">桌面应用安装版可用</span>}</section>
    <div className="journal-layout"><section className="panel journal-list"><div className="journal-filters"><label><Search size={15} /><input aria-label="搜索解题手记" placeholder="搜索题目、标签、笔记" value={query} onChange={(event) => setQuery(event.target.value)} /></label><select aria-label="筛选记录" value={filter} onChange={(event) => setFilter(event.target.value as typeof filter)}><option value="all">全部记录</option><option value="draft">草稿</option><option value="finished">已结束</option></select></div><div className="journal-filters journal-extra-filters"><select aria-label="筛选平台" value={platform} onChange={(event) => setPlatform(event.target.value)}><option value="">全部平台</option>{[...new Set(records.map((item) => item.platform).filter(Boolean))].map((item) => <option key={item} value={item}>{item}</option>)}</select><input aria-label="筛选日期" type="date" value={day} onChange={(event) => setDay(event.target.value)} /></div><div className="journal-count">共 {visible.length} 条记录</div><div className="journal-items">{visible.map((item) => <button key={item.id} className={selected?.id === item.id ? 'active' : ''} onClick={() => open(item)}><strong>{item.title || '未命名练习'}</strong>{item.sourceGroup && <small>{item.sourceGroup}</small>}<span>{item.status === 'draft' ? '草稿' : item.outcome === 'solved' ? '已完成' : '待复习'} · {duration(item.elapsedMs)} · {date(item.updatedAt)}</span>{item.tags.length > 0 && <small>{item.tags.map((tag) => `#${tag}`).join(' ')}</small>}</button>)}{!visible.length && <p className="empty">暂无匹配的记录</p>}</div></section>
      <section className="panel journal-detail">{selected ? <><header><div><small>{selected.status === 'draft' ? '未结束草稿' : '已归档记录'}</small><h2>{selected.title || '未命名练习'}</h2></div><div>{selected.status === 'draft' && <button onClick={() => { selectSolveDraft(selected.id); onOpenAssistant(); }}>继续草稿</button>}<button onClick={() => { setEditing(!editing); setTitle(selected.title); setNote(selected.note); }}>{editing ? '取消编辑' : '编辑'}</button><button className="danger" onClick={() => void remove()} title="删除记录"><Trash2 size={15} /></button></div></header>{editing ? <div className="journal-editor"><input aria-label="编辑题目名称" value={title} onChange={(event) => setTitle(event.target.value)} /><textarea aria-label="编辑手记" value={note} onChange={(event) => setNote(event.target.value)} /><button className="primary" disabled={saving} onClick={() => void saveEdit()}>{saving ? '保存中…' : '保存修改'}</button></div> : <><div className="journal-facts">{selected.sourceGroup && <span>来源：{selected.sourceGroup}</span>}<span>开始：{date(selected.createdAt)}</span><span>结束：{date(selected.endedAt)}</span><span>用时：{duration(selected.elapsedMs)}</span><span>失误：{selected.mistakes.length} 次</span>{/^https?:\/\//i.test(selected.url) && <a href={selected.url} target="_blank" rel="noreferrer">打开原题</a>}</div>{selected.tags.length > 0 && <div className="journal-tags">{selected.tags.map((tag) => <span key={tag}>#{tag}</span>)}</div>}<div className="journal-note"><h3>做题笔记</h3><MarkdownPreview text={selected.note || '暂无笔记'} /></div><div className="journal-mistakes"><h3>失误记录</h3>{selected.mistakes.length ? selected.mistakes.map((item) => <div key={item.id}><strong>{item.reason}</strong><span>{date(item.at)}{item.lostMinutes != null ? ` · 约 ${item.lostMinutes} 分钟` : ''}</span>{item.note && <p>{item.note}</p>}</div>) : <p>暂无失误记录</p>}</div></>}</> : <div className="empty">选择一条记录查看详情</div>}</section></div>
  </>;
}
