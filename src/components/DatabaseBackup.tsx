import { useState } from 'react';
import { Download, FolderOpen, ShieldCheck } from 'lucide-react';
import { dirname, sep } from '@tauri-apps/api/path';
import { save } from '@tauri-apps/plugin-dialog';
import { openPath } from '@tauri-apps/plugin-opener';
import { api } from '../services/api';

export default function DatabaseBackup({ notify }: { notify: (message: string) => void }) {
  const [busy, setBusy] = useState(false);
  const [directory, setDirectory] = useState<string | null>(null);
  const backup = async () => {
    setBusy(true);
    try {
      const storage = await api.storageInfo();
      const date = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
      const separator = sep();
      const slash = /[\\/]$/.test(storage.exportDir) ? '' : separator;
      const path = await save({
        defaultPath: `${storage.exportDir}${slash}oji-data-${date}.zip`,
        filters: [{ name: 'OJ Insight 数据库备份', extensions: ['zip'] }],
      });
      if (!path) return;
      await api.createDatabaseBackup(path);
      setDirectory(await dirname(path));
      notify('数据库备份已完成并通过完整性检查');
    } catch (error) { notify(String(error)); }
    finally { setBusy(false); }
  };
  return <section className="panel personal-export-panel database-backup-panel">
    <div className="preference-heading"><ShieldCheck size={18} /><div><small>LOCAL DATA BACKUP</small><h2>备份本机数据库</h2><p>保存一份一致的数据库快照，包含账号、提交、题单、比赛和训练记录。</p></div></div>
    <div className="personal-export-body">
      <p>备份文件包含 Cookie 等敏感凭据，请勿公开上传。它不包含界面偏好及已导出的文件；完整迁移仍需先退出应用，再复制整个 OJ Insight 目录。恢复时请先退出应用，把原 <code>data</code> 文件夹移到安全位置，再从 ZIP 解出新的 <code>data/oj-insight.sqlite3</code>。</p>
      <div className="personal-export-actions"><button className="primary" disabled={busy} onClick={() => void backup()}><Download size={15} />{busy ? '备份中…' : '创建数据库备份'}</button>{directory && <button onClick={() => void openPath(directory).catch((error) => notify(String(error)))}><FolderOpen size={15} />打开备份位置</button>}</div>
    </div>
  </section>;
}
