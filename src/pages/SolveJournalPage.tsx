import { useCallback, useEffect, useRef, useState } from 'react';
import { isTauri } from '@tauri-apps/api/core';
import { BookOpenText, Download, FileUp, Play, Search, Trash2 } from 'lucide-react';
import { MarkdownPreview } from '../components/MarkdownNote';
import { JOURNAL_CHANGED, deleteSolveRecord, exportMistakesCsv, exportSolveCsv, exportSolveJson, listSolveRecords, migrateLegacyAssistant, saveSolveRecord, selectSolveDraft, type SolveRecord } from '../services/solveJournal';
import { saveTrainingFile } from '../services/training';
import { api, type CompanionStatus } from '../services/api';
import { currentLocale, useI18n } from '../lib/i18n';

function date(value: number | null) { return value == null ? '—' : new Date(value).toLocaleString(currentLocale()); }
function localDay(value: number) { const day = new Date(value); return `${day.getFullYear()}-${String(day.getMonth() + 1).padStart(2, '0')}-${String(day.getDate()).padStart(2, '0')}`; }
function timerReading(ms: number) { const seconds = Math.floor(Math.max(0, ms) / 1000); return [Math.floor(seconds / 3600), Math.floor(seconds / 60) % 60, seconds % 60].map((part) => String(part).padStart(2, '0')).join(':'); }

