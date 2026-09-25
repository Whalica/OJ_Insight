import { useEffect, useRef, useState } from 'react';

function inline(text: string) {
  return text.split(/(\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\(https?:\/\/[^)]+\))/g).map((part, index) => {
    if (part.startsWith('**') && part.endsWith('**')) return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith('`') && part.endsWith('`')) return <code key={index}>{part.slice(1, -1)}</code>;
    const link = part.match(/^\[([^\]]+)\]\((https?:\/\/[^)]+)\)$/);
    if (link) return <a key={index} href={link[2]} target="_blank" rel="noreferrer">{link[1]}</a>;
    return part;
  });
}

function Preview({ text }: { text: string }) {
  let code = false;
  return <div className="vp-markdown">{text.split('\n').map((line, index) => {
    if (line.startsWith('```')) { code = !code; return null; }
    if (code) return <pre key={index}>{line}</pre>;
    if (line.startsWith('### ')) return <h4 key={index}>{inline(line.slice(4))}</h4>;
    if (line.startsWith('## ')) return <h3 key={index}>{inline(line.slice(3))}</h3>;
    if (line.startsWith('# ')) return <h2 key={index}>{inline(line.slice(2))}</h2>;
    if (line.startsWith('- ')) return <li key={index}>{inline(line.slice(2))}</li>;
    return <p key={index}>{inline(line || '\u00a0')}</p>;
  })}</div>;
}

export default function MarkdownNote({ value, onSave, placeholder }: { value: string; onSave: (value: string) => Promise<unknown>; placeholder: string }) {
  const [text, setText] = useState(value);
  const [preview, setPreview] = useState(false);
  const [status, setStatus] = useState('');
  const saveRef = useRef(onSave);
  const savedRef = useRef(value);
  useEffect(() => { saveRef.current = onSave; }, [onSave]);
  useEffect(() => { savedRef.current = value; setText(value); }, [value]);
  useEffect(() => { if (text === savedRef.current) return; const timer = window.setTimeout(() => { void saveRef.current(text).then(() => { savedRef.current = text; setStatus('已保存'); }).catch((error) => setStatus(String(error))); }, 700); return () => window.clearTimeout(timer); }, [text]);
  return <div className="vp-note"><div className="vp-note-tabs"><button className={!preview ? 'active' : ''} onClick={() => setPreview(false)}>编辑</button><button className={preview ? 'active' : ''} onClick={() => setPreview(true)}>预览</button><small>{status}</small></div>{preview ? <Preview text={text} /> : <textarea value={text} onChange={(event) => { setText(event.target.value); setStatus('保存中…'); }} onBlur={() => { if (text !== savedRef.current) void saveRef.current(text).then(() => { savedRef.current = text; setStatus('已保存'); }).catch((error) => setStatus(String(error))); }} placeholder={placeholder} />}</div>;
}
