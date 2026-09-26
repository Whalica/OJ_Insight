import { useEffect, useRef, useState } from 'react';
import { Download, ExternalLink, FileUp, Link2, Pencil, Play, Plus, Save, Share2, Trash2, X } from 'lucide-react';

import PlatformIcon from '../components/PlatformIcon';
import ConfirmDialog from '../components/ConfirmDialog';
import { MarkdownPreview } from '../components/MarkdownNote';
import { useTraining } from '../hooks/useTraining';
import { parseProblemUrl } from '../lib/problemIdentity';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import { api } from '../services/api';
import { openTrainingExportDirectory, saveTrainingJson } from '../services/training';
import type { ProblemSet, ProblemSetInput, ProblemSetProblem, TrainingRole } from '../types';

const ROLES: TrainingRole[] = ['Warmup', 'Stable', 'Core', 'Weakness', 'Observation', 'Stretch'];
const ROLE_NAMES: Record<TrainingRole, string> = { Warmup: '热身', Stable: '稳定题', Core: '核心题', Weakness: '弱项', Observation: '观察题', Stretch: '上限题' };
const emptyProblem = (): ProblemSetProblem => ({ position: 0, role: 'Core', note: '', problem: { canonicalId: '', platform: 'codeforces', problemKey: '', problemId: '', name: '', url: '', difficulty: null, tags: [], trainingSuitability: null, observationDependency: null, implementationLoad: null, knowledgeDependency: null, interactive: false, outputOnly: false } });
const emptyDraft = (): ProblemSetInput => ({ id: null, title: '', description: '', setType: 'static', tagVisibility: 'after_ac', sourceSetId: null, sourceUrl: null, problems: [] });

