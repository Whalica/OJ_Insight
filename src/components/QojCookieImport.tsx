import { isTauri } from '@tauri-apps/api/core';
import { useState } from 'react';

import { useI18n } from '../lib/i18n';
import { api } from '../services/api';
import type { AccountConfig } from '../types';

export default function QojCookieImport({ accounts, onCookie, notify }: {
  accounts: AccountConfig[];
  onCookie: (index: number, cookie: string) => void;
  notify: (message: string) => void;
}) {
  const { t } = useI18n();
  const [busy, setBusy] = useState(false);
  if (!isTauri()) return null;

  const openLogin = async () => {
    try { await api.openQojLogin(); }
    catch (error) { notify(t('打开 QOJ 登录窗口失败：{error}', { error: String(error) })); }
  };
  const importCookie = async (index: number) => {
    setBusy(true);
    try {
      onCookie(index, await api.readQojLoginCookie());
      notify(t('已导入 QOJ Cookie，请保存账号'));
    } catch (error) {
      notify(t('导入 QOJ Cookie 失败：{error}', { error: String(error) }));
    } finally { setBusy(false); }
  };

  return <section className="qoj-cookie-import panel">
    <div><strong>{t('在 OJI 中登录 QOJ')}</strong><p>{t('在应用打开的 QOJ 页面登录，然后把 Cookie 导入对应账号。OJI 会读取完整 Cookie，无需填写名称。')}</p></div>
    <button onClick={() => void openLogin()}>{t('打开 QOJ 登录页')}</button>
    {accounts.map((entry, index) => <button key={index} onClick={() => void importCookie(index)} disabled={busy}>
      {t('导入到 {account}', { account: entry.account.trim() || `QOJ ${index + 1}` })}
    </button>)}
  </section>;
}
