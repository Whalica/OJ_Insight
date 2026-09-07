import { useEffect, useState } from 'react';
import { Monitor, Palette, Plus, RotateCcw, Save, Type, X } from 'lucide-react';
import { api } from '../lib/api';
import { TIME_ZONE_OPTIONS, timeZoneLabel } from '../lib/date';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import { emptyAccounts, type AccountMap } from '../lib/ui';
import { DEFAULT_PREFERENCES, type Preferences } from '../lib/preferences';
import type { Platform } from '../types';

interface Props {
  accounts: AccountMap;
  timeZone: string;
  onTimeZone: (value: string) => void;
  preferences: Preferences;
  onPreferences: (patch: Partial<Preferences>) => void;
  onSaved: () => void | Promise<void>;
  syncing: string | null;
  notify: (message: string) => void;
}

export default function SettingsPage({ accounts, timeZone, onTimeZone, preferences, onPreferences, onSaved, syncing, notify }: Props) {
  const [tab, setTab] = useState<'accounts' | 'personalization'>('accounts');
  const [form, setForm] = useState<AccountMap>(emptyAccounts);
  const [saving, setSaving] = useState(false);
  useEffect(() => {
    const next = emptyAccounts();
    for (const platform of PLATFORM_ORDER) next[platform] = accounts[platform].length ? accounts[platform].map((entry) => ({ ...entry })) : [{ platform, account: '', secret: '' }];
    setForm(next);
  }, [accounts]);
  const change = (platform: Platform, index: number, key: 'account' | 'secret', value: string) => setForm((current) => ({ ...current, [platform]: current[platform].map((entry, row) => row === index ? { ...entry, [key]: value } : entry) }));
  const add = (platform: Platform) => setForm((current) => ({ ...current, [platform]: [...current[platform], { platform, account: '', secret: '' }] }));
  const remove = (platform: Platform, index: number) => setForm((current) => { const rows = current[platform].filter((_, row) => row !== index); return { ...current, [platform]: rows.length ? rows : [{ platform, account: '', secret: '' }] }; });
  const save = async () => {
    const next = PLATFORM_ORDER.flatMap((platform) => form[platform].filter((entry) => entry.account.trim()));
    for (const platform of PLATFORM_ORDER) {
      const ids = next.filter((entry) => entry.platform === platform).map((entry) => entry.account.trim());
      if (new Set(ids).size !== ids.length) { notify(`${PLATFORM_META[platform].name} 中存在重复 ID，请合并后保存`); return; }
    }
    const removed = PLATFORM_ORDER.flatMap((platform) => accounts[platform].filter((entry) => !next.some((value) => value.platform === platform && value.account.trim() === entry.account)).map((entry) => `${PLATFORM_META[platform].name} · ${entry.account}`));
    if (removed.length && !confirm(`将移除以下 ID 及其本地提交、活动、难度、Rating 和同步记录：\n${removed.join('\n')}\n\n其他 ID 的记录会保留。确认保存？`)) return;
    setSaving(true);
    try { await api.saveAllAccounts(next); await onSaved(); } catch (error) { notify(String(error)); } finally { setSaving(false); }
  };
  const choices = <T extends string>(value: T, items: Array<[T, string]>, changeValue: (value: T) => void) => <div className="preference-choices">{items.map(([key, label]) => <button className={value === key ? 'active' : ''} onClick={() => changeValue(key)} key={key}>{label}</button>)}</div>;

  return <>
    <header className="topbar"><div><small>SETTINGS</small><h1>设置</h1><p>管理平台账号和本机显示偏好。</p></div>{tab === 'accounts' && <button className="primary" onClick={save} disabled={saving || !!syncing}><Save size={16} />{saving ? '保存中' : '保存账号'}</button>}</header>
    <div className="settings-tabs"><button className={tab === 'accounts' ? 'active' : ''} onClick={() => setTab('accounts')}>账号设置</button><button className={tab === 'personalization' ? 'active' : ''} onClick={() => setTab('personalization')}>个性化</button></div>
    {tab === 'accounts' ? <><section className="settings-intro"><strong>怎么填写？</strong><span>填写个人主页 URL 中的账号标识，不是显示昵称。Cookie 仅在对应平台需要登录数据时填写，全部只保存在本机。删除或改名会在保存时清理原 ID 的本地记录；同步期间请等待完成后再保存。</span></section><section className="account-panel">{PLATFORM_ORDER.map((platform) => <article className="account-card" key={platform}><div className="account-card-head"><div><span className="platform-monogram" style={{ color: PLATFORM_META[platform].accent }}>{PLATFORM_META[platform].short}</span><div><strong>{PLATFORM_META[platform].name}</strong><small>{PLATFORM_META[platform].accountHint}</small></div></div><button onClick={() => add(platform)}><Plus size={14} />添加 ID</button></div><div className="account-inputs">{form[platform]?.map((entry, index) => <div className="account-entry" key={index}><span>{index + 1}</span><input aria-label={`${PLATFORM_META[platform].name} ID ${index + 1}`} value={entry.account} onChange={(event) => change(platform, index, 'account', event.target.value)} placeholder={PLATFORM_META[platform].accountHint} />{PLATFORM_META[platform].secretHint ? <input aria-label={`${PLATFORM_META[platform].name} Cookie ${index + 1}`} type="password" value={entry.secret} onChange={(event) => change(platform, index, 'secret', event.target.value)} placeholder={PLATFORM_META[platform].secretHint} autoComplete="off" /> : <small>公开数据，无需密码</small>}<button className="remove-account" onClick={() => remove(platform, index)} aria-label="移除账号"><X size={14} /></button></div>)}</div></article>)}</section></> : <section className="preferences-panel">
      <article className="panel preference-card"><div className="preference-heading"><Palette size={18} /><div><small>APPEARANCE</small><h2>外观</h2><p>修改后立即生效，并保存在当前设备。</p></div></div><div className="preference-row"><div><strong>主题</strong><span>跟随系统会响应操作系统的亮暗模式。</span></div>{choices(preferences.theme, [['system', '跟随系统'], ['light', '亮色'], ['dark', '暗色']], (theme) => onPreferences({ theme }))}</div><div className="preference-row"><div><strong>字号</strong><span>标准字号已经提高小字尺寸，不会恢复到原来的过小字号。</span></div>{choices(preferences.fontSize, [['standard', '标准'], ['large', '放大'], ['xlarge', '特大']], (fontSize) => onPreferences({ fontSize }))}</div><div className="preference-row"><div><strong>界面密度</strong><span>只调整卡片和列表间距，不缩小文字。</span></div>{choices(preferences.density, [['comfortable', '舒适'], ['compact', '紧凑']], (density) => onPreferences({ density }))}</div></article>
      <article className="panel preference-card"><div className="preference-heading"><Type size={18} /><div><small>ACCESSIBILITY</small><h2>图表与动效</h2><p>改善长时间查看统计数据时的辨识度。</p></div></div><div className="preference-row"><div><strong>活动砖配色</strong><span>只影响训练热力图，不覆盖各 OJ 的平台颜色。</span></div>{choices(preferences.heatmapPalette, [['green', '绿色'], ['blue', '蓝色'], ['accessible', '色觉友好']], (heatmapPalette) => onPreferences({ heatmapPalette }))}</div><div className="preference-row"><div><strong>减少动态效果</strong><span>关闭曲线、活动砖和页面状态的过渡动画。</span></div><button className={`switch ${preferences.reduceMotion ? 'active' : ''}`} role="switch" aria-checked={preferences.reduceMotion} aria-label="减少动态效果" onClick={() => onPreferences({ reduceMotion: !preferences.reduceMotion })}><i /></button></div></article>
      <article className="panel preference-card"><div className="preference-heading"><Monitor size={18} /><div><small>GENERAL</small><h2>常规</h2><p>统计范围与统计口径仍在总览和各 OJ 页面中切换。</p></div></div><div className="preference-row"><div><strong>统计时区</strong><span>影响今日进度、活动砖、连续天数和问候语。</span></div><label className="preference-select"><select value={timeZone} onChange={(event) => onTimeZone(event.target.value)}>{TIME_ZONE_OPTIONS.map(([value, label]) => <option value={value} key={value}>{label} · {value}</option>)}</select><small>{timeZoneLabel(timeZone)}</small></label></div><div className="preference-row"><div><strong>启动页面</strong><span>固定打开总览，或回到上次浏览的位置。</span></div>{choices(preferences.startupPage, [['overview', '总览'], ['last', '上次浏览']], (startupPage) => onPreferences({ startupPage }))}</div></article>
      <div className="preferences-foot"><button onClick={() => { onPreferences({ ...DEFAULT_PREFERENCES }); onTimeZone('Asia/Shanghai'); }}><RotateCcw size={14} />恢复个性化默认值</button></div>
    </section>}
  </>;
}
