import { ExternalLink, RefreshCw, UserRound } from 'lucide-react';
import { useMemo, useState } from 'react';
import { api } from '../lib/api';
import type { AccountConfig } from '../types';

export type ExternalTracker = 'codeforces' | 'atcoder' | 'nowcoder';

const META: Record<ExternalTracker, { name: string; short: string; url: string; accent: string }> = {
  codeforces: { name: 'Codeforces Tracker', short: 'CF', url: 'https://cftracker.netlify.app/contests', accent: '#5aa6e8' },
  atcoder: { name: 'AtCoder Problems', short: 'ATC', url: 'https://kenkoooo.com/atcoder/#/table', accent: 'var(--atcoder-accent)' },
  nowcoder: { name: 'NowCoder Tracker', short: 'NC', url: 'https://www.nowcoder.com/problem/tracker', accent: '#00b96b' },
};

export default function ExternalTrackerPage({ tracker, accounts }: { tracker: ExternalTracker; accounts: AccountConfig[] }) {
  const meta = META[tracker];
  const account = accounts.find((entry) => entry.account.trim())?.account.trim() || '';
  const url = useMemo(() => tracker === 'atcoder' && account ? `${meta.url}/${encodeURIComponent(account)}` : meta.url, [tracker, account, meta.url]);
  const [reloadKey, setReloadKey] = useState(0);
  return <section className="embedded-tracker-page">
    <header className="embedded-tracker-head">
      <div className="embedded-tracker-title"><span style={{ color: meta.accent }}>{meta.short}</span><div><small>TRACKER · EMBEDDED</small><h1>{meta.name}</h1></div></div>
      <div className="embedded-tracker-actions">{account && <span className="embedded-account"><UserRound size={14} />{account}</span>}<button onClick={() => setReloadKey((value) => value + 1)}><RefreshCw size={14} />刷新</button><button onClick={() => void api.openExternal(meta.url)}><ExternalLink size={14} />浏览器打开</button></div>
    </header>
    <div className="embedded-tracker-note">{tracker === 'atcoder' && account ? `已把设置中的 AtCoder 用户名 ${account} 自动带入。` : account ? `已识别设置中的账号 ${account}。首次填写或登录后，内嵌网页会记住状态。` : '尚未在 OJ Insight 设置账号；也可以直接在下方 Tracker 中填写或登录。'}</div>
    <div className="embedded-tracker-frame"><iframe key={`${url}-${reloadKey}`} src={url} title={meta.name} /></div>
  </section>;
}
