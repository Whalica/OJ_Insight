import { AlertTriangle, Check, RefreshCw, Trash2 } from 'lucide-react';
import { api } from '../lib/api';
import { formatDateTime } from '../lib/date';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform, SyncStatus } from '../types';

interface Props {
  statuses: SyncStatus[];
  syncing: string | null;
  timeZone: string;
  onSync: (platform: Platform, full?: boolean) => void;
  onSyncAll: () => void;
  onCleared: () => void | Promise<void>;
  notify: (message: string) => void;
}

function syncStatusLabel(status?: string) {
  return ({ idle: '未同步', syncing: '同步中', ok: '同步成功', warning: '部分成功', error: '同步失败', auth_required: '需要重新登录' } as Record<string, string>)[status || 'idle'] || status || '未同步';
}

export default function DataPage({ statuses, syncing, timeZone, onSync, onSyncAll, onCleared, notify }: Props) {
  const by = new Map(statuses.map((status) => [status.platform, status]));
  const clearOne = async (platform: Platform) => {
    if (!confirm(`清空 ${PLATFORM_META[platform].name} 的全部本地记录？账号设置会保留。`)) return;
    try { await api.clearPlatform(platform); notify('已清空'); await onCleared(); } catch (error) { notify(String(error)); }
  };
  const clearAll = async () => {
    if (!confirm('清空所有 OJ 的本地记录？账号设置会保留。')) return;
    try { await api.clearAll(); notify('所有本地记录已清空'); await onCleared(); } catch (error) { notify(String(error)); }
  };

  return <>
    <header className="topbar"><div><small>SYNC & LOCAL DATA</small><h1>同步与数据</h1><p>同步失败不会删除旧记录；“重建”用于重新读取该平台的完整历史。</p></div><button className="primary" onClick={() => onSyncAll()} disabled={!!syncing}><RefreshCw size={16} className={syncing ? 'spin' : ''} />同步全部</button></header>
    <section className="panel source-list">{PLATFORM_ORDER.map((platform) => {
      const status = by.get(platform);
      const needsAttention = status?.status === 'warning' || status?.status === 'error' || status?.status === 'auth_required';
      return <article key={platform}><div className="source-id"><span className="platform-monogram" style={{ color: PLATFORM_META[platform].accent }}>{PLATFORM_META[platform].short}</span><div><strong>{PLATFORM_META[platform].name}</strong><small>{status?.account || '未配置账号'} · 缓存 {status?.cached_records || 0} 条</small></div></div><div className={`source-state ${status?.status || 'idle'}`}>{status?.status === 'ok' ? <Check size={15} /> : needsAttention ? <AlertTriangle size={15} /> : null}<div><strong>{syncStatusLabel(status?.status)}</strong><small>{status?.message || '尚未同步'} · 上次成功 {formatDateTime(status?.last_success || null, timeZone)}</small></div></div><div className="source-actions"><button onClick={() => onSync(platform)} disabled={!!syncing}><RefreshCw size={14} />增量</button><button onClick={() => onSync(platform, true)} disabled={!!syncing}>重建</button><button className="danger-ghost" disabled={!!syncing} onClick={() => clearOne(platform)}><Trash2 size={14} />清空</button></div></article>;
    })}</section>
    <section className="danger-zone"><div><strong>清空全部训练数据</strong><p>会删除提交、活动砖、难度、Rating 和同步状态，账号设置不会被删除。</p></div><button disabled={!!syncing} onClick={clearAll}><Trash2 size={15} />清空全部</button></section>
  </>;
}
