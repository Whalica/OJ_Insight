import { useEffect, useRef, useState } from 'react';
import { ChevronDown, ExternalLink, Flag, Pause, Play, RefreshCw, Trash2 } from 'lucide-react';
import MarkdownNote from '../components/MarkdownNote';
import PlatformIcon from '../components/PlatformIcon';
import { PLATFORM_META } from '../lib/platforms';
import { api } from '../services/api';
import type { TrainingMatch, TrainingMatchProblem, VpSubmission } from '../types';

const verdictName = (value: string) => ({ OK: 'AC', AC: 'AC', WRONG_ANSWER: 'WA', WA: 'WA', TIME_LIMIT_EXCEEDED: 'TLE', TLE: 'TLE', MEMORY_LIMIT_EXCEEDED: 'MLE', MLE: 'MLE', RUNTIME_ERROR: 'RE', RE: 'RE', COMPILATION_ERROR: 'CE', CE: 'CE' } as Record<string, string>)[value] || value;
function elapsedRemaining(item: TrainingMatch, now: number) { const reference = item.pausedAt || Math.floor(now / 1000); return Math.max(0, item.durationMinutes * 60 - (reference - item.startedAt - item.totalPausedSeconds)); }

function VpCard({ item, notify, reload }: { item: TrainingMatch; notify: (message: string) => void; reload: () => Promise<void> }) {
  const [expanded, setExpanded] = useState(item.status === 'running');
  const [now, setNow] = useState(Date.now());
  const [submissions, setSubmissions] = useState<VpSubmission[]>([]);
  const [checking, setChecking] = useState(false);
  const transitioning = useRef(false);
  useEffect(() => { void api.listVpSubmissions(item.id).then(setSubmissions).catch(() => undefined); }, [item.id]);
  useEffect(() => { if (item.status !== 'running') return; const sync = async () => { try { setSubmissions(await api.refreshVpSubmissions(item.id)); await api.refreshTrainingMatch(item.id); await reload(); } catch { /* Explicit refresh reports errors. */ } }; void sync(); const timer = window.setInterval(() => void sync(), 90_000); return () => window.clearInterval(timer); }, [item.id, item.status]);
  useEffect(() => { if (item.status !== 'running' && item.status !== 'waiting') return; const timer = window.setInterval(() => setNow(Date.now()), 1000); return () => window.clearInterval(timer); }, [item.status]);
  useEffect(() => { if (item.status !== 'waiting' || !item.scheduledStartAt || Date.now() < item.scheduledStartAt * 1000 || transitioning.current) return; transitioning.current = true; void api.startVp(item.id).then(reload).finally(() => { transitioning.current = false; }).catch(() => undefined); }, [item.id, item.status, item.scheduledStartAt, now, reload]);
  useEffect(() => { if (item.status !== 'running' || elapsedRemaining(item, now) > 0 || transitioning.current) return; transitioning.current = true; void api.finishTrainingMatch(item.id).then(reload).finally(() => { transitioning.current = false; }).catch(() => undefined); }, [item, now, reload]);
  const action = async (operation: () => Promise<unknown>, message: string) => { try { await operation(); await reload(); notify(message); } catch (error) { notify(String(error)); } };
  const check = async () => { setChecking(true); try { const entries = await api.refreshVpSubmissions(item.id); setSubmissions(entries); await api.refreshTrainingMatch(item.id); await reload(); notify(`已获取 ${entries.length} 条可用提交记录`); } catch (error) { notify(String(error)); } finally { setChecking(false); } };
  const remove = () => { if (!confirm(`确定删除 VP“${item.title}”及其笔记和代码绑定？`)) return; void action(() => api.deleteTrainingMatch(item.id), 'VP 已删除'); };
  const solved = item.problems.filter((entry) => entry.solved).length;
  const remaining = item.status === 'running' || item.status === 'paused' ? elapsedRemaining(item, now) : 0;
  const state = item.status === 'waiting' ? `赛前倒计时 ${Math.max(0, Math.ceil(((item.scheduledStartAt || 0) * 1000 - now) / 1000))} 秒` : item.status === 'paused' ? '已暂停' : `${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, '0')}`;
  return <article className={`training-match vp-session ${item.status}`}><header><button className="match-title" onClick={() => setExpanded((value) => !value)}><ChevronDown className={expanded ? 'expanded' : ''} size={16} /><div><small>{item.status === 'waiting' ? '等待开始' : item.status === 'paused' ? '暂停中' : '参赛中'}</small><h2>{item.title}</h2></div></button><strong>{state}</strong></header><div className="match-progress"><strong>{solved}/{item.problems.length} 题</strong><span>目标 {Math.round(item.targetSolveRateMin * 100)}%–{Math.round(item.targetSolveRateMax * 100)}%</span><i><b style={{ width: `${item.problems.length ? solved / item.problems.length * 100 : 0}%` }} /></i></div>
    {expanded && <><div className="vp-actions">{item.status === 'waiting' && <button className="primary" disabled={!!item.scheduledStartAt && now < item.scheduledStartAt * 1000} onClick={() => void action(() => api.startVp(item.id), '比赛已开始')}><Play size={15} />开始 VP</button>}{item.status === 'running' && <button onClick={() => void action(() => api.pauseVp(item.id), '比赛已暂停')}><Pause size={15} />暂停</button>}{item.status === 'paused' && <button className="primary" onClick={() => void action(() => api.resumeVp(item.id), '比赛继续')}><Play size={15} />继续</button>}{item.status !== 'waiting' && <button disabled={checking} onClick={() => void check()}><RefreshCw size={15} />检查提交</button>}{item.status !== 'waiting' && <button onClick={() => void action(() => api.finishTrainingMatch(item.id), '比赛已结束，进入赛后分析')}><Flag size={15} />结束比赛</button>}<button title="删除 VP" onClick={remove}><Trash2 size={15} /></button></div>
      <div className="vp-problems">{item.problems.map((entry) => <VpProblem key={entry.position} match={item} entry={entry} submissions={submissions.filter((s) => s.platform === entry.problem.platform && s.problemKey === entry.problem.problemKey)} />)}</div><section className="vp-general-note"><h3>整场笔记</h3><p>记录选题策略、时间分配和赛中感受；自动保存。</p><MarkdownNote value={item.generalNote} onSave={(text) => api.saveVpNote(item.id, null, text)} placeholder="例如：前 30 分钟应该先看完所有题…" /></section>
      <p className="vp-verdict-note">提交结果：Codeforces 官方 API 与 AtCoder 可用记录会显示 AC、WA 等 verdict；其他平台依赖本地 AC 同步，缺失记录不会计作 WA。</p>
    </>}</article>;
}

