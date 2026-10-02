import { FileUp, ShieldCheck } from 'lucide-react';
import { useRef, useState } from 'react';
import { PLATFORM_ORDER } from '../lib/platforms';
import { api } from '../services/api';
import type { AccountConfig, Platform } from '../types';

interface PersonalProfile {
  schema: string;
  schema_version: number;
  contains_credentials?: boolean;
  accounts?: Array<{ platform?: string; account?: string; secret?: string }>;
}

export default function PersonalInfoImport({ accounts, onImported, notify }: { accounts: AccountConfig[]; onImported: () => void | Promise<void>; notify: (message: string) => void }) {
  const input = useRef<HTMLInputElement>(null);
  const [busy, setBusy] = useState(false);

  const read = async (file?: File) => {
    if (!file) return;
    setBusy(true);
    try {
      const profile = JSON.parse(await file.text()) as PersonalProfile;
      if (profile.schema !== 'com.ojinsight.personal-profile' || profile.schema_version !== 1 || !Array.isArray(profile.accounts)) {
        throw new Error('不是受支持的 OJ Insight 个人信息备份');
      }
      const imported: AccountConfig[] = profile.accounts.map((entry) => {
        if (!PLATFORM_ORDER.includes(entry.platform as Platform) || typeof entry.account !== 'string' || !entry.account.trim()) {
          throw new Error('备份中包含无效的平台或账号');
        }
        return { platform: entry.platform as Platform, account: entry.account.trim(), secret: typeof entry.secret === 'string' ? entry.secret.trim() : '' };
      });
      const unique = new Set(imported.map((entry) => `${entry.platform}\u0000${entry.account}`));
      if (unique.size !== imported.length) throw new Error('备份中包含重复账号');
      const credentials = imported.filter((entry) => entry.secret).length;
      if (!confirm(`将导入 ${imported.length} 个账号${credentials ? `，其中 ${credentials} 个包含 Cookie / Secret` : ''}。\n\n同名账号会更新，其他现有账号会保留；训练记录不会被导入或覆盖。确认继续？`)) return;
      const merged = accounts.map((entry) => ({ ...entry }));
      for (const entry of imported) {
        const index = merged.findIndex((current) => current.platform === entry.platform && current.account.trim() === entry.account);
        if (index < 0) merged.push(entry);
        else merged[index] = { ...merged[index], secret: entry.secret || merged[index].secret };
      }
      await api.saveAllAccounts(merged);
      await onImported();
      notify(`已导入 ${imported.length} 个账号${credentials ? '及其凭据' : ''}，现有训练记录未改动`);
    } catch (error) {
      notify(`导入失败：${String(error)}`);
    } finally {
      setBusy(false);
      if (input.current) input.current.value = '';
    }
  };

  return <section className="panel personal-export-panel personal-import-panel">
    <div className="preference-heading"><ShieldCheck size={18} /><div><small lang="en">PERSONAL INFORMATION IMPORT</small><h2>导入个人信息</h2><p>额外导入入口；上方手动填写方式保持不变。</p></div></div>
    <div className="personal-export-body">
      <div className="personal-export-summary"><div><strong>账号与凭据</strong><span>合并导入，不覆盖训练数据</span></div><p>支持 OJ Insight 导出的个人信息 JSON。同名账号更新，未包含的现有账号会保留。</p></div>
      <div className="personal-export-actions"><input ref={input} type="file" accept="application/json,.json" hidden onChange={(event) => void read(event.target.files?.[0])} /><button className="primary" disabled={busy} onClick={() => input.current?.click()}><FileUp size={15} />{busy ? '导入中…' : '选择备份并导入'}</button></div>
    </div>
  </section>;
}
