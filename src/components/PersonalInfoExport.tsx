import { Download, KeyRound, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { exportPersonalProfile } from '../lib/export';
import type { AccountConfig } from '../types';

export default function PersonalInfoExport({ accounts, notify }: { accounts: AccountConfig[]; notify?: (message: string) => void }) {
  const [busy, setBusy] = useState<'safe' | 'full' | null>(null);
  const run = async (includeCredentials: boolean) => {
    if (includeCredentials && !confirm('导出的文件将包含 Cookie / Session 等敏感凭据。\n\n请把文件视同密码，仅交给可信的接收方，不要上传到公开网盘、聊天群或代码仓库。\n\n仍要继续导出吗？')) return;
    setBusy(includeCredentials ? 'full' : 'safe');
    try {
      const saved = await exportPersonalProfile(accounts, includeCredentials);
      if (saved) notify?.(includeCredentials ? '个人信息已导出，文件包含敏感凭据，请妥善保管' : '个人信息已安全导出，不包含 Cookie / Session');
    } catch (error) { notify?.(String(error)); }
    finally { setBusy(null); }
  };
  const configured = accounts.filter((entry) => entry.account.trim()).length;
  return <section className="panel personal-export-panel">
    <div className="preference-heading"><ShieldCheck size={18} /><div><small>PERSONAL INFORMATION EXPORT</small><h2>导出个人信息</h2><p>供未来教练端批量导入账号档案；不包含个性化与训练记录。</p></div></div>
    <div className="personal-export-body"><div className="personal-export-scope"><div><strong>平台账号</strong><span>{configured} 个已配置账号</span></div><div><strong>Cookie / Session</strong><span>默认排除，可单独选择包含</span></div><div><strong>个性化设置</strong><span>始终排除</span></div></div><div className="personal-export-actions"><button className="primary" disabled={!!busy || !configured} onClick={() => run(false)}><Download size={15} />{busy === 'safe' ? '导出中…' : '安全导出（不含凭据）'}</button><button className="credential-export" disabled={!!busy || !configured} onClick={() => run(true)}><KeyRound size={15} />{busy === 'full' ? '导出中…' : '导出并包含 Cookie'}</button><small>包含凭据的文件应视同密码，仅在接收方和传输环境可信时使用。</small></div></div>
  </section>;
}
