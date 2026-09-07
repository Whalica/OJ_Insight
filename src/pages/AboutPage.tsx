import { useEffect, useState } from 'react';
import { AlertTriangle, ExternalLink, Github, RefreshCw } from 'lucide-react';
import { api } from '../lib/api';
import { APP_VERSION } from '../lib/version';

export default function AboutPage() {
  const [update, setUpdate] = useState<Awaited<ReturnType<typeof api.checkForUpdates>> | null>(null);
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState('');
  const [storage, setStorage] = useState<Awaited<ReturnType<typeof api.storageInfo>> | null>(null);
  useEffect(() => { api.storageInfo().then(setStorage).catch(() => undefined); }, []);
  const check = async () => {
    setChecking(true); setError('');
    try { setUpdate(await api.checkForUpdates()); } catch (value) { setError(String(value)); } finally { setChecking(false); }
  };

  return <>
    <header className="topbar"><div><small>ABOUT</small><h1>关于 OJ Insight</h1><p>把分散在多个 Online Judge 的训练轨迹，整理成一份属于你的本地记录。</p></div></header>
    <section className="about-layout"><article className="panel about-card"><div className="about-mark">OI</div><h2>OJ Insight</h2><p>版本 {APP_VERSION} · Local-first</p>{update ? <div className={update.updateAvailable ? 'update-state available' : 'update-state'}><strong>{update.updateAvailable ? `发现新版本 · v${update.latestVersion}` : '当前已是最新版本'}</strong>{update.updateAvailable && <button onClick={() => api.openExternal(update.releaseUrl)}>查看发布页 <ExternalLink size={14} /></button>}</div> : <button className="primary" onClick={check} disabled={checking}><RefreshCw size={15} className={checking ? 'spin' : ''} />{checking ? '正在检查' : '检查更新'}</button>}{error && <div className="warning"><AlertTriangle size={15} />{error}</div>}<div className="about-links"><button onClick={() => api.openExternal('https://github.com/Whalica/OJ_Insight')}><Github size={16} />GitHub 仓库<ExternalLink size={13} /></button><button onClick={() => api.openExternal('https://github.com/Whalica/OJ_Insight/issues')}><AlertTriangle size={16} />Issue 反馈<ExternalLink size={13} /></button></div></article><article className="panel storage-card"><small>YOUR DATA</small><h2>数据放在哪里？</h2><p>OJ Insight 把账号设置、同步记录和导出文件集中放在应用自己的数据目录里。迁移电脑时，只需要一起复制这个目录。</p><div><strong>训练记录</strong><span>账号设置、提交记录、活动、Rating 与难度统计</span></div><div><strong>导出内容</strong><span>你生成的 PNG 与 SVG 文件</span></div><div><strong>运行记录</strong><span>遇到同步问题时用于排查，敏感凭据会脱敏</span></div>{storage && <code>{storage.rootDir}</code>}</article></section>
  </>;
}
