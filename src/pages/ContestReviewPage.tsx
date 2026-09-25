import { useEffect, useMemo, useState } from 'react';
import { save } from '@tauri-apps/plugin-dialog';
import { AlertTriangle, Archive, CheckCircle2, ExternalLink, FileSearch, LoaderCircle, Settings, ShieldCheck } from 'lucide-react';

import { PLATFORM_META } from '../lib/platforms';
import { api } from '../services/api';
import type { ContestReviewPreview, Platform } from '../types';
import type { AccountMap } from '../lib/ui';

const SUPPORTED: Platform[] = ['atcoder'];

function safeName(value: string) {
  return value.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-').replace(/\s+/g, '-').replace(/-+/g, '-').slice(0, 80);
}

export default function ContestReviewPage({ accounts, notify, onOpenSettings, embedded = false }: { accounts: AccountMap; notify: (message: string) => void; onOpenSettings: () => void; embedded?: boolean }) {
  const [platform, setPlatform] = useState<Platform>('atcoder');
  const [account, setAccount] = useState('');
  const [contestInput, setContestInput] = useState('');
  const [includePostContest, setIncludePostContest] = useState(false);
  const [preview, setPreview] = useState<ContestReviewPreview | null>(null);
  const [checking, setChecking] = useState(false);
  const [generating, setGenerating] = useState(false);
  const platformAccounts = accounts[platform] || [];
  const supported = SUPPORTED.includes(platform);
  const canCheck = supported && !!account.trim() && !!contestInput.trim() && !checking && !generating;
  const capability = useMemo(() => supported ? '当前支持 · 自动整理题目、提交与可获取的源代码' : '统一流程已预留，当前版本暂未接入该 OJ', [supported]);

  useEffect(() => {
    setAccount((current) => platformAccounts.some((entry) => entry.account === current) ? current : platformAccounts[0]?.account || '');
    setPreview(null);
  }, [platform, platformAccounts]);
  useEffect(() => { setPreview(null); }, [account, contestInput, includePostContest]);

  const inspect = async () => {
    if (!canCheck) return;
    setChecking(true);
    try {
      const result = await api.inspectContestReview(platform, account, contestInput.trim(), includePostContest);
      setPreview(result);
      notify(`已识别 ${result.contestName}`);
    } catch (error) { notify(String(error)); }
    finally { setChecking(false); }
  };

  const generate = async () => {
    if (!preview || generating) return;
    const filename = `OJ-Insight-Review-${PLATFORM_META[platform].short}-${safeName(preview.contestId)}-${safeName(account)}.zip`;
    const path = await save({ defaultPath: filename, filters: [{ name: 'ZIP 复盘包', extensions: ['zip'] }] });
    if (!path) return;
    setGenerating(true);
    try {
      const result = await api.generateContestReview(platform, account, contestInput.trim(), includePostContest, path);
      notify(`复盘包已生成：${result.submissionCount} 次提交，${result.codeCount} 份代码`);
      setPreview((current) => current ? { ...current, completeness: result.completeness, notes: result.notes } : current);
    } catch (error) { notify(String(error)); }
    finally { setGenerating(false); }
  };

  return <>
    {!embedded && <header className="topbar"><div><small>CONTEST REVIEW PACKAGE</small><h1>比赛复盘</h1><p>把比赛、提交与代码整理成统一的四文档压缩包，交给大模型后即可直接开始复盘。</p></div></header>}
    <section className="review-builder">
      <div className="panel review-form">
        <header><div><small>STEP 1</small><h2>选择比赛</h2></div><span><ShieldCheck size={14} />凭据不会写入复盘包</span></header>
        <label><span>OJ</span><div className="review-platforms">{SUPPORTED.map((item) => <button key={item} type="button" className={platform === item ? 'active' : ''} onClick={() => setPlatform(item)}><i style={{ background: PLATFORM_META[item].accent }} />{PLATFORM_META[item].name}</button>)}</div></label>
        <p className={`review-capability ${supported ? '' : 'unsupported'}`}>{supported ? <CheckCircle2 size={14} /> : <AlertTriangle size={14} />}{capability}</p>
        <div className="review-fields">
          <label><span>账号</span><select value={account} onChange={(event) => setAccount(event.target.value)} disabled={!platformAccounts.length}><option value="">{platformAccounts.length ? '选择账号' : '请先在设置中配置账号'}</option>{platformAccounts.map((entry) => <option key={entry.account} value={entry.account}>{entry.account}</option>)}</select></label>
          <label><span>比赛 ID 或链接</span><input value={contestInput} onChange={(event) => setContestInput(event.target.value)} placeholder="例如 abc380 或比赛链接" disabled={!supported} /></label>
        </div>
        <div className="review-credential-guide"><div><strong>复盘账号</strong><span>在设置中确认复盘使用的 AtCoder 账号。</span></div><button type="button" onClick={onOpenSettings}><Settings size={14} />前往配置</button></div>
        <label className="review-post-option"><input type="checkbox" checked={includePostContest} onChange={(event) => setIncludePostContest(event.target.checked)} /><i /><span><strong>包含赛后补题</strong><small>与正式比赛提交分开标记，不影响比赛过程判断</small></span></label>
        <button className="primary review-check" disabled={!canCheck} onClick={() => void inspect()}>{checking ? <LoaderCircle className="spin" size={16} /> : <FileSearch size={16} />}{checking ? '正在检查比赛…' : '检查比赛'}</button>
      </div>

      <div className={`panel review-preview ${preview ? 'ready' : ''}`}>
        {!preview ? <div className="review-empty"><Archive size={30} /><strong>等待检查比赛</strong><span>确认能取得的题目、提交和代码后，再选择保存位置。</span></div> : <>
          <header><div><small>STEP 2</small><h2>{preview.contestName}</h2><p>{PLATFORM_META[preview.platform].name} · {preview.account} · {preview.contestId}</p></div><div className="review-preview-actions"><button type="button" onClick={() => void api.openExternal(preview.contestUrl)}>打开比赛<ExternalLink size={13} /></button><span className={preview.completeness === 'complete' ? 'complete' : 'partial'}>{preview.completeness === 'complete' ? '数据可用' : '部分可用'}</span></div></header>
          <div className="review-stats"><div><small>题目</small><strong>{preview.problemCount}</strong></div><div><small>提交</small><strong>{preview.submissionCount}</strong></div><div><small>代码</small><strong>{preview.codeAvailable ? '生成时获取' : '不可用'}</strong></div></div>
          {preview.notes.length > 0 && <div className="review-notes">{preview.notes.map((note) => <p key={note}><AlertTriangle size={13} />{note}</p>)}</div>}
          <div className="review-package"><strong>固定四文档</strong><span>00-START-HERE · 01-CONTEST · 02-PROBLEMS · 03-SUBMISSIONS</span><small>上传后无需解释；若聊天平台要求输入文字，只需说“开始复盘”。</small></div>
          <button className="primary" disabled={generating} onClick={() => void generate()}>{generating ? <LoaderCircle className="spin" size={16} /> : <Archive size={16} />}{generating ? '正在获取代码并打包…' : '生成复盘包'}</button>
        </>}
      </div>
    </section>
  </>;
}
