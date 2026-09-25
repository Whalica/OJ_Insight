import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/plugin-dialog';
import { Download, FileCode2, RefreshCw, Trash2 } from 'lucide-react';
import ContestReviewPage from './ContestReviewPage';
import MarkdownNote from '../components/MarkdownNote';
import PlatformIcon from '../components/PlatformIcon';
import { api } from '../services/api';
import { saveTrainingFile } from '../services/training';
import type { TrainingMatch, VpSubmission } from '../types';
import type { AccountMap } from '../lib/ui';

const verdictName = (value: string) => ({ OK: 'AC', WRONG_ANSWER: 'WA', TIME_LIMIT_EXCEEDED: 'TLE', MEMORY_LIMIT_EXCEEDED: 'MLE', RUNTIME_ERROR: 'RE', COMPILATION_ERROR: 'CE' } as Record<string, string>)[value] || value;

function ReviewCard({ item, notify, reload }: { item: TrainingMatch; notify: (message: string) => void; reload: () => Promise<void> }) {
  const [submissions, setSubmissions] = useState<VpSubmission[]>([]);
  const [codeFiles, setCodeFiles] = useState<[number, string][]>([]);
  useEffect(() => { void api.listVpSubmissions(item.id).then(setSubmissions).catch(() => undefined); }, [item.id]);
  useEffect(() => { void api.listVpCodeFiles(item.id).then(setCodeFiles).catch(() => undefined); }, [item.id]);
  const refresh = async () => { try { setSubmissions(await api.refreshVpSubmissions(item.id)); await api.refreshTrainingMatch(item.id); await reload(); notify('提交记录已更新'); } catch (error) { notify(String(error)); } };
  const bind = async (position: number) => { try { const path = await open({ multiple: false, title: '选择本地代码文件' }); if (!path || typeof path !== 'string') return; await api.bindVpCode(item.id, position, path); setCodeFiles(await api.listVpCodeFiles(item.id)); notify('代码已绑定到复盘记录'); } catch (error) { notify(String(error)); } };
  const exportPack = async () => { try { const bytes = await api.exportVpReviewPack(item.id); const saved = await saveTrainingFile(`OJ-Insight-VP复盘-${item.id}.zip`, bytes, 'zip'); if (saved) notify(`复盘包已保存到 ${saved.path}`); } catch (error) { notify(String(error)); } };
  const remove = async () => { if (!confirm(`确定删除比赛记录“${item.title}”及笔记、代码绑定？`)) return; try { await api.deleteTrainingMatch(item.id); await reload(); notify('记录已删除'); } catch (error) { notify(String(error)); } };
  return <article className="panel vp-review-card"><header><div><small>VP · 已结束</small><h2>{item.title}</h2><p>{item.problems.filter((p) => p.solved).length}/{item.problems.length} 题 AC · {new Date((item.endedAt || item.createdAt) * 1000).toLocaleString()}</p></div><div className="icon-actions"><button title="更新可用提交" onClick={() => void refresh()}><RefreshCw size={15} /></button><button title="导出复盘包" onClick={() => void exportPack()}><Download size={15} /></button><button title="删除记录" onClick={() => void remove()}><Trash2 size={15} /></button></div></header><details><summary>查看题目、提交与笔记</summary><div className="vp-review-body"><h3>整场笔记</h3><MarkdownNote value={item.generalNote} onSave={(text) => api.saveVpNote(item.id, null, text)} placeholder="记录整场总评" />{item.problems.map((entry) => { const rows = submissions.filter((submission) => submission.platform === entry.problem.platform && submission.problemKey === entry.problem.problemKey); const attached = codeFiles.filter(([position]) => position === entry.position); return <section key={entry.position}><h4><PlatformIcon platform={entry.problem.platform} />{entry.position + 1}. {entry.problem.name || entry.problem.problemId} · {entry.solved ? 'AC' : '未 AC'}</h4><div className="vp-review-submissions">{rows.map((submission) => <a href={submission.sourceUrl} target="_blank" rel="noreferrer" key={`${submission.sourceUrl}-${submission.submittedAt}`}>{new Date(submission.submittedAt * 1000).toLocaleTimeString()} · {verdictName(submission.verdict)}</a>)}{rows.length === 0 && <small>暂无可获取的赛时提交；本地同步仅记录 AC。</small>}</div><button onClick={() => void bind(entry.position)}><FileCode2 size={14} />绑定本地代码</button>{attached.length > 0 && <small className="vp-attached-code">已绑定：{attached.map(([, name]) => name).join('、')}</small>}<MarkdownNote value={entry.solutionNote} onSave={(text) => api.saveVpNote(item.id, entry.position, text)} placeholder="补充题目思路" /></section>; })}</div></details><footer><button className="primary" onClick={() => void exportPack()}><Download size={15} />导出给大模型的复盘包</button></footer></article>;
}

export default function TrainingReviewPage({ accounts, notify, onOpenSettings }: { accounts: AccountMap; notify: (message: string) => void; onOpenSettings: () => void }) {
  const [matches, setMatches] = useState<TrainingMatch[]>([]);
  const [tab, setTab] = useState<'vp' | 'official'>('vp');
  const reload = async () => setMatches(await api.listTrainingMatches());
  useEffect(() => { void reload().catch((error) => notify(String(error))); }, [notify]);
  return <><header className="topbar"><div><small>POST CONTEST</small><h1>赛后分析</h1><p>复盘 VP 或已有的 AtCoder 正式比赛。能获取的提交显示 verdict；缺失的错误提交不会凭空补齐。</p></div></header><div className="review-tabs"><button className={tab === 'vp' ? 'active' : ''} onClick={() => setTab('vp')}>VP 复盘</button><button className={tab === 'official' ? 'active' : ''} onClick={() => setTab('official')}>正式比赛复盘</button></div>{tab === 'vp' ? <div className="vp-review-list">{matches.filter((item) => item.status === 'finished').map((item) => <ReviewCard key={item.id} item={item} notify={notify} reload={reload} />)}{matches.every((item) => item.status !== 'finished') && <div className="panel training-empty">结束一场 VP 后，复盘会出现在这里。</div>}</div> : <ContestReviewPage accounts={accounts} notify={notify} onOpenSettings={onOpenSettings} embedded />}</>;
}