function VpProblem({ match, entry, submissions }: { match: TrainingMatch; entry: TrainingMatchProblem; submissions: VpSubmission[] }) {
  const [open, setOpen] = useState(false);
  const showTags = match.tagVisibility === 'before_solving' || (match.tagVisibility === 'after_ac' && entry.solved);
  return <div className="vp-problem"><button className="vp-problem-trigger" onClick={() => setOpen((value) => !value)}><ChevronDown className={open ? 'expanded' : ''} size={15} /><PlatformIcon platform={entry.problem.platform} /><span className="problem-number">{entry.position + 1}</span><strong>{entry.problem.name || entry.problem.problemId}</strong><small>{PLATFORM_META[entry.problem.platform].name} · {entry.role}</small><b className={entry.solved ? 'ac' : ''}>{entry.solved ? 'AC' : submissions.length ? verdictName(submissions[submissions.length - 1].verdict) : '未通过'}</b></button>{open && <div className="vp-problem-body"><a href={entry.problem.url} target="_blank" rel="noreferrer">打开原题 <ExternalLink size={13} /></a>{showTags && entry.problem.tags.length > 0 && <p>标签：{entry.problem.tags.join(' · ')}</p>}{submissions.length > 0 && <div className="vp-submissions"><strong>赛时提交</strong>{submissions.map((submission) => <a key={`${submission.submittedAt}-${submission.sourceUrl}`} href={submission.sourceUrl} target="_blank" rel="noreferrer"><span>{new Date(submission.submittedAt * 1000).toLocaleTimeString()}</span><b className={verdictName(submission.verdict) === 'AC' ? 'ac' : ''}>{verdictName(submission.verdict)}</b><ExternalLink size={12} /></a>)}</div>}<h4>做题思路</h4><MarkdownNote value={entry.solutionNote} onSave={(text) => api.saveVpNote(match.id, entry.position, text)} placeholder="记录思路、卡点或赛后补充。支持标题、列表和代码块预览。" /></div>}</div>;
}

export default function VpPage({ notify }: { notify: (message: string) => void }) {
  const [matches, setMatches] = useState<TrainingMatch[]>([]);
  const reload = async () => setMatches(await api.listTrainingMatches());
  useEffect(() => { void reload().catch((error) => notify(String(error))); }, [notify]);
  const active = matches.filter((item) => item.status !== 'finished');
  return <><header className="topbar"><div><small>VIRTUAL PARTICIPATION</small><h1>参赛区</h1><p>准备、倒计时、比赛计时、暂停与赛中笔记都在这里。提交仍在原 OJ 完成。</p></div></header><div className="training-match-list">{active.length ? active.map((item) => <VpCard key={item.id} item={item} notify={notify} reload={reload} />) : <div className="panel training-empty">暂无等待或进行中的 VP。先从“模拟赛”加入一场比赛。</div>}</div></>;
}
