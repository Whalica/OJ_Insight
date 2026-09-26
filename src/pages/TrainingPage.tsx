import { useRef, useState } from 'react';
import { Download, FileUp, FolderOpen, Sparkles } from 'lucide-react';
import PlatformIcon from '../components/PlatformIcon';
import { PLATFORM_META } from '../lib/platforms';
import { api } from '../services/api';
import { openTrainingExportDirectory, saveTrainingFile } from '../services/training';
import type { CandidatePool, Platform, ProblemSetInput, TrainingMode } from '../types';

const MODES: Array<{ id: TrainingMode; name: string; target: string; description: string }> = [
  { id: 'relaxed', name: '轻松', target: '70%–90%', description: '稳定输出，保持题感' },
  { id: 'balanced', name: '均衡', target: '50%–70%', description: '正常训练，兼顾强项与上限' },
  { id: 'pressure', name: '压力', target: '35%–55%', description: '时间与选题压力，加入陌生性' },
];
const AUTO_PLATFORMS: Platform[] = ['codeforces', 'atcoder', 'qoj'];

export default function TrainingPage({ notify, onOpenProblemSets, onOpenContests }: { notify: (message: string) => void; onOpenProblemSets: () => void; onOpenContests: () => void }) {
  const [mode, setMode] = useState<TrainingMode>('balanced');
  const [platforms, setPlatforms] = useState<Platform[]>(AUTO_PLATFORMS);
  const [candidateCount, setCandidateCount] = useState(36);
  const [pool, setPool] = useState<CandidatePool | null>(null);
  const [generating, setGenerating] = useState(false);
  const [extra, setExtra] = useState('');
  const [savedPath, setSavedPath] = useState('');
  const [importTarget, setImportTarget] = useState<'set' | 'contest'>('contest');
  const fileRef = useRef<HTMLInputElement>(null);
  const generate = async () => { if (!platforms.length) return notify('至少选择一个平台'); setGenerating(true); try { const result = await api.generateTrainingCandidates(platforms, mode, candidateCount); setPool(result); notify(`候选池：${result.candidates.length} 道；排除已做 ${result.excludedSolved} 道`); } catch (error) { notify(String(error)); } finally { setGenerating(false); } };
  const exportPack = async () => { if (!pool) return; try { const bytes = await api.exportAiTrainingPack(pool.candidates, mode, extra); const saved = await saveTrainingFile(`OJ-Insight-AI组题包-${Date.now()}.zip`, bytes, 'zip'); if (saved) { setSavedPath(saved.path); notify(`组题包已保存到 ${saved.path}`); } } catch (error) { notify(String(error)); } };
  const importGenerated = async (file?: File) => {
    if (!file) return;
    try {
      const data = await file.text(); const value = JSON.parse(data);
      if (!['com.ojinsight.generated-contest', 'com.ojinsight.match-manifest', 'com.ojinsight.problem-set'].includes(value.schema) || value.schemaVersion !== 1) throw new Error('请选择 OJ Insight 生成的 JSON 文件');
      if (importTarget === 'contest') { const contest = await api.importGeneratedContest(data); notify(`比赛“${contest.title}”已导入`); onOpenContests(); }
      else { const input: ProblemSetInput = { id: null, title: value.title, description: value.description || '', setType: 'static', tagVisibility: value.tagVisibility || 'after_ac', sourceSetId: null, sourceUrl: null, problems: value.problems }; const set = await api.saveProblemSet(input); notify(`题单“${set.title}”已导入`); onOpenProblemSets(); }
    } catch (error) { notify(String(error)); } finally { if (fileRef.current) fileRef.current.value = ''; }
  };
  return <>
    <header className="topbar"><div><small>PERSONALIZED TRAINING</small><h1>个性化组题</h1><p>先由 OJ Insight 从可靠目录初筛，再让大模型在候选池中复筛和编排。上传一个 ZIP 即可，无需额外提示词。</p></div><div className="compact-actions"><button title="导入大模型生成的 JSON 文件并保存为比赛" onClick={() => { setImportTarget('contest'); fileRef.current?.click(); }}><FileUp size={14} />导入为比赛</button><button title="导入大模型生成的 JSON 文件并保存为题单" onClick={() => { setImportTarget('set'); fileRef.current?.click(); }}><FileUp size={14} />导入为题单</button><input ref={fileRef} hidden type="file" accept="application/json,.json" onChange={(event) => void importGenerated(event.target.files?.[0])} /></div></header>
    <section className="panel training-builder"><header className="training-section-title"><div><small>TRAINING PACK</small><h2>生成候选池</h2><p>训练模式是节奏与压力目标；完成比例为可调建议。</p></div><Sparkles size={22} /></header>
      <div className="training-modes">{MODES.map((entry) => <button key={entry.id} className={mode === entry.id ? 'active' : ''} onClick={() => { setMode(entry.id); setPool(null); }}><strong>{entry.name}</strong><span>{entry.description}</span><small>参考比例 {entry.target}</small></button>)}</div>
      <div className="candidate-controls"><div><span>初筛平台</span><div className="platform-picks">{AUTO_PLATFORMS.map((platform) => <button key={platform} className={platforms.includes(platform) ? 'active' : ''} onClick={() => { setPlatforms((current) => current.includes(platform) ? current.filter((item) => item !== platform) : [...current, platform]); setPool(null); }}><PlatformIcon platform={platform} />{PLATFORM_META[platform].name}</button>)}</div><small>洛谷、牛客和 LeetCode 暂无足够可靠的完整目录。</small></div><label><span>候选池大小</span><input type="number" min={12} max={120} value={candidateCount} onChange={(event) => { setCandidateCount(Number(event.target.value)); setPool(null); }} /></label><button className="primary generate-candidates" disabled={generating} onClick={() => void generate()}><Sparkles size={16} />{generating ? '正在筛题…' : '生成候选池'}</button></div>
      {pool && <div className="candidate-result"><header><div><strong>候选池已准备好</strong><span>{pool.candidates.length} 道可用题 · 排除已做 {pool.excludedSolved} 道</span></div></header><p>{pool.selectionBasis}</p><div className="source-statuses">{pool.sources.map((source) => <span key={source.platform} className={source.available ? 'ok' : 'unavailable'}><i />{PLATFORM_META[source.platform].name} · {source.available ? `${source.problemCount} 道目录题` : source.message}</span>)}</div><details><summary>查看候选题</summary><div className="candidate-preview">{pool.candidates.slice(0, 30).map((problem) => <a key={problem.canonicalId} href={problem.url} target="_blank" rel="noreferrer"><PlatformIcon platform={problem.platform} /><span><strong>{problem.name}</strong><small>{problem.problemId}</small></span></a>)}</div></details><label className="training-extra"><span>额外组题要求（可选）</span><textarea value={extra} onChange={(event) => setExtra(event.target.value)} placeholder="留空即可直接把 ZIP 交给大模型" /></label><div className="training-pack-actions"><button className="primary" onClick={() => void exportPack()}><Download size={15} />导出组题包 ZIP</button></div><p>模型返回一个 JSON；选择导入为比赛或可复用题单。</p>{savedPath && <div className="export-result"><span><strong>最近导出</strong>{savedPath}</span><button onClick={() => void openTrainingExportDirectory(savedPath)}><FolderOpen size={14} />打开所在文件夹</button></div>}</div>}
    </section>
  </>;
}
