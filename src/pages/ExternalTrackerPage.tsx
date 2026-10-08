import { ExternalLink, RefreshCw, UserRound } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../services/api';
import type { AccountConfig } from '../types';

export type ExternalTracker = 'codeforces' | 'atcoder';

const META: Record<ExternalTracker, { name: string; url: string }> = {
  codeforces: { name: 'Codeforces Tracker', url: 'https://cftracker.netlify.app/#/contests' },
  atcoder: { name: 'AtCoder Problems', url: 'https://kenkoooo.com/atcoder/#/table' },
};

export default function ExternalTrackerPage({ tracker, accounts }: { tracker: ExternalTracker; accounts: AccountConfig[] }) {
  const meta = META[tracker];
  const entry = accounts.find((item) => item.account.trim());
  const account = entry?.account.trim() || '';
  const url = useMemo(() => {
    if (tracker === 'atcoder' && account) return `${meta.url}/${encodeURIComponent(account)}`;
    if (tracker === 'codeforces' && account) return `https://cftracker.netlify.app/?oji_handle=${encodeURIComponent(account)}#/contests`;
    return meta.url;
  }, [tracker, account, meta.url]);
  const [frameUrl, setFrameUrl] = useState(url);
  const [reloadKey, setReloadKey] = useState(0);
  useEffect(() => {
    const origins = tracker === 'codeforces'
      ? new Set(['https://cftracker.netlify.app'])
      : new Set(['https://kenkoooo.com']);
    const receive = (event: MessageEvent) => {
      if (!origins.has(event.origin) || event.data?.type !== 'oj-insight:open-external') return;
      try {
        const target = new URL(String(event.data.url));
        if (target.protocol === 'https:' || target.protocol === 'http:') void api.openExternal(target.href);
      } catch { /* Ignore malformed messages from embedded pages. */ }
    };
    window.addEventListener('message', receive);
    return () => window.removeEventListener('message', receive);
  }, [tracker]);
  useEffect(() => { setFrameUrl(url); }, [url]);

  const note = tracker === 'codeforces' && account ? `已将设置中的 Codeforces 用户名 ${account} 自动填入。`
        : tracker === 'atcoder' && account ? `已将设置中的 AtCoder 用户名 ${account} 自动带入。`
          : account ? `已读取设置中的账号 ${account}。` : '尚未在 OJ Insight 设置账号，也可以直接在下方网页中填写。';
  return <section className="embedded-tracker-page">
    <header className="embedded-tracker-head">
      <div className="embedded-tracker-title"><div><h1>{meta.name}</h1><small>第三方进度页 · 已嵌入 OJ Insight</small></div></div>
      <div className="embedded-tracker-actions">{account && <span className="embedded-account"><UserRound size={14} />{account}</span>}<button onClick={() => setReloadKey((value) => value + 1)}><RefreshCw size={14} />刷新</button><button onClick={() => void api.openExternal(meta.url)}><ExternalLink size={14} />浏览器打开</button></div>
    </header>
    <div className="embedded-tracker-note">{note}</div>
    <div className="embedded-tracker-frame"><iframe key={`${frameUrl}-${reloadKey}`} src={frameUrl} title={meta.name} /></div>
  </section>;
}
