import PlatformIcon from '../components/PlatformIcon';
import { useEffect, useState } from 'react';
import { Monitor, Palette, Plus, RotateCcw, Save, Type, X } from 'lucide-react';
import { api } from '../services/api';
import { TIME_ZONE_OPTIONS, timeZoneLabel } from '../lib/date';
import { parseCodeforcesCredentials, PLATFORM_META, PLATFORM_ORDER, serializeCodeforcesCredentials, type CodeforcesCredentials } from '../lib/platforms';
import { emptyAccounts, type AccountMap } from '../lib/ui';
import { DEFAULT_PREFERENCES, type Preferences } from '../lib/preferences';
import type { Platform, SyncStatus } from '../types';
import PersonalInfoExport from '../components/PersonalInfoExport';
import PersonalInfoImport from '../components/PersonalInfoImport';
import DatabaseBackup from '../components/DatabaseBackup';
import QojCookieImport from '../components/QojCookieImport';
import DataPage from './DataPage';
import { useI18n } from '../lib/i18n';

interface Props {
  accounts: AccountMap;
  timeZone: string;
  onTimeZone: (value: string) => void;
  preferences: Preferences;
  onPreferences: (patch: Partial<Preferences>) => void;
  onSaved: () => void | Promise<void>;
  syncing: string | null;
  notify: (message: string) => void;
  statuses: SyncStatus[];
  onSync: (platform: Platform, full?: boolean) => void;
  onSyncAll: () => void;
  onCleared: () => void | Promise<void>;
}

