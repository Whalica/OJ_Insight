import { ExternalLink, RefreshCw, UserRound } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { api } from '../lib/api';
import type { AccountConfig } from '../types';

export type ExternalTracker = 'codeforces' | 'atcoder' | 'nowcoder';

const META: Record<ExternalTracker, { name: string; url: string }> = {
  codeforces: { name: 'Codeforces Tracker', url: 'https://cftracker.netlify.app/#/contests' },
  atcoder: { name: 'AtCoder Problems', url: 'https://kenkoooo.com/atcoder/#/table' },
  nowcoder: { name: '牛客练习记录', url: 'https://www.nowcoder.com/problem/tracker' },
};

export default function ExternalTrackerPage({ tracker, accounts }: { tracker: ExternalTracker; accounts: AccountConfig[] }) {
  const meta = META[tracker];
  const entry = accounts.find((item) => item.account.trim());
  const account = entry?.account.trim() || '';
  const secret = entry?.secret.trim() || '';
  const url = useMemo(() => {
    if (tracker === 'atcoder' && account) return `${meta.url}/${encodeURIComponent(account)}`;
    if (tracker === 'codeforces' && account) return `https://cftracker.netlify.app/?oji_handle=${encodeURIComponent(account)}#/contests`;
    return meta.url;
  }, [tracker, account, meta.url]);
  const [reloadKey, setReloadKey] = useState(0);
  const [sessionReady, setSessionReady] = useState(tracker !== 'nowcoder');
  const [sessionError, setSessionError] = useState('');
  useEffect(() => {
    let active = true;
    setSessionError('');
    if (tracker !== 'nowcoder' || !secret) { setSessionReady(true); return () => { active = false; }; }
    setSessionReady(false);
    void api.prepareTrackerSession(tracker, secret)
      .catch((error) => { if (active) setSessionError(String(error)); })
      .finally(() => { if (active) setSessionReady(true); });
    return () => { active = false; };
  }, [tracker, secret, reloadKey]);

  const note = sessionError
    ? sessionError
    : tracker === 'nowcoder' && secret ? '已将设置中的 Cookie 写入内嵌网页会话。若 Cookie 已过期，请在设置中更新。'
      : tracker === 'codeforces' && account ? `已将设置中的 Codeforces 用户名 ${account} 自动填入。`
        : tracker === 'atcoder' && account ? `已将设置中的 AtCoder 用户名 ${account} 自动带入。`
          : account ? `已读取设置中的账号 ${account}。` : '尚未在 OJ Insight 设置账号，也可以直接在下方网页中填写或登录。';
  return <section className="embedded-tracker-page">
    <header className="embedded-tracker-head">
      <div className="embedded-tracker-title"><div><h1>{meta.name}</h1><small>第三方进度页 · 已嵌入 OJ Insight</small></div></div>
      <div className="embedded-tracker-actions">{account && <span className="embedded-account"><UserRound size={14} />{account}</span>}<button onClick={() => setReloadKey((value) => value + 1)}><RefreshCw size={14} />刷新</button><button onClick={() => void api.openExternal(meta.url)}><ExternalLink size={14} />浏览器打开</button></div>
    </header>
    <div className="embedded-tracker-note">{note}</div>
    <div className="embedded-tracker-frame">{sessionReady && <iframe key={`${url}-${reloadKey}`} src={url} title={meta.name} />}</div>
  </section>;
}
