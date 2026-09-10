import { AlertTriangle, Download, KeyRound, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { exportPersonalProfile } from '../lib/export';
import type { AccountConfig } from '../types';

export default function PersonalInfoExport({ accounts, notify }: { accounts: AccountConfig[]; notify?: (message: string) => void }) {
  const [busy, setBusy] = useState<'safe' | 'full' | null>(null);
  const [confirmCredentials, setConfirmCredentials] = useState(false);
  const run = async (includeCredentials: boolean) => {
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
    <div className="personal-export-body">
      <div className="personal-export-summary"><div><strong>账号档案</strong><span>{configured} 个已配置的平台账号</span></div><p>仅导出平台、账号标识及可选凭据；训练记录和个性化设置不会导出。</p></div>
      <div className="personal-export-actions"><button className="primary" disabled={!!busy || !configured} onClick={() => run(false)}><Download size={15} />{busy === 'safe' ? '导出中…' : '安全导出（不含凭据）'}</button><button className="credential-export" disabled={!!busy || !configured} onClick={() => setConfirmCredentials(true)}><KeyRound size={15} />{busy === 'full' ? '导出中…' : '包含 Cookie 导出'}</button></div>
      <div className="credential-note"><AlertTriangle size={16} /><span><strong>敏感导出</strong>包含 Cookie / Session 的文件可能被用于登录你的账号，请将其视同密码。</span></div>
    </div>
    {confirmCredentials && <div className="credential-confirm-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setConfirmCredentials(false); }}>
      <div className="credential-confirm" role="alertdialog" aria-modal="true" aria-labelledby="credential-confirm-title" aria-describedby="credential-confirm-description">
        <div className="credential-confirm-icon"><AlertTriangle size={22} /></div>
        <div><small>SENSITIVE INFORMATION</small><h3 id="credential-confirm-title">确认包含 Cookie 导出？</h3><p id="credential-confirm-description">导出文件将包含 Cookie / Session 等敏感凭据。仅交给可信的接收方，不要上传到公开网盘、聊天群或代码仓库。</p></div>
        <div className="credential-confirm-actions"><button onClick={() => setConfirmCredentials(false)}>取消</button><button className="credential-export" onClick={() => { setConfirmCredentials(false); void run(true); }}><KeyRound size={15} />仍要包含 Cookie</button></div>
      </div>
    </div>}
  </section>;
}