export default function SettingsPage({ accounts, timeZone, onTimeZone, preferences, onPreferences, onSaved, syncing, notify, statuses, onSync, onSyncAll, onCleared }: Props) {
  const { t } = useI18n();
  const [tab, setTab] = useState<'accounts' | 'personalization' | 'data'>('accounts');
  const [form, setForm] = useState<AccountMap>(emptyAccounts);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    const next = emptyAccounts();
    for (const platform of PLATFORM_ORDER) next[platform] = accounts[platform].length ? accounts[platform].map((entry) => ({ ...entry })) : [{ platform, account: '', secret: '' }];
    setForm(next);
  }, [accounts]);
  const change = (platform: Platform, index: number, key: 'account' | 'secret', value: string) => setForm((current) => ({ ...current, [platform]: current[platform].map((entry, row) => row === index ? { ...entry, [key]: value } : entry) }));
  const changeCodeforcesCredential = (index: number, key: keyof CodeforcesCredentials, value: string) => setForm((current) => ({
    ...current,
    codeforces: current.codeforces.map((entry, row) => row === index
      ? { ...entry, secret: serializeCodeforcesCredentials({ ...parseCodeforcesCredentials(entry.secret), [key]: value }) }
      : entry),
  }));
  const add = (platform: Platform) => setForm((current) => ({ ...current, [platform]: [...current[platform], { platform, account: '', secret: '' }] }));
  const remove = (platform: Platform, index: number) => setForm((current) => { const rows = current[platform].filter((_, row) => row !== index); return { ...current, [platform]: rows.length ? rows : [{ platform, account: '', secret: '' }] }; });
  const save = async () => {
    const next = PLATFORM_ORDER.flatMap((platform) => form[platform].filter((entry) => entry.account.trim()));
    for (const platform of PLATFORM_ORDER) {
      const ids = next.filter((entry) => entry.platform === platform).map((entry) => entry.account.trim());
      if (new Set(ids).size !== ids.length) { notify(t('{platform} 中存在重复 ID，请合并后保存', { platform: PLATFORM_META[platform].name })); return; }
    }
    const removed = PLATFORM_ORDER.flatMap((platform) => accounts[platform].filter((entry) => !next.some((value) => value.platform === platform && value.account.trim() === entry.account)).map((entry) => `${PLATFORM_META[platform].name} · ${entry.account}`));
    if (removed.length && !confirm(t('将移除以下 ID 及其本地提交、活动、难度、Rating 和同步记录：\n{ids}\n\n其他 ID 的记录会保留。确认保存？', { ids: removed.join('\n') }))) return;
    setSaving(true);
    try { await api.saveAllAccounts(next); await onSaved(); } catch (error) { notify(String(error)); } finally { setSaving(false); }
  };
  const choices = <T extends string>(value: T, items: Array<[T, string]>, changeValue: (value: T) => void) => <div className="preference-choices">{items.map(([key, label]) => <button className={value === key ? 'active' : ''} onClick={() => changeValue(key)} key={key}>{t(label)}</button>)}</div>;

  return <>
    <header className="topbar"><div><small lang="en">SETTINGS</small><h1>{t('设置')}</h1><p>{t('管理平台账号和本机显示偏好。')}</p></div>{tab === 'accounts' && <button className="primary" onClick={save} disabled={saving || !!syncing}><Save size={16} />{saving ? t('保存中') : t('保存账号')}</button>}</header>
    <div className="settings-tabs"><button className={tab === 'accounts' ? 'active' : ''} onClick={() => setTab('accounts')}>{t('账号设置')}</button><button className={tab === 'personalization' ? 'active' : ''} onClick={() => setTab('personalization')}>{t('个性化')}</button><button className={tab === 'data' ? 'active' : ''} onClick={() => setTab('data')}>{t('数据源')}</button></div>
    {tab === 'accounts' && <QojCookieImport accounts={form.qoj} onCookie={(index, cookie) => change('qoj', index, 'secret', cookie)} notify={notify} />}
    {tab === 'accounts' ? <><section className="settings-intro"><strong>{t('怎么填写？')}</strong><span>{t('填写个人主页 URL 中的账号标识，不是显示昵称。Cookie 与 API 凭据仅保存在本机；获取入口位于对应 OJ 页的同步按钮左侧。删除或改名会在保存时清理原 ID 的本地记录。')}</span></section><section className="account-panel">{PLATFORM_ORDER.map((platform) => <article className="account-card" key={platform}><div className="account-card-head"><div><PlatformIcon platform={platform} /><div><strong lang="en">{PLATFORM_META[platform].name}</strong><small>{t(PLATFORM_META[platform].accountHint)}</small></div></div><div className="account-card-actions"><button onClick={() => add(platform)}><Plus size={14} />{t('添加 ID')}</button></div></div><div className="account-inputs">{form[platform]?.map((entry, index) => platform === 'codeforces' ? (() => { const credentials = parseCodeforcesCredentials(entry.secret); return <div className="account-entry account-entry-cf" key={index}><span>{index + 1}</span><input aria-label={`Codeforces ID ${index + 1}`} value={entry.account} onChange={(event) => change(platform, index, 'account', event.target.value)} placeholder={t(PLATFORM_META[platform].accountHint)} /><input aria-label={`Codeforces API Key ${index + 1}`} type="password" value={credentials.apiKey} onChange={(event) => changeCodeforcesCredential(index, 'apiKey', event.target.value)} placeholder={t('API Key（可选）')} autoComplete="off" /><input aria-label={`Codeforces API Secret ${index + 1}`} type="password" value={credentials.apiSecret} onChange={(event) => changeCodeforcesCredential(index, 'apiSecret', event.target.value)} placeholder={t('API Secret（可选）')} autoComplete="off" /><button className="remove-account" onClick={() => remove(platform, index)} aria-label={t('移除账号')}><X size={14} /></button></div>; })() : <div className="account-entry" key={index}><span>{index + 1}</span><input aria-label={`${PLATFORM_META[platform].name} ID ${index + 1}`} value={entry.account} onChange={(event) => change(platform, index, 'account', event.target.value)} placeholder={t(PLATFORM_META[platform].accountHint)} />{PLATFORM_META[platform].secretHint ? <input aria-label={`${PLATFORM_META[platform].name} Cookie ${index + 1}`} type="password" value={entry.secret} onChange={(event) => change(platform, index, 'secret', event.target.value)} placeholder={t(PLATFORM_META[platform].secretHint)} autoComplete="off" /> : <small>{t('公开数据，无需密码')}</small>}<button className="remove-account" onClick={() => remove(platform, index)} aria-label={t('移除账号')}><X size={14} /></button></div>)}</div></article>)}</section><PersonalInfoImport accounts={PLATFORM_ORDER.flatMap((platform) => form[platform]).filter((entry) => entry.account.trim())} onImported={onSaved} notify={notify} /><PersonalInfoExport accounts={PLATFORM_ORDER.flatMap((platform) => form[platform])} notify={notify} /><DatabaseBackup notify={notify} /></> : <section className="preferences-panel">
      <article className="panel preference-card"><div className="preference-heading"><Monitor size={18} /><div><small lang="en">LANGUAGE</small><h2>{t('界面语言')} <span className="language-status">{t('开发中')}</span></h2><p>{t('选择应用界面使用的语言；题目名称、OJ 标签和你的笔记不会翻译。')}</p></div></div></article>
      <article className="panel preference-card"><div className="preference-heading"><Palette size={18} /><div><small lang="en">APPEARANCE</small><h2>{t('外观')}</h2><p>{t('修改后立即生效，并保存在当前设备。')}</p></div></div><div className="preference-row"><div><strong>{t('主题')}</strong><span>{t('灰色介于亮色与暗色之间；跟随系统仍响应操作系统的亮暗模式。')}</span></div>{choices(preferences.theme, [['system', '跟随系统'], ['light', '亮色'], ['gray', '灰色'], ['dark', '暗色']], (theme) => onPreferences({ theme }))}</div><div className="preference-row"><div><strong>{t('字号')}</strong><span>{t('标准字号已经提高小字尺寸，不会恢复到原来的过小字号。')}</span></div>{choices(preferences.fontSize, [['standard', '标准'], ['large', '放大'], ['xlarge', '特大']], (fontSize) => onPreferences({ fontSize }))}</div><div className="preference-row"><div><strong>{t('界面密度')}</strong><span>{t('只调整卡片和列表间距，不缩小文字。')}</span></div>{choices(preferences.density, [['comfortable', '舒适'], ['compact', '紧凑']], (density) => onPreferences({ density }))}</div></article>
      <article className="panel preference-card"><div className="preference-heading"><Type size={18} /><div><small lang="en">ACCESSIBILITY</small><h2>{t('图表与动效')}</h2><p>{t('改善长时间查看统计数据时的辨识度。')}</p></div></div><div className="preference-row"><div><strong>{t('活动砖配色')}</strong><span>{t('只影响训练热力图，不覆盖各 OJ 的平台颜色。')}</span></div>{choices(preferences.heatmapPalette, [['green', '绿色'], ['blue', '蓝色'], ['accessible', '色觉友好']], (heatmapPalette) => onPreferences({ heatmapPalette }))}</div><div className="preference-row"><div><strong>{t('减少动态效果')}</strong><span>{t('关闭曲线、活动砖和页面状态的过渡动画。')}</span></div><button className={`switch ${preferences.reduceMotion ? 'active' : ''}`} role="switch" aria-checked={preferences.reduceMotion} aria-label={t('减少动态效果')} onClick={() => onPreferences({ reduceMotion: !preferences.reduceMotion })}><i /></button></div></article>
      <article className="panel preference-card"><div className="preference-heading"><Monitor size={18} /><div><small lang="en">GENERAL</small><h2>{t('常规')}</h2><p>{t('统计范围与统计口径仍在总览和各 OJ 页面中切换。')}</p></div></div><div className="preference-row"><div><strong>{t('统计时区')}</strong><span>{t('影响今日进度、活动砖、连续天数和问候语。')}</span></div><label className="preference-select"><select value={timeZone} onChange={(event) => onTimeZone(event.target.value)}>{TIME_ZONE_OPTIONS.map(([value, label]) => <option value={value} key={value}>{t(label)} · {value}</option>)}</select><small>{timeZoneLabel(timeZone)}</small></label></div><div className="preference-row"><div><strong>{t('启动页面')}</strong><span>{t('固定打开总览，或回到上次浏览的位置。')}</span></div>{choices(preferences.startupPage, [['overview', '总览'], ['last', '上次浏览']], (startupPage) => onPreferences({ startupPage }))}</div><div className="preference-row"><div><strong>{t('最近 AC 通知保留条数')}</strong><span>{t('历史通知单独保存；超过上限时自动删除较旧记录，最多 100 条。')}</span></div><label className="preference-number"><input aria-label={t('最近 AC 通知保留条数')} type="number" min={1} max={100} step={1} value={preferences.watchedEventRetention} onChange={(event) => { const value = Number(event.target.value); onPreferences({ watchedEventRetention: Number.isFinite(value) ? Math.min(100, Math.max(1, Math.trunc(value))) : 1 }); }} /><span>{t('条')}</span></label></div><div className="preference-row"><div><strong>{t('启动时检查更新')}</strong><span>{t('后台检查新版本；没有更新或临时断网时不打扰。')}</span></div><button className={`switch ${preferences.autoCheckUpdates ? 'active' : ''}`} role="switch" aria-checked={preferences.autoCheckUpdates} aria-label={t('启动时检查更新')} onClick={() => onPreferences({ autoCheckUpdates: !preferences.autoCheckUpdates })}><i /></button></div><div className="preference-row"><div><strong>{t('启动时同步全部')}</strong><span>{t('先显示本地缓存，再在后台增量同步已配置的平台。')}</span></div><button className={`switch ${preferences.autoSync ? 'active' : ''}`} role="switch" aria-checked={preferences.autoSync} aria-label={t('启动时同步全部')} onClick={() => onPreferences({ autoSync: !preferences.autoSync })}><i /></button></div></article>
      <div className="preferences-foot"><button onClick={() => { onPreferences({ ...DEFAULT_PREFERENCES }); onTimeZone('Asia/Shanghai'); }}><RotateCcw size={14} />{t('恢复个性化默认值')}</button></div>
    </section>}
    {tab === 'data' && <DataPage embedded statuses={statuses} syncing={syncing} timeZone={timeZone} onSync={onSync} onSyncAll={onSyncAll} onCleared={onCleared} notify={notify} />}
  </>;
}
