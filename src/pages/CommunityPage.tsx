import { useEffect, useMemo, useState } from 'react';
import { Download, ExternalLink, LibraryBig, RefreshCw, Search } from 'lucide-react';

import PlatformIcon from '../components/PlatformIcon';
import { MarkdownPreview } from '../components/MarkdownNote';
import { PLATFORM_META } from '../lib/platforms';
import { api } from '../services/api';
import type { CommunityCatalog, CommunityEntry } from '../types';

const REPOSITORY = 'https://github.com/Whalica/OJ_Insight-Community';

export default function CommunityPage({ notify, onOpenLocalSets }: { notify: (message: string) => void; onOpenLocalSets: () => void }) {
  const [catalog, setCatalog] = useState<CommunityCatalog | null>(null);
  const [entry, setEntry] = useState<CommunityEntry | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(false);
  const [entryLoading, setEntryLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const refresh = async () => {
    setLoading(true); setError('');
    try { setCatalog(await api.getCommunityCatalog()); }
    catch (reason) { setError(String(reason)); }
    finally { setLoading(false); }
  };
  useEffect(() => { void refresh(); }, []);
  const visible = useMemo(() => (catalog?.entries || []).filter((item) => item.type === 'problem-set' && `${item.title} ${item.summary} ${item.author.name} ${item.categories.join(' ')}`.toLocaleLowerCase().includes(query.toLocaleLowerCase())), [catalog, query]);
  const selected = catalog?.entries.find((item) => item.id === selectedId) || null;

  const open = async (id: string) => {
    setSelectedId(id); setEntry(null); setEntryLoading(true);
    try { setEntry(await api.getCommunityProblemSet(id)); }
    catch (reason) { notify(`读取推荐题单失败：${String(reason)}`); }
    finally { setEntryLoading(false); }
  };
  const save = async () => {
    if (!entry || saving) return;
    setSaving(true);
    try { const saved = await api.saveCommunityProblemSet(entry); notify(`“${saved.title}”已保存到本地题单`); onOpenLocalSets(); }
    catch (reason) { notify(`保存题单失败：${String(reason)}`); }
    finally { setSaving(false); }
  };

  return <>
    <header className="topbar"><div><small>TRAINING CENTER · COMMUNITY</small><h1>推荐题单</h1><p>浏览经审核的社区题单，预览后保存为可编辑的本地副本。</p></div><div className="compact-actions"><button onClick={() => void refresh()} disabled={loading}><RefreshCw size={15} className={loading ? 'spin' : ''} />刷新</button><button onClick={() => void api.openExternal(REPOSITORY)}><ExternalLink size={15} />投稿与审核</button></div></header>
    {error && <section className="panel community-notice" role="alert">社区目录暂不可用：{error}<button onClick={() => void refresh()}>重试</button></section>}
    <div className="problem-sets-layout community-layout">
      <aside className="panel set-gallery community-gallery"><header><div><strong>社区题单</strong><small>{loading ? '读取中…' : `${visible.length} 份`}</small></div></header><label className="community-search"><Search size={15} /><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索题单、作者或分类" /></label>
        {visible.map((item) => <article key={item.id} className={selectedId === item.id ? 'active' : ''}><button className="set-card-main" onClick={() => void open(item.id)}><strong>{item.title}</strong><span>{item.problemCount} 道题 · {item.author.name}</span><small>{item.categories.join(' · ') || '未分类'}</small></button></article>)}
        {!loading && !visible.length && <div className="set-empty"><strong>{catalog?.entries.length ? '没有匹配的题单' : '社区题单征集中'}</strong><span>{catalog?.entries.length ? '换个关键词试试。' : '审核通过的题单会在这里出现。'}</span></div>}
      </aside>
      <section className="panel set-detail community-detail">
        {entryLoading ? <div className="community-placeholder">正在读取题单…</div> : entry && selected ? <>
          <header><div><small>社区题单 · {entry.content.problems.length} 题</small><h2>{entry.title}</h2><p>{entry.summary}</p></div><button className="primary community-save" onClick={() => void save()} disabled={saving}><Download size={15} />{saving ? '保存中…' : '保存到本地'}</button></header>
          <div className="community-meta"><span>作者：{entry.author.name}</span><span>授权：{entry.license}</span>{entry.categories.map((category) => <span key={category}>{category}</span>)}</div>
          {entry.content.description && <div className="set-description-preview"><MarkdownPreview text={entry.content.description} /></div>}
          <div className="set-problem-list">{entry.content.problems.map((row, index) => <a key={`${row.problem.platform}:${row.problem.problemKey}`} href={row.problem.url} target="_blank" rel="noreferrer"><PlatformIcon platform={row.problem.platform} /><span className="problem-number">{index + 1}</span><div><strong>{row.problem.name || row.problem.problemId || row.problem.problemKey}</strong><small>{PLATFORM_META[row.problem.platform]?.name || row.problem.platform}{row.note ? ` · ${row.note}` : ''}</small>{entry.content.tagVisibility === 'before_solving' && row.problem.tags?.length > 0 && <em>{row.problem.tags.join(' · ')}</em>}</div><ExternalLink size={14} /></a>)}</div>
          <footer><span>保存后可在「题单」编辑；社区更新不会覆盖本地副本。</span><button className="primary" onClick={() => void save()} disabled={saving}><Download size={15} />保存到本地</button></footer>
        </> : <div className="community-placeholder"><LibraryBig size={32} /><strong>选择一份题单查看</strong><span>从左侧打开题单，确认内容后再保存到本地。</span></div>}
      </section>
    </div>
  </>;
}
