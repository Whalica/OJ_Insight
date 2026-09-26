import { useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { AlertTriangle } from 'lucide-react';

export default function ConfirmDialog({ title, message, onCancel, onConfirm }: { title: string; message: string; onCancel: () => void; onConfirm: () => void | Promise<unknown> }) {
  const [busy, setBusy] = useState(false);
  const confirm = async () => { if (busy) return; setBusy(true); try { await onConfirm(); } finally { setBusy(false); } };
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => { if (event.key === 'Escape' && !busy) onCancel(); };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [busy, onCancel]);
  return createPortal(<div className="training-confirm-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget && !busy) onCancel(); }}>
    <div className="training-confirm" role="alertdialog" aria-modal="true" aria-labelledby="training-confirm-title" aria-describedby="training-confirm-message">
      <AlertTriangle size={23} aria-hidden="true" />
      <div><h3 id="training-confirm-title">{title}</h3><p id="training-confirm-message">{message}</p></div>
      <div className="training-confirm-actions"><button autoFocus disabled={busy} onClick={onCancel}>取消</button><button className="danger" disabled={busy} onClick={() => void confirm()}>{busy ? '正在处理…' : '确认删除'}</button></div>
    </div>
  </div>, document.body);
}