export default function SolveJournalPage({ notify, onOpenAssistant }: { notify: (message: string) => void; onOpenAssistant: () => void }) {
  const { t } = useI18n();
  const tRef = useRef(t);
  tRef.current = t;
  const duration = (ms: number) => t('{minutes} 分钟', { minutes: Math.round(ms / 6000) / 10 });
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
  const refresh = useCallback(() => { void migrateLegacyAssistant().then(listSolveRecords).then(setRecords).catch((error) => notify(tRef.current('读取解题手记失败：{error}', { error: String(error) }))); }, [notify]);
  useEffect(() => { refresh(); window.addEventListener(JOURNAL_CHANGED, refresh); window.addEventListener('storage', refresh); window.addEventListener('focus', refresh); return () => { window.removeEventListener(JOURNAL_CHANGED, refresh); window.removeEventListener('storage', refresh); window.removeEventListener('focus', refresh); }; }, [refresh]);
  useEffect(() => { if (!isTauri()) return; void api.getCompanionStatus().then((status) => { setCompanionStatus(status); setCompanionPort(String(status.port)); }).catch((error) => notify(tRef.current('读取 Competitive Companion 状态失败：{error}', { error: String(error) }))); }, [notify]);
  const selected = records.find((item) => item.id === selectedId) || null;
  const visible = records.filter((item) => (filter === 'all' || item.status === filter) && (!platform || item.platform === platform) && (!day || localDay(item.createdAt) === day) && `${item.title} ${item.url} ${item.platform} ${item.sourceGroup || ''} ${item.tags.join(' ')} ${item.note}`.toLowerCase().includes(query.toLowerCase()));
  const saveCompanionPort = async () => { const port = Number(companionPort); if (!Number.isInteger(port) || port < 1024 || port > 65535) return notify(t('端口须为 1024–65535 的整数')); setCompanionSaving(true); try { const status = await api.setCompanionPort(port); setCompanionStatus(status); notify(t('Competitive Companion 接收端已监听 {port}', { port: status.port })); } catch (error) { notify(t('设置接收端口失败：{error}', { error: String(error) })); } finally { setCompanionSaving(false); } };
  const open = (item: SolveRecord) => { setSelectedId(item.id); setTitle(item.title); setNote(item.note); setEditing(false); };
  const saveEdit = async () => { if (!selected) return; setSaving(true); try { await saveSolveRecord({ ...selected, title: title.trim() || '未命名练习', note, tags: [...new Set([...note.matchAll(/(?:^|\s)#([^\s#]+)/gu)].map((match) => match[1]))], updatedAt: Date.now() }); setEditing(false); notify(t('手记已保存')); } catch (error) { notify(t('保存失败：{error}', { error: String(error) })); } finally { setSaving(false); } };
  const remove = async () => { if (!selected || !window.confirm(t('删除“{title}”这次记录？此操作无法撤销。', { title: selected.title || t('未命名练习') }))) return; try { await deleteSolveRecord(selected.id); setSelectedId(null); notify(t('记录已删除')); } catch (error) { notify(t('删除失败：{error}', { error: String(error) })); } };
  const exportFile = async (kind: 'json' | 'csv' | 'mistakes') => { try { const data = kind === 'json' ? exportSolveJson(records) : kind === 'csv' ? exportSolveCsv(records) : exportMistakesCsv(records); const extension = kind === 'json' ? 'json' : 'csv'; const saved = await saveTrainingFile(`OJ-Insight-${kind === 'mistakes' ? '失误明细' : '解题手记'}-${Date.now()}.${extension}`, data, extension); if (saved) notify(t('已导出到 {path}', { path: saved.path })); } catch (error) { notify(t('导出失败：{error}', { error: String(error) })); } };
  const importFile = async (file?: File) => {
    if (!file) return;
    try {
      const value = JSON.parse(await file.text()) as { schema?: string; schemaVersion?: number; records?: SolveRecord[] };
      if (value.schema !== 'com.ojinsight.solve-journal' || value.schemaVersion !== 1 || !Array.isArray(value.records)) throw new Error(t('请选择 OJ Insight 解题手记 JSON'));
      if (value.records.length > 10000 || value.records.some((item) => !item || typeof item.id !== 'string' || !['draft', 'finished'].includes(item.status) || !Array.isArray(item.mistakes) || !Array.isArray(item.tags))) throw new Error(t('文件包含无效记录'));
      const existing = new Set(records.map((item) => item.id));
      let count = 0;
      for (const item of value.records) {
        if (existing.has(item.id)) continue;
        await saveSolveRecord(item); count++;
      }
      refresh(); notify(t('已导入 {count} 条；同 ID 的现有记录已保留', { count }));
    } catch (error) { notify(t('导入失败：{error}', { error: String(error) })); } finally { if (importRef.current) importRef.current.value = ''; }
  };
  return <>
    <header className="topbar"><div><small lang="en">SOLVE JOURNAL</small><h1>{t('解题手记')}</h1><p>{t('保存每次练习的用时、思路、标签和失误，随时回看与复盘。')}</p></div><div className="compact-actions"><button onClick={() => void exportFile('json')} disabled={!records.length}><Download size={14} />{t('导出 JSON')}</button><button onClick={() => void exportFile('csv')} disabled={!records.length}><Download size={14} />{t('导出 CSV')}</button><button onClick={() => void exportFile('mistakes')} disabled={!records.some((item) => item.mistakes.length)}><Download size={14} />{t('失误明细 CSV')}</button><button onClick={() => importRef.current?.click()}><FileUp size={14} />{t('导入 JSON')}</button><input ref={importRef} hidden type="file" accept="application/json,.json" onChange={(event) => void importFile(event.target.files?.[0])} /></div></header>
    <section className="panel journal-intro"><BookOpenText size={25} /><div><h2>{t('开始一次练习')}</h2><p>{t('打开悬浮小助手，边做题边计时和记笔记；结束后自动收录到这里。关闭窗口会保留草稿。')}</p></div><button className="primary" onClick={onOpenAssistant}><Play size={15} />{t('打开做题小助手')}</button></section>
    <section className="panel journal-companion"><div><small lang="en">COMPETITIVE COMPANION</small><h2>{t('浏览器一键收题')}</h2><p>{t('在扩展设置的 Custom Ports 中添加 OJI 端口。点击扩展后，运行中的 OJI 会自动建立草稿；计时仍由你手动开始。CPH 可同时接收。')}</p></div>{isTauri() ? <div className="journal-companion-controls"><label>{t('OJI 端口')}<input aria-label={t('Competitive Companion 端口')} type="number" min="1024" max="65535" value={companionPort} onChange={(event) => setCompanionPort(event.target.value)} /></label><button onClick={() => void saveCompanionPort()} disabled={companionSaving}>{companionSaving ? t('设置中…') : companionStatus?.listening ? t('应用端口') : t('重试监听')}</button><span role="status" className={companionStatus?.listening ? 'ok' : 'error'}>{companionStatus?.listening ? t('正在监听 127.0.0.1:{port}', { port: companionStatus.port }) : companionStatus?.error || t('检查监听状态…')}</span></div> : <span className="journal-companion-browser">{t('桌面应用安装版可用')}</span>}</section>
    <div className="journal-layout"><section className="panel journal-list"><div className="journal-filters"><label><Search size={15} /><input aria-label={t('搜索解题手记')} placeholder={t('搜索题目、标签、笔记')} value={query} onChange={(event) => setQuery(event.target.value)} /></label><select aria-label={t('筛选记录')} value={filter} onChange={(event) => setFilter(event.target.value as typeof filter)}><option value="all">{t('全部记录')}</option><option value="draft">{t('草稿')}</option><option value="finished">{t('已结束')}</option></select></div><div className="journal-filters journal-extra-filters"><select aria-label={t('筛选平台')} value={platform} onChange={(event) => setPlatform(event.target.value)}><option value="">{t('全部平台')}</option>{[...new Set(records.map((item) => item.platform).filter(Boolean))].map((item) => <option key={item} value={item}>{item}</option>)}</select><input aria-label={t('筛选日期')} type="date" value={day} onChange={(event) => setDay(event.target.value)} /></div><div className="journal-count">{t('共 {count} 条记录', { count: visible.length })}</div><div className="journal-items">{visible.map((item) => <button key={item.id} className={selected?.id === item.id ? 'active' : ''} onClick={() => open(item)}><strong>{item.title || t('未命名练习')}</strong>{item.sourceGroup && <small>{item.sourceGroup}</small>}<span>{item.status === 'draft' ? t('草稿') : item.outcome === 'solved' ? t('已完成') : t('待复习')} · {duration(item.elapsedMs)} · {date(item.updatedAt)}</span>{item.tags.length > 0 && <small>{item.tags.map((tag) => `#${tag}`).join(' ')}</small>}</button>)}{!visible.length && <p className="empty">{t('暂无匹配的记录')}</p>}</div></section>
      <section className="panel journal-detail">{selected ? <><header><div><small>{selected.status === 'draft' ? t('未结束草稿') : t('已归档记录')}</small><h2>{selected.title || t('未命名练习')}</h2></div><div>{selected.status === 'draft' && <button onClick={() => { selectSolveDraft(selected.id); onOpenAssistant(); }}>{t('继续草稿')}</button>}<button onClick={() => { setEditing(!editing); setTitle(selected.title); setNote(selected.note); }}>{editing ? t('取消编辑') : t('编辑')}</button><button className="danger" onClick={() => void remove()} title={t('删除记录')}><Trash2 size={15} /></button></div></header>{editing ? <div className="journal-editor"><input aria-label={t('编辑题目名称')} value={title} onChange={(event) => setTitle(event.target.value)} /><textarea aria-label={t('编辑手记')} value={note} onChange={(event) => setNote(event.target.value)} /><button className="primary" disabled={saving} onClick={() => void saveEdit()}>{saving ? t('保存中…') : t('保存修改')}</button></div> : <><div className="journal-facts">{selected.sourceGroup && <span>{t('来源：{group}', { group: selected.sourceGroup })}</span>}<span>{t('开始：{date}', { date: date(selected.createdAt) })}</span><span>{t('结束：{date}', { date: date(selected.endedAt) })}</span><span>{t('用时：{duration}', { duration: duration(selected.elapsedMs) })}</span><span>{t('失误：{count} 次', { count: selected.mistakes.length })}</span>{/^https?:\/\//i.test(selected.url) && <a href={selected.url} target="_blank" rel="noreferrer">{t('打开原题')}</a>}</div>{selected.tags.length > 0 && <div className="journal-tags">{selected.tags.map((tag) => <span key={tag}>#{tag}</span>)}</div>}<div className="journal-note"><h3>{t('做题笔记')}</h3><MarkdownPreview text={selected.note || t('暂无笔记')} /></div><div className="journal-mistakes"><h3>{t('失误记录')}</h3>{selected.mistakes.length ? selected.mistakes.map((item) => <div key={item.id}><strong>{t(item.reason)}</strong><span>{date(item.at)}{item.elapsedMs != null ? ` · ${t('计时 {elapsed}', { elapsed: timerReading(item.elapsedMs) })}` : ''}</span>{item.note && <p>{item.note}</p>}</div>) : <p>{t('暂无失误记录')}</p>}</div></>}</> : <div className="empty">{t('选择一条记录查看详情')}</div>}</section></div>
  </>;
}
