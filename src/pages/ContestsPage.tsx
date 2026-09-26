import { useEffect, useRef, useState } from 'react';
import { FileUp, Pencil, Plus, Play, Trash2 } from 'lucide-react';
import PlatformIcon from '../components/PlatformIcon';
import ConfirmDialog from '../components/ConfirmDialog';
import { MarkdownPreview } from '../components/MarkdownNote';
import { parseProblemUrl } from '../lib/problemIdentity';
import { api } from '../services/api';
import type { Contest, ContestInput, ProblemSet, ProblemSetProblem, TrainingMatch, TrainingMode } from '../types';

const empty = (): ContestInput => ({ id: null, title: '', description: '', origin: 'manual', sourceSetId: null, mode: 'balanced', durationMinutes: 120, tagVisibility: 'after_ac', targetSolveRateMin: .5, targetSolveRateMax: .7, problems: [] });
const targets: Record<TrainingMode, [number, number]> = { relaxed: [.7, .9], balanced: [.5, .7], pressure: [.35, .55] };

export default function ContestsPage({ notify, onOpenVp }: { notify: (message: string) => void; onOpenVp: () => void }) {
  const [contests, setContests] = useState<Contest[]>([]);
  const [sets, setSets] = useState<ProblemSet[]>([]);
  const [matches, setMatches] = useState<TrainingMatch[]>([]);
  const [draft, setDraft] = useState<ContestInput | null>(null);
  const [links, setLinks] = useState('');
  const [busy, setBusy] = useState(false);
  const [contestToDelete, setContestToDelete] = useState<Contest | null>(null);
  const [problemToRemove, setProblemToRemove] = useState<number | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const reload = async () => { const [next, nextSets, nextMatches] = await Promise.all([api.listContests(), api.listProblemSets(), api.listTrainingMatches()]); setContests(next); setSets(nextSets); setMatches(nextMatches); };
  useEffect(() => { void reload().catch((error) => notify(String(error))); }, [notify]);
  const save = async () => {
    if (!draft || busy) return;
    setBusy(true);
    try {
      const result = await api.saveContest(draft);
      setDraft(null); await reload(); notify(`比赛“${result.title}”已保存`);
    } catch (error) { notify(String(error)); }
    finally { setBusy(false); }
  };
  const fromSet = async (setId: number) => { try { const item = await api.contestFromSet(setId, 'balanced', 120); await reload(); notify(`已从题单创建比赛“${item.title}”`); } catch (error) { notify(String(error)); } };
  const importFile = async (file?: File) => { if (!file) return; try { const contest = await api.importGeneratedContest(await file.text()); await reload(); notify(`比赛“${contest.title}”已导入`); } catch (error) { notify(String(error)); } finally { if (inputRef.current) inputRef.current.value = ''; } };
  const addLinks = () => {
    if (!draft) return;
    const lines = links.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    const parsed = lines.map(parseProblemUrl);
    const entries: ProblemSetProblem[] = parsed.flatMap((problem) => problem ? [{ position: 0, role: 'Core', note: '', problem: { ...problem, canonicalId: '', difficulty: null, tags: [], trainingSuitability: null, observationDependency: null, implementationLoad: null, knowledgeDependency: null, interactive: false, outputOnly: false } }] : []);
    setDraft((current) => current ? { ...current, problems: [...current.problems, ...entries] } : current);
    const invalid = lines.filter((_, index) => !parsed[index]);
    setLinks(invalid.join('\n'));
    notify(`已添加 ${entries.length} 道题${invalid.length ? `；${invalid.length} 条链接无法识别，已保留` : ''}`);
  };
  const start = async (contest: Contest) => { try { await api.queueContest(contest.id, 0); notify('已加入参赛区，可在“开始 VP”旁设置赛前倒计时'); onOpenVp(); } catch (error) { notify(String(error)); } };
  const remove = async () => { if (!contestToDelete) return; try { await api.deleteContest(contestToDelete.id); setContestToDelete(null); await reload(); notify('比赛已删除'); } catch (error) { notify(String(error)); } };
  return <><header className="topbar"><div><small>CONTEST LIBRARY</small><h1>模拟赛</h1><p>管理比赛配置和历史场次。从题单创建、手动组题，或导入 AI 生成的 JSON。比赛可以多次加入参赛区。</p></div><div className="compact-actions"><input ref={inputRef} hidden type="file" accept="application/json,.json" onChange={(event) => void importFile(event.target.files?.[0])} /><button onClick={() => inputRef.current?.click()}><FileUp size={15} />导入比赛 JSON</button><button className="primary" onClick={() => setDraft(empty())}><Plus size={15} />新建比赛</button></div></header>
    <section className="panel contest-create-from-set"><strong>从题单创建</strong><span>题单保持可复用；生成的是独立比赛快照。</span><select defaultValue="" onChange={(event) => { if (event.target.value) void fromSet(Number(event.target.value)); event.target.value = ''; }}><option value="">选择题单…</option>{sets.map((set) => <option key={set.id} value={set.id}>{set.title} · {set.problems.length} 题</option>)}</select></section>
    {draft && <section className="panel contest-editor"><header><h2>{draft.id ? '编辑比赛' : '新建比赛'}</h2><button onClick={() => setDraft(null)}>取消</button></header><div className="contest-form"><label>比赛名称<input value={draft.title} onChange={(event) => setDraft({ ...draft, title: event.target.value })} /></label><label>模式<select value={draft.mode} onChange={(event) => { const mode = event.target.value as TrainingMode; setDraft({ ...draft, mode, targetSolveRateMin: targets[mode][0], targetSolveRateMax: targets[mode][1] }); }}><option value="relaxed">轻松</option><option value="balanced">均衡</option><option value="pressure">压力</option></select></label><label>时长（分钟）<input type="number" min={15} max={480} value={draft.durationMinutes} onChange={(event) => setDraft({ ...draft, durationMinutes: Number(event.target.value) })} /></label><label>目标完成比例（最小 %）<input type="number" min={0} max={100} value={Math.round(draft.targetSolveRateMin * 100)} onChange={(event) => setDraft({ ...draft, targetSolveRateMin: Number(event.target.value) / 100 })} /></label><label>目标完成比例（最大 %）<input type="number" min={0} max={100} value={Math.round(draft.targetSolveRateMax * 100)} onChange={(event) => setDraft({ ...draft, targetSolveRateMax: Number(event.target.value) / 100 })} /></label><label>标签可见性<select value={draft.tagVisibility} onChange={(event) => setDraft({ ...draft, tagVisibility: event.target.value as ContestInput['tagVisibility'] })}><option value="after_ac">AC 后显示</option><option value="never">始终隐藏</option><option value="before_solving">解题前显示</option></select></label><div className="wide contest-description-editor"><span>比赛说明（Markdown · 支持 LaTeX）</span><div className="markdown-split"><div><strong>编辑</strong><textarea value={draft.description} onChange={(event) => setDraft({ ...draft, description: event.target.value })} placeholder="支持 Markdown、$行内公式$ 和 $$独立公式$$" /></div><div><strong>预览</strong><MarkdownPreview text={draft.description} /></div></div></div></div><div className="link-adder"><strong>添加题目链接</strong><textarea value={links} onChange={(event) => setLinks(event.target.value)} placeholder="每行一个题目链接" /><button onClick={addLinks} disabled={!links.trim()}>添加链接</button></div><div className="contest-edit-problems">{draft.problems.map((entry, index) => <div key={index}><PlatformIcon platform={entry.problem.platform} /><span>{index + 1}</span><input value={entry.problem.name} onChange={(event) => setDraft({ ...draft, problems: draft.problems.map((problem, position) => position === index ? { ...problem, problem: { ...problem.problem, name: event.target.value } } : problem) })} aria-label="题目名称" /><select aria-label="题目角色" value={entry.role} onChange={(event) => setDraft({ ...draft, problems: draft.problems.map((problem, position) => position === index ? { ...problem, role: event.target.value as ProblemSetProblem['role'] } : problem) })}><option value="Warmup">热身</option><option value="Stable">稳定题</option><option value="Core">核心题</option><option value="Weakness">弱项</option><option value="Observation">观察题</option><option value="Stretch">上限题</option></select><button title="移除" onClick={() => setProblemToRemove(index)}><Trash2 size={14} /></button></div>)}</div><footer><span>{draft.problems.length} 道题</span><button className="primary" disabled={busy} onClick={() => void save()}>保存比赛</button></footer></section>}
    <div className="contest-grid">{contests.map((contest) => { const sessions = matches.filter((match) => match.contestId === contest.id); const active = sessions.filter((match) => match.status !== 'finished'); const finished = sessions.filter((match) => match.status === 'finished'); return <article className="panel contest-card" key={contest.id}><header><small>{contest.origin === 'ai' ? 'AI 生成' : contest.origin === 'problem_set' ? '来自题单' : '手动创建'}</small><h2>{contest.title}</h2><div className="contest-description-preview"><MarkdownPreview text={contest.description || '暂无说明'} /></div></header><div className="contest-stats"><span>{contest.problems.length} 道题</span><span>{contest.durationMinutes} 分钟</span><span>{active.length} 场参赛中</span><span>{finished.length} 场已结束</span></div><div className="contest-problem-preview">{contest.problems.map((entry) => <span key={`${entry.problem.platform}:${entry.problem.problemKey}`}><PlatformIcon platform={entry.problem.platform} />{entry.problem.name || entry.problem.problemId}</span>)}</div>{sessions.length > 0 && <details className="contest-session-history"><summary>查看 {sessions.length} 场 VP 状态</summary>{sessions.map((match) => <div key={match.id}><span>{new Date(match.createdAt * 1000).toLocaleDateString()} · {match.status === 'finished' ? '已结束' : match.status === 'paused' ? '暂停中' : match.status === 'waiting' ? '待开始' : '进行中'}</span><strong>{match.problems.filter((entry) => entry.solved).length}/{match.problems.length} AC</strong></div>)}</details>}<footer><button title="编辑比赛" onClick={() => setDraft({ ...contest, id: contest.id })}><Pencil size={15} /></button><button title="删除比赛" onClick={() => setContestToDelete(contest)}><Trash2 size={15} /></button><button className="primary" onClick={() => void start(contest)}><Play size={15} />加入参赛区</button></footer></article>; })}</div>
    {problemToRemove !== null && <ConfirmDialog title="移除题目？" message={`从当前比赛移除第 ${problemToRemove + 1} 题；保存比赛后才会生效。`} onCancel={() => setProblemToRemove(null)} onConfirm={() => { setDraft((current) => current ? { ...current, problems: current.problems.filter((_, index) => index !== problemToRemove) } : current); setProblemToRemove(null); }} />}
    {contestToDelete && <ConfirmDialog title="删除模拟赛？" message={`将删除“${contestToDelete.title}”的比赛配置，已有 VP 记录会保留。`} onCancel={() => setContestToDelete(null)} onConfirm={remove} />}
  </>;
}
