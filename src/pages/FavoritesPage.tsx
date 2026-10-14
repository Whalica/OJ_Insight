import { useEffect, useMemo, useState } from 'react';
import { Bookmark, ExternalLink, FolderPlus, Pencil, Pin, Plus, Search, Trash2, X } from 'lucide-react';

import { MarkdownPreview } from '../components/MarkdownNote';
import { api } from '../services/api';
import type { FavoriteCategory, FavoriteInput, FavoriteItem, FavoriteKind } from '../types';

const kinds: Record<FavoriteKind, string> = { problem: '题目', problem_set: '题单', article: '博客 / 文章', resource: '其他资源' };
const blank = (): FavoriteInput => ({ id: null, kind: 'problem', title: '', url: '', summary: '', note: '', categoryId: null, pinned: false });

export default function FavoritesPage({ notify }: { notify: (message: string) => void }) {
  const [items, setItems] = useState<FavoriteItem[]>([]);
  const [categories, setCategories] = useState<FavoriteCategory[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [category, setCategory] = useState<number | 'all' | 'none'>('all');
  const [kind, setKind] = useState<FavoriteKind | 'all'>('all');
  const [query, setQuery] = useState('');
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState<FavoriteInput>(blank);
  const [busy, setBusy] = useState(false);
  const [compact, setCompact] = useState(() => localStorage.getItem('oj-insight.favorites.compact') === 'true');
  const selected = items.find((item) => item.id === selectedId) || null;

  const reload = async () => {
    const [nextItems, nextCategories] = await Promise.all([api.listFavoriteItems(), api.listFavoriteCategories()]);
    setItems(nextItems); setCategories(nextCategories);
  };
  useEffect(() => { void reload().catch((error) => notify(`读取收藏夹失败：${String(error)}`)); }, []);

  const visible = useMemo(() => items.filter((item) => {
    if (category === 'none' && item.categoryId != null) return false;
    if (typeof category === 'number' && item.categoryId !== category) return false;
    if (kind !== 'all' && item.kind !== kind) return false;
    const text = `${item.title} ${item.url} ${item.summary} ${item.note}`.toLocaleLowerCase();
    return text.includes(query.trim().toLocaleLowerCase());
  }), [items, category, kind, query]);

  const startNew = () => { setDraft(blank()); setEditing(true); setSelectedId(null); };
  const startEdit = (item: FavoriteItem) => { setDraft({ id: item.id, kind: item.kind, title: item.title, url: item.url, summary: item.summary, note: item.note, categoryId: item.categoryId, pinned: item.pinned }); setSelectedId(item.id); setEditing(true); };
  const save = async () => {
    if (!draft.title.trim() || !/^https?:\/\/\S+$/i.test(draft.url.trim())) { notify('请填写显示名称和完整的 http(s) 链接'); return; }
    setBusy(true);
    try {
      const saved = await api.saveFavoriteItem(draft);
      await reload(); setSelectedId(saved.id); setEditing(false);
      notify(draft.id ? '收藏已更新' : '链接已收藏');
    } catch (error) { notify(`保存失败：${String(error)}`); }
    finally { setBusy(false); }
  };
  const remove = async (item: FavoriteItem) => {
    if (!window.confirm(`删除收藏“${item.title}”？原网页不会受影响。`)) return;
    try { await api.deleteFavoriteItem(item.id); await reload(); if (selectedId === item.id) { setSelectedId(null); setEditing(false); } notify('已删除收藏'); }
    catch (error) { notify(`删除失败：${String(error)}`); }
  };
  const addCategory = async () => {
    const name = window.prompt('分类名称（最多 40 字）')?.trim();
    if (!name) return;
    try { const saved = await api.saveFavoriteCategory(null, name); await reload(); setCategory(saved.id); }
    catch (error) { notify(`创建分类失败：${String(error)}`); }
  };
  const renameCategory = async (item: FavoriteCategory) => {
    const name = window.prompt('修改分类名称', item.name)?.trim();
    if (!name || name === item.name) return;
    try { await api.saveFavoriteCategory(item.id, name); await reload(); }
    catch (error) { notify(`修改分类失败：${String(error)}`); }
  };
  const removeCategory = async (item: FavoriteCategory) => {
    if (!window.confirm(`删除分类“${item.name}”？其中的链接会移到未分类。`)) return;
    try { await api.deleteFavoriteCategory(item.id); await reload(); if (category === item.id) setCategory('none'); notify('分类已删除，链接已移到未分类'); }
    catch (error) { notify(`删除分类失败：${String(error)}`); }
  };
  const togglePin = async (item: FavoriteItem) => {
    try { await api.saveFavoriteItem({ ...item, pinned: !item.pinned }); await reload(); }
    catch (error) { notify(`置顶失败：${String(error)}`); }
  };
  const open = (item: FavoriteItem) => { void api.openFavoriteLink(item.id).catch((error) => notify(`打开链接失败：${String(error)}`)); };

  return <>
    <header className="topbar"><div><small lang="en">TRAINING CENTER · FAVORITES</small><h1>收藏夹</h1><p>按用途整理题目、题单、博客等链接。收藏只保存入口和你的备注，不复制原内容。</p></div><div className="compact-actions"><button onClick={() => { const next = !compact; setCompact(next); localStorage.setItem('oj-insight.favorites.compact', String(next)); }}>{compact ? '显示简介' : '紧凑显示'}</button><button className="primary" onClick={startNew}><Plus size={15} />添加链接</button></div></header>
    <div className="favorites-layout">
      <aside className="panel favorites-categories"><header><strong>分类</strong><button title="新建分类" onClick={() => void addCategory()}><FolderPlus size={16} /></button></header><button className={category === 'all' ? 'active' : ''} onClick={() => setCategory('all')}>全部 <span>{items.length}</span></button><button className={category === 'none' ? 'active' : ''} onClick={() => setCategory('none')}>未分类 <span>{items.filter((item) => item.categoryId == null).length}</span></button>{categories.map((item) => <div key={item.id} className={`favorites-category ${category === item.id ? 'active' : ''}`}><button onClick={() => setCategory(item.id)}>{item.name} <span>{items.filter((entry) => entry.categoryId === item.id).length}</span></button><button title="重命名分类" onClick={() => void renameCategory(item)}><Pencil size={13} /></button><button title="删除分类" onClick={() => void removeCategory(item)}><Trash2 size={13} /></button></div>)}</aside>
      <section className="panel favorites-list"><div className="favorites-filters"><label><Search size={15} /><input aria-label="搜索收藏" placeholder="搜索名称、链接、简介、备注" value={query} onChange={(event) => setQuery(event.target.value)} /></label><select aria-label="筛选类型" value={kind} onChange={(event) => setKind(event.target.value as FavoriteKind | 'all')}><option value="all">全部类型</option>{Object.entries(kinds).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></div><small className="favorites-count">{visible.length} 条链接</small><div className="favorites-items">{visible.map((item) => <article key={item.id} className={selectedId === item.id && !editing ? 'active' : ''}><button className="favorites-item-main" onClick={() => { setSelectedId(item.id); setEditing(false); }}><div><Bookmark size={15} /><strong>{item.title}</strong>{item.pinned && <Pin size={13} />}</div><small>{kinds[item.kind]} · {categories.find((entry) => entry.id === item.categoryId)?.name || '未分类'}</small>{!compact && item.summary && <p>{item.summary}</p>}<span title={item.url}>{item.url}</span></button><div className="favorites-item-actions"><button title="打开链接" onClick={() => open(item)}><ExternalLink size={15} /></button><button title="编辑" onClick={() => startEdit(item)}><Pencil size={15} /></button></div></article>)}{!visible.length && <p className="empty">暂无匹配链接。可粘贴一个题目、题单或博客网址开始整理。</p>}</div></section>
      <section className="panel favorites-detail">{editing ? <><header><div><small>{draft.id ? 'EDIT LINK' : 'NEW LINK'}</small><h2>{draft.id ? '编辑收藏' : '添加链接'}</h2></div><button title="关闭编辑" onClick={() => setEditing(false)}><X size={16} /></button></header><div className="favorites-form"><label>链接类型<select value={draft.kind} onChange={(event) => setDraft({ ...draft, kind: event.target.value as FavoriteKind })}>{Object.entries(kinds).map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label><label>链接地址<input value={draft.url} onChange={(event) => setDraft({ ...draft, url: event.target.value })} placeholder="https://..." /></label><label>显示名称<input value={draft.title} maxLength={200} onChange={(event) => setDraft({ ...draft, title: event.target.value })} placeholder="例如：二分答案专题" /></label><label>分类<select value={draft.categoryId ?? ''} onChange={(event) => setDraft({ ...draft, categoryId: event.target.value ? Number(event.target.value) : null })}><option value="">未分类</option>{categories.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select></label><label>简介<textarea value={draft.summary} maxLength={1000} onChange={(event) => setDraft({ ...draft, summary: event.target.value })} placeholder="这一条链接有什么用？" /></label><label>个人备注（支持 Markdown）<textarea className="favorites-note-input" value={draft.note} maxLength={20000} onChange={(event) => setDraft({ ...draft, note: event.target.value })} placeholder="想法、使用建议、待复习的点…" /></label><label className="favorites-check"><input type="checkbox" checked={draft.pinned} onChange={(event) => setDraft({ ...draft, pinned: event.target.checked })} />置顶</label><button className="primary" disabled={busy} onClick={() => void save()}>{busy ? '保存中…' : '保存收藏'}</button></div></> : selected ? <><header><div><small>{kinds[selected.kind]}</small><h2>{selected.title}</h2></div><div><button title={selected.pinned ? '取消置顶' : '置顶'} onClick={() => void togglePin(selected)}><Pin size={16} /></button><button title="编辑" onClick={() => startEdit(selected)}><Pencil size={16} /></button><button title="删除" onClick={() => void remove(selected)}><Trash2 size={16} /></button></div></header><div className="favorites-detail-content"><p>{selected.summary || '暂无简介'}</p><button className="favorites-open" onClick={() => open(selected)}><ExternalLink size={15} />打开原链接</button><small title={selected.url}>{selected.url}</small>{selected.note && <div className="favorites-note"><h3>个人备注</h3><MarkdownPreview text={selected.note} /></div>}</div></> : <div className="empty favorites-welcome"><Bookmark size={28} /><h2>选择一条链接</h2><p>可以查看备注、打开原网页或编辑显示内容。</p></div>}</section>
    </div>
  </>;
}
