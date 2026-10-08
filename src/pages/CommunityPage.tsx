import { useEffect, useMemo, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import { Download, ExternalLink, Folder, LibraryBig, RefreshCw, Search } from 'lucide-react';

import PlatformIcon from '../components/PlatformIcon';
import { MarkdownPreview } from '../components/MarkdownNote';
import { PLATFORM_META } from '../lib/platforms';
import { api } from '../services/api';
import type { CommunityCatalog, CommunityEntry, CommunityListing } from '../types';

const REPOSITORY = 'https://github.com/Whalica/OJ_Insight-Community';
const LICENSE_LABELS: Record<string, string> = { 'CC-BY-4.0': '转载或改编时需署名', 'CC-BY-SA-4.0': '需署名，改编后沿用相同许可', 'CC0-1.0': '作者尽可能放弃权利限制' };
const CONTENT_ROOT = 'content/problem-sets/';
interface CommunityFolder { name: string; path: string; count: number; folders: Map<string, CommunityFolder>; entries: CommunityListing[] }

function folderTree(entries: CommunityListing[]): CommunityFolder {
  const root: CommunityFolder = { name: '', path: '', count: 0, folders: new Map(), entries: [] };
  for (const entry of entries) {
    const segments = entry.path.slice(CONTENT_ROOT.length).split('/').slice(0, -1);
    let current = root;
    current.count += 1;
    for (const name of segments) {
      if (!current.folders.has(name)) current.folders.set(name, { name, path: `${current.path}${name}/`, count: 0, folders: new Map(), entries: [] });
      current = current.folders.get(name)!;
      current.count += 1;
    }
    current.entries.push(entry);
  }
  return root;
}

export default function CommunityPage({ notify, onOpenLocalSets, embedded = false }: { notify: (message: string) => void; onOpenLocalSets: () => void; embedded?: boolean }) {
  const [catalog, setCatalog] = useState<CommunityCatalog | null>(null);
  const [entry, setEntry] = useState<CommunityEntry | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [entryLoading, setEntryLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [openFolders, setOpenFolders] = useState<Set<string>>(() => new Set());
  const entryRequest = useRef(0);

  const refresh = async (fresh = false) => {
    setLoading(true); setError('');
    try { setCatalog(await api.getCommunityCatalog(fresh)); }
    catch (reason) { setError(String(reason)); }
    finally { setLoading(false); }
  };
  useEffect(() => { void refresh(true); }, []);
  const visible = useMemo(() => (catalog?.entries || []).filter((item) => item.type === 'problem-set' && `${item.title} ${item.summary} ${item.author.name} ${item.categories.join(' ')} ${item.path.slice(CONTENT_ROOT.length).split('/').slice(0, -1).join(' ')}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())), [catalog, query]);
  const folders = useMemo(() => folderTree(visible), [visible]);
  const selected = catalog?.entries.find((item) => item.id === selectedId) || null;

  const open = async (item: CommunityListing) => {
    const request = ++entryRequest.current;
    setSelectedId(item.id); setEntry(null); setEntryLoading(true);
    try { const result = await api.getCommunityProblemSet(item.id, item.path); if (request === entryRequest.current) setEntry(result); }
    catch (reason) { if (request === entryRequest.current) notify(`读取推荐题单失败：${String(reason)}`); }
    finally { if (request === entryRequest.current) setEntryLoading(false); }
  };
  const save = async () => {
    if (!entry || !selected || entry.id !== selected.id || saving) return;
    setSaving(true);
    try { const saved = await api.saveCommunityProblemSet(entry, selected.path); notify(`“${saved.title}”已保存到本地题单`); onOpenLocalSets(); }
    catch (reason) { notify(`保存题单失败：${String(reason)}`); }
    finally { setSaving(false); }
  };
  const renderEntry = (item: CommunityListing) => <article key={item.id} className={selectedId === item.id ? 'active' : ''}><button className="set-card-main" onClick={() => void open(item)}><strong>{item.title}</strong><span>{item.problemCount} 道题 · {item.author.name}</span><small>{item.categories.join(' · ') || '未分类'}</small></button></article>;
  const renderFolder = (folder: CommunityFolder): ReactNode => <details key={folder.path} className="community-folder" open={!!query || openFolders.has(folder.path)}><summary onClick={(event) => { event.preventDefault(); setOpenFolders((current) => { const next = new Set(current); if (next.has(folder.path)) next.delete(folder.path); else next.add(folder.path); return next; }); }}><Folder size={15} /><span>{folder.name}</span><small>{folder.count} 份</small></summary><div className="community-folder-items">{folder.entries.map(renderEntry)}{[...folder.folders.values()].sort((a, b) => a.name.localeCompare(b.name)).map(renderFolder)}</div></details>;

  return <>
    {!embedded && <header className="topbar"><div><small lang="en">TRAINING CENTER · COMMUNITY</small><h1>推荐题单</h1><p>浏览经审核的社区题单，预览后保存为可编辑的本地副本。</p></div><div className="compact-actions"><button onClick={() => void refresh(true)} disabled={loading}><RefreshCw size={15} className={loading ? 'spin' : ''} />刷新</button><button onClick={() => void api.openExternal(REPOSITORY)}><ExternalLink size={15} />投稿与审核</button></div></header>}
    {embedded && <div className="data-tab-head"><p>浏览经审核的社区题单，预览后保存为可编辑的本地副本。</p><div className="compact-actions"><button onClick={() => void refresh(true)} disabled={loading}><RefreshCw size={15} className={loading ? 'spin' : ''} />刷新</button><button onClick={() => void api.openExternal(REPOSITORY)}><ExternalLink size={15} />投稿与审核</button></div></div>}
    {error && <section className="panel community-notice" role="alert">社区目录暂不可用：{error}<button onClick={() => void refresh(true)}>重试</button></section>}
    {catalog?.cached && <section className="panel community-notice" role="status">当前显示上次保存的社区目录，可能不是最新内容。<button onClick={() => void refresh(true)}>重试</button></section>}
    {entry?.cached && <section className="panel community-notice" role="status">当前题单来自本地缓存，保存前请留意内容可能已更新。</section>}
    <div className="problem-sets-layout community-layout">
      <aside className="panel set-gallery community-gallery"><header><div><strong>社区题单</strong><small>{loading ? '读取中…' : `${visible.length} 份`}</small></div></header><label className="community-search"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索题单、作者或分类" /></label>
        {folders.entries.map(renderEntry)}{[...folders.folders.values()].sort((a, b) => a.name.localeCompare(b.name)).map(renderFolder)}
        {!loading && !visible.length && <div className="set-empty"><strong>{catalog?.entries.length ? '没有匹配的题单' : '社区题单征集中'}</strong><span>{catalog?.entries.length ? '换个关键词试试。' : '审核通过的题单会在这里出现。'}</span></div>}
      </aside>
      <section className="panel set-detail community-detail">
        {entryLoading ? <div className="community-placeholder">正在读取题单…</div> : entry && selected ? <>
          <header><div><small>社区题单 · {entry.content.problems.length} 题 · 已通过 {entry.solvedKeys.length} 题</small><h2>{entry.title}</h2><p>{entry.summary}</p></div><button className="primary community-save" onClick={() => void save()} disabled={saving}><Download size={15} />{saving ? '保存中…' : '保存到本地'}</button></header>
          <div className="community-meta"><span>作者：{entry.author.name}</span><span title="作者对题单说明和原创笔记的分享规则；题目仍归原平台或原作者。">分享许可：{LICENSE_LABELS[entry.license] || entry.license}（{entry.license}）</span>{entry.categories.map((category) => <span key={category}>{category}</span>)}</div>
          {entry.content.description && <div className="set-description-preview"><MarkdownPreview text={entry.content.description} /></div>}
          <div className="set-problem-list">{entry.content.problems.map((row, index) => { const passed = entry.solvedKeys.includes(`${row.problem.platform}:${row.problem.problemKey}`); return <a key={`${row.problem.platform}:${row.problem.problemKey}`} href={row.problem.url} target="_blank" rel="noreferrer"><PlatformIcon platform={row.problem.platform} /><span className="problem-number">{index + 1}</span><div><strong>{row.problem.name || row.problem.problemId || row.problem.problemKey}</strong><small>{PLATFORM_META[row.problem.platform]?.name || row.problem.platform}{row.note ? ` · ${row.note}` : ''}</small>{(entry.content.tagVisibility === 'before_solving' || entry.content.tagVisibility === 'after_ac' && passed) && row.problem.tags?.length > 0 && <em>{row.problem.tags.join(' · ')}</em>}</div><span className={`set-solve-badge ${passed ? 'passed' : ''}`}>{passed ? '已通过' : '未记录通过'}</span></a>; })}</div>
          <footer><span>保存后可在「题单」编辑；社区更新不会覆盖本地副本。</span><button className="primary" onClick={() => void save()} disabled={saving}><Download size={15} />保存到本地</button></footer>
        </> : <div className="community-placeholder"><LibraryBig size={32} /><strong>选择一份题单查看</strong><span>从左侧打开题单，确认内容后再保存到本地。</span></div>}
      </section>
    </div>
  </>;
}
