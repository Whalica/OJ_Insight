import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, Download, FileUp, Flag, Play, RefreshCw, Timer } from 'lucide-react';
import { useTraining } from '../hooks/useTraining';
import { api } from '../services/api';
import { saveTrainingJson } from '../services/training';
import type { TrainingMatch, TrainingMode } from '../types';

const MODES: Array<{ id: TrainingMode; name: string; target: string; description: string }> = [
  { id: 'relaxed', name: '轻松', target: '70%–90%', description: '稳定输出与保持题感' },
  { id: 'balanced', name: '均衡', target: '50%–70%', description: '正常训练赛节奏' },
  { id: 'pressure', name: '压力', target: '35%–55%', description: '时间、选题与陌生性压力' },
];

function MatchCard({ item, onRefresh, onFinish }: { item: TrainingMatch; onRefresh: () => void; onFinish: () => void }) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => { if (item.status !== 'running') return; const timer = window.setInterval(() => setNow(Date.now()), 1000); return () => window.clearInterval(timer); }, [item.status]);
  const end = item.startedAt * 1000 + item.durationMinutes * 60_000;
  const remaining = Math.max(0, Math.floor((end - now) / 1000));
  const solved = item.problems.filter((problem) => problem.solved).length;
  return <article className={`training-match ${item.status}`}><header><div><small>{item.mode.toUpperCase()} · {item.status === 'running' ? '进行中' : '已结束'}</small><h2>{item.title}</h2></div><span><Timer size={14} />{item.status === 'running' ? `${Math.floor(remaining / 60)}:${String(remaining % 60).padStart(2, '0')}` : 'FINISHED'}</span></header><div className="match-progress"><strong>{solved}/{item.problems.length}</strong><span>目标 {Math.round(item.targetSolveRateMin * 100)}%–{Math.round(item.targetSolveRateMax * 100)}%</span><i><b style={{ width: `${item.problems.length ? solved / item.problems.length * 100 : 0}%` }} /></i></div><div className="match-problems">{item.problems.map((entry) => { const showTags = item.tagVisibility === 'before_solving' || (item.tagVisibility === 'after_ac' && entry.solved); return <a key={entry.problem.canonicalId} className={entry.solved ? 'solved' : ''} href={entry.problem.url || undefined} target="_blank" rel="noreferrer"><span>{entry.position + 1}</span><div><strong>{entry.problem.name || entry.problem.problemId || entry.problem.problemKey}</strong><small>{entry.problem.platform} · {entry.role}{entry.note ? ` · ${entry.note}` : ''}</small>{showTags && entry.problem.tags.length > 0 && <em>{entry.problem.tags.join(' · ')}</em>}</div>{entry.solved ? <CheckCircle2 size={17} /> : null}</a>; })}</div><footer><button onClick={onRefresh}><RefreshCw size={14} />检查同步结果</button>{item.status === 'running' && <button className="primary" onClick={onFinish}><Flag size={14} />结束训练</button>}</footer></article>;
}

export default function TrainingPage({ notify }: { notify: (message: string) => void }) {
  const { sets, matches, loading, startMatch, importMatch, refreshMatch, finishMatch } = useTraining(notify);
  const [setId, setSetId] = useState<number | null>(null);
  const [mode, setMode] = useState<TrainingMode>('balanced');
  const [duration, setDuration] = useState(120);
  const [target, setTarget] = useState<[number, number]>([50, 70]);
  const fileRef = useRef<HTMLInputElement>(null);
  useEffect(() => { if (setId == null && sets[0]) setSetId(sets[0].id); }, [sets, setId]);
  const start = async () => { if (setId == null) return notify('请先创建或选择题单'); try { await startMatch(setId, mode, duration, target[0] / 100, target[1] / 100); notify('训练赛已开始，已做题已自动排除'); } catch (error) { notify(String(error)); } };
  const pack = async () => { if (setId == null) return notify('请先选择题单'); try { const data = await api.exportTrainingPack(setId, mode); if (await saveTrainingJson(`OJ-Insight-training-pack-${setId}.json`, data)) notify('Training Pack 已导出'); } catch (error) { notify(String(error)); } };
  const onManifest = async (file?: File) => { if (!file) return; try { await importMatch(await file.text()); notify('Match Manifest 已导入并开始训练'); } catch (error) { notify(String(error)); } finally { if (fileRef.current) fileRef.current.value = ''; } };
  return <>
    <header className="topbar"><div><small>TRAINING SYSTEM</small><h1>新时代训练方案</h1><p>从跨 OJ 候选池构造可重复运行的虚拟比赛；提交仍在原 OJ 完成，同步后自动更新 AC。</p></div><div className="topbar-actions"><input ref={fileRef} hidden type="file" accept="application/json,.json" onChange={(event) => void onManifest(event.target.files?.[0])} /><button onClick={() => fileRef.current?.click()}><FileUp size={15} />导入 Match Manifest</button><button onClick={() => void pack()}><Download size={15} />导出 Training Pack</button></div></header>
    <section className="panel training-builder"><div className="training-modes">{MODES.map((entry) => <button key={entry.id} className={mode === entry.id ? 'active' : ''} onClick={() => { setMode(entry.id); const next = entry.id === 'relaxed' ? [70, 90] : entry.id === 'balanced' ? [50, 70] : [35, 55]; setTarget(next as [number, number]); }}><strong>{entry.name}</strong><span>{entry.description}</span><small>目标完成比例 {entry.target} · 可调待验证</small></button>)}</div><div className="training-start"><label><span>Problem Set</span><select value={setId || ''} onChange={(event) => setSetId(Number(event.target.value))}><option value="" disabled>选择题单</option>{sets.map((set) => <option key={set.id} value={set.id}>{set.title} · {set.problems.length} 题</option>)}</select></label><label><span>时长（分钟）</span><input type="number" min={15} max={480} value={duration} onChange={(event) => setDuration(Number(event.target.value))} /></label><label><span>目标完成比例（%）</span><div className="target-rate"><input type="number" min={0} max={100} value={target[0]} onChange={(event) => setTarget([Number(event.target.value), target[1]])} /><b>–</b><input type="number" min={0} max={100} value={target[1]} onChange={(event) => setTarget([target[0], Number(event.target.value)])} /></div></label><button className="primary" disabled={loading || setId == null} onClick={() => void start()}><Play size={16} />开始 Match</button></div></section>
    <section className="training-match-list"><header><strong>训练记录</strong><small>{matches.length} 场</small></header>{matches.length ? matches.map((item) => <MatchCard key={item.id} item={item} onRefresh={() => void refreshMatch(item.id).then(() => notify('已根据本地同步记录更新')).catch((error) => notify(String(error)))} onFinish={() => void finishMatch(item.id).then(() => notify('训练赛已结束')).catch((error) => notify(String(error)))} />) : <div className="panel training-empty">还没有训练赛。选择题单和模式后开始第一场 Match。</div>}</section>
  </>;
}