export default function ProblemSetsPage({ notify, onTrain }: { notify: (message: string) => void; onTrain: (setId: number) => void }) {
  const { sets, loading, saveSet, deleteSet, importSet } = useTraining(notify);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [editing, setEditing] = useState(false);
  const [saving, setSaving] = useState(false);
  const [setToDelete, setSetToDelete] = useState<ProblemSet | null>(null);
  const [communityDraftFor, setCommunityDraftFor] = useState<ProblemSet | null>(null);
  const [draft, setDraft] = useState<ProblemSetInput>(emptyDraft);
  const [urlInput, setUrlInput] = useState('');
  const [luoguUrl, setLuoguUrl] = useState('');
  const [luoguOpen, setLuoguOpen] = useState(false);
  const [luoguLoading, setLuoguLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const selected = sets.find((set) => set.id === selectedId) || sets[0] || null;

  useEffect(() => { if (selectedId == null && sets[0]) setSelectedId(sets[0].id); }, [sets, selectedId]);
  const beginNew = () => { setDraft(emptyDraft()); setEditing(true); };
  const beginEdit = (set: ProblemSet) => { setDraft({ id: set.id, title: set.title, description: set.description, setType: 'static', tagVisibility: set.tagVisibility, sourceSetId: set.sourceSetId, sourceUrl: set.sourceUrl, problems: set.problems }); setEditing(true); };
  const updateProblem = (index: number, patch: Omit<Partial<ProblemSetProblem>, 'problem'> & { problem?: Partial<ProblemSetProblem['problem']> }) => setDraft((current) => ({ ...current, problems: current.problems.map((entry, position) => position === index ? { ...entry, ...patch, problem: { ...entry.problem, ...(patch.problem || {}) } } : entry) }));
  const lookupDraftProblem = async (index: number) => {
    const problem = draft.problems[index]?.problem;
    if (!problem || !parseProblemUrl(problem.url)) return;
    try { updateProblem(index, { problem: await api.lookupProblemMetadata(problem) }); }
    catch (error) { notify(`题目标题未识别：${String(error)}。可重试或在高级信息中填写显示名称。`); }
  };
  const addLinks = () => {
    const lines = urlInput.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    const parsed = lines.map(parseProblemUrl);
    const entries = parsed.flatMap((problem) => problem ? [{ ...emptyProblem(), problem: { ...emptyProblem().problem, ...problem } }] : []);
    setDraft((current) => ({ ...current, problems: [...current.problems, ...entries] }));
    const invalid = lines.filter((_, index) => !parsed[index]);
    setUrlInput(invalid.join('\n'));
    notify(`已添加 ${entries.length} 道题${invalid.length ? `；${invalid.length} 条链接无法识别，已保留` : ''}。标题可在高级信息中填写。`);
  };
  const submit = async () => {
    if (saving) return;
    setSaving(true);
    try {
      const saved = await saveSet({ ...draft, problems: draft.problems.map((entry, position) => ({ ...entry, position })) });
      setSelectedId(saved.id); setEditing(false); notify('题单已保存');
    } catch (error) { notify(String(error)); }
    finally { setSaving(false); }
  };
  const onImport = async (file?: File) => { if (!file) return; try { const imported = await importSet(await file.text()); setSelectedId(imported.id); setEditing(false); notify(`题单“${imported.title}”已导入`); } catch (error) { notify(String(error)); } finally { if (inputRef.current) inputRef.current.value = ''; } };
  const exportSet = async (set: ProblemSet) => { try { const data = await api.exportProblemSet(set.id); const saved = await saveTrainingJson(`OJ-Insight-题单-${set.id}.json`, data); if (saved) notify(`题单已保存到 ${saved.path}`); } catch (error) { notify(String(error)); } };
  const exportCommunityDraft = async (set: ProblemSet, id: string, summary: string, author: string, categories: string[], license: string) => {
    const content = JSON.parse(await api.exportProblemSet(set.id));
    content.sourceSetId = null;
    content.sourceUrl = null;
    const entry = { schema: 'com.ojinsight.community-entry', schemaVersion: 1, id, type: 'problem-set', title: set.title, summary, author: { name: author, url: null }, categories, license, sourceUrl: null, content };
    const saved = await saveTrainingJson(`${id}.json`, `${JSON.stringify(entry, null, 2)}\n`);
    if (saved) { setCommunityDraftFor(null); notify(`投稿文件已保存到 ${saved.path}；请在推荐题单仓库发起 PR。`); }
  };
  const removeSet = async () => { if (!setToDelete) return; try { await deleteSet(setToDelete.id); setSelectedId(null); setEditing(false); setSetToDelete(null); notify('题单已删除'); } catch (error) { notify(String(error)); } };
  const previewLuogu = async () => {
    if (!luoguUrl.trim() || luoguLoading) return;
    setLuoguLoading(true);
    try {
      const imported = await api.previewLuoguProblemSet(luoguUrl);
      setDraft(imported); setEditing(true); setLuoguOpen(false);
      notify(`已读取 ${imported.problems.length} 道题，请检查后保存。`);
    } catch (error) { notify(`洛谷题单读取失败：${String(error)}`); }
    finally { setLuoguLoading(false); }
  };

  return <>
    <header className="topbar"><div><small>TRAINING CENTER · PROBLEM SETS</small><h1>题单</h1><p>查看、整理和分享跨 OJ 题单。点击题单进入详情，编辑和导出使用卡片右侧操作。</p></div><div className="compact-actions"><input ref={inputRef} hidden type="file" accept="application/json,.json" onChange={(event) => void onImport(event.target.files?.[0])} /><button title="导入 OJ Insight 题单 JSON" onClick={() => inputRef.current?.click()}><FileUp size={15} /><span>导入题单</span></button><button onClick={() => setLuoguOpen((open) => !open)}><Link2 size={15} /><span>导入洛谷题单</span></button><button className="primary" onClick={beginNew}><Plus size={15} /><span>新建题单</span></button></div></header>
    {luoguOpen && <section className="panel luogu-import"><div><strong>从洛谷题单链接导入</strong><small>读取题单的题目列表，先预览编辑，再保存到本地。洛谷可能要求登录，可在设置中配置 Cookie。</small></div><div><input aria-label="洛谷题单链接" value={luoguUrl} onChange={(event) => setLuoguUrl(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') void previewLuogu(); }} placeholder="https://www.luogu.com.cn/training/12345" /><button className="primary" disabled={!luoguUrl.trim() || luoguLoading} onClick={() => void previewLuogu()}>{luoguLoading ? '读取中…' : '读取题目'}</button></div></section>}
    <div className="problem-sets-layout">
      <aside className="panel set-gallery"><header><div><strong>题单</strong><small>{loading ? '读取中…' : `${sets.length} 个`}</small></div></header>{sets.length ? sets.map((set) => <article key={set.id} className={selected?.id === set.id && !editing ? 'active' : ''}><button className="set-card-main" onClick={() => { setSelectedId(set.id); setEditing(false); }}><div className="set-card-icons">{[...new Set(set.problems.map((entry) => entry.problem.platform))].slice(0, 4).map((platform) => <PlatformIcon key={platform} platform={platform} />)}</div><strong>{set.title}</strong><span>{set.problems.length} 道题 · 已通过 {set.solvedKeys?.length || 0} 题</span></button><div className="set-card-actions"><button title="编辑题单" onClick={() => beginEdit(set)}><Pencil size={14} /></button><button title="导出题单" onClick={() => void exportSet(set)}><Download size={14} /></button><button title="删除题单" onClick={() => setSetToDelete(set)}><Trash2 size={14} /></button></div></article>) : <div className="set-empty"><strong>还没有题单</strong><span>粘贴题目链接创建，或导入 OJ Insight 题单 JSON。</span><button onClick={beginNew}><Plus size={14} />创建第一个题单</button></div>}</aside>
      {editing ? <SetEditor draft={draft} setDraft={setDraft} urlInput={urlInput} setUrlInput={setUrlInput} addLinks={addLinks} updateProblem={updateProblem} lookupProblem={lookupDraftProblem} saving={saving} onSave={() => void submit()} onCancel={() => setEditing(false)} /> : selected ? <SetDetail set={selected} onEdit={() => beginEdit(selected)} onExport={() => void exportSet(selected)} onCommunityDraft={() => setCommunityDraftFor(selected)} onDelete={() => setSetToDelete(selected)} onTrain={() => onTrain(selected.id)} onOpenExports={() => void openTrainingExportDirectory().catch((error) => notify(String(error)))} /> : <section className="panel set-welcome"><Link2 size={30} /><h2>用链接建立第一份题单</h2><p>只需要粘贴题目链接，OJ Insight 会自动识别平台与题目标识。</p><button className="primary" onClick={beginNew}><Plus size={15} />新建题单</button></section>}
    </div>
    {setToDelete && <ConfirmDialog title="删除题单？" message={`将删除“${setToDelete.title}”。训练记录和同步数据会保留，题单本身无法恢复。`} onCancel={() => setSetToDelete(null)} onConfirm={removeSet} />}
    {communityDraftFor && <CommunityDraftDialog set={communityDraftFor} onCancel={() => setCommunityDraftFor(null)} onExport={exportCommunityDraft} notify={notify} />}
  </>;
}

function SetDetail({ set, onEdit, onExport, onCommunityDraft, onDelete, onTrain, onOpenExports }: { set: ProblemSet; onEdit: () => void; onExport: () => void; onCommunityDraft: () => void; onDelete: () => void; onTrain: () => void; onOpenExports: () => void }) {
  const roles = new Map<TrainingRole, number>();
  const solved = new Set(set.solvedKeys || []);
  for (const entry of set.problems) roles.set(entry.role, (roles.get(entry.role) || 0) + 1);
  return <section className="panel set-detail"><header><div><small>固定题单 · {set.problems.length} 题 · 已通过 {solved.size} 题</small><h2>{set.title}</h2><div className="set-description-preview"><MarkdownPreview text={set.description || '暂无说明'} /></div></div><div className="icon-actions"><button title="编辑题单" onClick={onEdit}><Pencil size={15} /></button><button title="导出题单" onClick={onExport}><Download size={15} /></button><button title="制作社区投稿文件" onClick={onCommunityDraft}><Share2 size={15} /></button><button title="打开导出文件夹" onClick={onOpenExports}><ExternalLink size={15} /></button><button className="danger" title="删除题单" onClick={onDelete}><Trash2 size={15} /></button></div></header><div className="set-overview"><div><span>平台</span><strong>{[...new Set(set.problems.map((entry) => entry.problem.platform))].map((platform) => PLATFORM_META[platform].name).join(' · ') || '暂无'}</strong></div><div><span>角色分布</span><strong>{[...roles].map(([role, count]) => `${ROLE_NAMES[role]} ${count}`).join(' · ') || '暂无'}</strong></div><div><span>标签</span><strong>{set.tagVisibility === 'after_ac' ? 'AC 后显示' : set.tagVisibility === 'never' ? '始终隐藏' : '解题前显示'}</strong></div></div><div className="set-problem-list">{set.problems.map((entry) => { const passed = solved.has(`${entry.problem.platform}:${entry.problem.problemKey}`); return <a key={entry.problem.canonicalId || `${entry.problem.platform}:${entry.problem.problemKey}`} href={entry.problem.url || undefined} target="_blank" rel="noreferrer"><PlatformIcon platform={entry.problem.platform} /><span className="problem-number">{entry.position + 1}</span><div><strong>{entry.problem.name || `${PLATFORM_META[entry.problem.platform].short} ${entry.problem.problemId || entry.problem.problemKey}`}</strong><small>{PLATFORM_META[entry.problem.platform].name} · {ROLE_NAMES[entry.role]}{entry.note ? ` · ${entry.note}` : ''}</small>{(set.tagVisibility === 'before_solving' || set.tagVisibility === 'after_ac' && passed) && entry.problem.tags.length > 0 && <em>{entry.problem.tags.join(' · ')}</em>}</div><span className={`set-solve-badge ${passed ? 'passed' : ''}`}>{passed ? '已通过' : '未记录通过'}</span></a>; })}</div><footer><span>通过状态来自本地同步记录；题单可反复使用。</span><button className="primary" onClick={onTrain}><Play size={16} />转为模拟赛</button></footer></section>;
}

function CommunityDraftDialog({ set, onCancel, onExport, notify }: { set: ProblemSet; onCancel: () => void; onExport: (set: ProblemSet, id: string, summary: string, author: string, categories: string[], license: string) => Promise<void>; notify: (message: string) => void }) {
  const [id, setId] = useState('');
  const [summary, setSummary] = useState('');
  const [author, setAuthor] = useState('');
  const [categories, setCategories] = useState('');
  const [license, setLicense] = useState('CC-BY-4.0');
  const [confirmed, setConfirmed] = useState(false);
  const [saving, setSaving] = useState(false);
  const submit = async () => {
    if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(id) || id.length > 100) { notify('文件标识请使用小写英文字母、数字和连字符。'); return; }
    if (!summary.trim() || !author.trim() || !confirmed) { notify('请填写简介、作者，并确认可以公开分享。'); return; }
    setSaving(true);
    try { await onExport(set, id, summary.trim(), author.trim(), categories.split(',').map((item) => item.trim()).filter(Boolean), license); }
    catch (error) { notify(`制作投稿文件失败：${String(error)}`); }
    finally { setSaving(false); }
  };
  return <div className="training-confirm-backdrop"><section className="panel community-draft-dialog" role="dialog" aria-modal="true" aria-label="制作社区投稿文件"><header><h2>制作社区投稿文件</h2><button title="关闭" onClick={onCancel}><X size={16} /></button></header><p>从“{set.title}”生成投稿 JSON。检查题单说明和备注后，到「推荐题单」中的仓库发起 PR。</p><label><span>文件标识</span><input value={id} onChange={(event) => setId(event.target.value.toLowerCase())} placeholder="graph-basics-2026" /></label><label><span>简介</span><textarea value={summary} onChange={(event) => setSummary(event.target.value)} maxLength={500} placeholder="用一句话说明适合谁、练什么" /></label><label><span>作者或 GitHub 用户名</span><input value={author} onChange={(event) => setAuthor(event.target.value)} /></label><label><span>分类（逗号分隔）</span><input value={categories} onChange={(event) => setCategories(event.target.value)} placeholder="图论, 入门" /></label><label><span>授权方式</span><select value={license} onChange={(event) => setLicense(event.target.value)}><option value="CC-BY-4.0">CC BY 4.0</option><option value="CC-BY-SA-4.0">CC BY-SA 4.0</option><option value="CC0-1.0">CC0 1.0</option></select></label><label className="community-confirm"><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} /><span>我确认题单内容可公开，说明与备注不含隐私或无权转载的题面。</span></label><footer><button onClick={onCancel}>取消</button><button className="primary" disabled={saving} onClick={() => void submit()}><Share2 size={15} />{saving ? '导出中…' : '导出投稿 JSON'}</button></footer></section></div>;
}

type EditorProps = { draft: ProblemSetInput; setDraft: (value: ProblemSetInput | ((current: ProblemSetInput) => ProblemSetInput)) => void; urlInput: string; setUrlInput: (value: string) => void; addLinks: () => void; updateProblem: (index: number, patch: Omit<Partial<ProblemSetProblem>, 'problem'> & { problem?: Partial<ProblemSetProblem['problem']> }) => void; lookupProblem: (index: number) => Promise<void>; saving: boolean; onSave: () => void; onCancel: () => void };
function SetEditor({ draft, setDraft, urlInput, setUrlInput, addLinks, updateProblem, lookupProblem, saving, onSave, onCancel }: EditorProps) {
  const [problemToRemove, setProblemToRemove] = useState<number | null>(null);
  return <section className="panel set-editor"><header><div><small>{draft.id ? 'EDIT PROBLEM SET' : 'NEW PROBLEM SET'}</small><h2>{draft.id ? '编辑题单' : '创建题单'}</h2></div><div className="compact-actions"><button onClick={onCancel}><X size={15} />取消</button><button className="primary" onClick={onSave} disabled={saving}><Save size={15} />{saving ? '正在保存…' : '保存题单'}</button></div></header><div className="set-fields"><label><span>题单名称</span><input value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} placeholder="例如：图论基础训练" /></label><label><span>标签什么时候显示</span><select value={draft.tagVisibility} onChange={(event) => setDraft({ ...draft, tagVisibility: event.target.value as ProblemSetInput['tagVisibility'] })}><option value="after_ac">AC 后显示</option><option value="before_solving">解题前显示</option><option value="never">始终隐藏</option></select></label><div className="wide set-description-editor"><span>题单说明（Markdown · 支持 LaTeX）</span><div className="markdown-split"><div><strong>编辑</strong><textarea value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} placeholder="支持 Markdown、$行内公式$ 和 $$独立公式$$" /></div><div><strong>预览</strong><MarkdownPreview text={draft.description} /></div></div></div></div><div className="link-adder"><div><Link2 size={18} /><span><strong>粘贴题目链接</strong><small>支持一次粘贴多行，自动识别 Codeforces、AtCoder、洛谷、牛客、QOJ 和 LeetCode。</small></span></div><textarea value={urlInput} onChange={(event) => setUrlInput(event.target.value)} placeholder={'每行一个题目链接\nhttps://codeforces.com/contest/1935/problem/C'} /><button disabled={!urlInput.trim()} onClick={addLinks}><Plus size={15} />添加链接</button></div><div className="problem-editor-cards"><header><strong>题目</strong><button onClick={() => setDraft({ ...draft, problems: [...draft.problems, { ...emptyProblem(), position: draft.problems.length }] })}><Plus size={14} />手动添加</button></header>{draft.problems.map((entry, index) => <article key={index}><span className="problem-index">{index + 1}</span><PlatformIcon platform={entry.problem.platform} /><div className="problem-edit-main"><input value={entry.problem.url} onChange={(event) => { const parsed = parseProblemUrl(event.target.value); updateProblem(index, { problem: parsed ? { ...parsed } : { url: event.target.value } }); }} placeholder="题目链接" /><div><select value={entry.role} onChange={(event) => updateProblem(index, { role: event.target.value as TrainingRole })}>{ROLES.map((role) => <option key={role} value={role}>{ROLE_NAMES[role]}</option>)}</select><input value={entry.note} onChange={(event) => updateProblem(index, { note: event.target.value })} placeholder="备注（可选）" /></div><details><summary>高级信息</summary><button type="button" className="optional-lookup" onClick={() => void lookupProblem(index)}>尝试获取标题与标签（可选）</button><div className="advanced-problem-fields"><label><span>所属平台</span><select value={entry.problem.platform} onChange={(event) => updateProblem(index, { problem: { platform: event.target.value as ProblemSetProblem['problem']['platform'] } })}>{PLATFORM_ORDER.map((platform) => <option key={platform} value={platform}>{PLATFORM_META[platform].name}</option>)}</select></label><label><span>题目标识</span><input value={entry.problem.problemKey} onChange={(event) => updateProblem(index, { problem: { problemKey: event.target.value } })} placeholder="无法识别链接时填写" /></label><label><span>显示名称（可选）</span><input value={entry.problem.name} onChange={(event) => updateProblem(index, { problem: { name: event.target.value } })} /></label><label><span>标签（逗号分隔）</span><input value={entry.problem.tags.join(', ')} onChange={(event) => updateProblem(index, { problem: { tags: event.target.value.split(',').map((tag) => tag.trim()).filter(Boolean) } })} /></label></div></details></div><div className="problem-row-actions"><button disabled={index === 0} title="上移" onClick={() => setDraft((current) => { const problems = [...current.problems]; [problems[index - 1], problems[index]] = [problems[index], problems[index - 1]]; return { ...current, problems }; })}>↑</button><button disabled={index === draft.problems.length - 1} title="下移" onClick={() => setDraft((current) => { const problems = [...current.problems]; [problems[index + 1], problems[index]] = [problems[index], problems[index + 1]]; return { ...current, problems }; })}>↓</button><button title="移除" onClick={() => setProblemToRemove(index)}><Trash2 size={14} /></button></div></article>)}</div>{problemToRemove !== null && <ConfirmDialog title="移除题目？" message={`从当前题单移除第 ${problemToRemove + 1} 题；保存题单后才会生效。`} onCancel={() => setProblemToRemove(null)} onConfirm={() => { setDraft((current) => ({ ...current, problems: current.problems.filter((_, index) => index !== problemToRemove) })); setProblemToRemove(null); }} />}</section>;
}
