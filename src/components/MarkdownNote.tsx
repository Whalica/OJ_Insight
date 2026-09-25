import { useEffect, useRef, useState } from 'react';
import katex from 'katex';
import 'katex/dist/katex.min.css';

function MathText({ source, displayMode = false }: { source: string; displayMode?: boolean }) {
  const html = katex.renderToString(source, { displayMode, throwOnError: false, trust: false, strict: 'warn' });
  return <span className={displayMode ? 'math-display' : 'math-inline'} dangerouslySetInnerHTML={{ __html: html }} />;
}

function inline(text: string) {
  return text.split(/(\$[^$\n]+\$|\*\*[^*]+\*\*|`[^`]+`|\[[^\]]+\]\(https?:\/\/[^)]+\))/g).map((part, index) => {
    if (part.startsWith('$') && part.endsWith('$') && part.length > 2) return <MathText key={index} source={part.slice(1, -1)} />;
    if (part.startsWith('**') && part.endsWith('**')) return <strong key={index}>{part.slice(2, -2)}</strong>;
    if (part.startsWith('`') && part.endsWith('`')) return <code key={index}>{part.slice(1, -1)}</code>;
    const link = part.match(/^\[([^\]]+)\]\((https?:\/\/[^)]+)\)$/);
    if (link) return <a key={index} href={link[2]} target="_blank" rel="noreferrer">{link[1]}</a>;
    return part;
  });
}

export function MarkdownPreview({ text }: { text: string }) {
  let code = false;
  const lines = text.replace(/\$\$([\s\S]*?)\$\$/g, (_, formula: string) => `$$${formula.replace(/\r?\n/g, ' ')}$$`).split('\n');
  return <div className="vp-markdown">{lines.map((line, index) => {
    if (line.startsWith('```')) { code = !code; return null; }
    if (code) return <pre key={index}>{line}</pre>;
    if (line.startsWith('$$') && line.endsWith('$$') && line.length > 4) return <MathText key={index} source={line.slice(2, -2)} displayMode />;
    if (line.startsWith('### ')) return <h4 key={index}>{inline(line.slice(4))}</h4>;
    if (line.startsWith('## ')) return <h3 key={index}>{inline(line.slice(3))}</h3>;
    if (line.startsWith('# ')) return <h2 key={index}>{inline(line.slice(2))}</h2>;
    if (line.startsWith('- ')) return <li key={index}>{inline(line.slice(2))}</li>;
    return <p key={index}>{inline(line || '\u00a0')}</p>;
  })}</div>;
}

export default function MarkdownNote({ value, onSave, placeholder }: { value: string; onSave: (value: string) => Promise<unknown>; placeholder: string }) {
  const [text, setText] = useState(value);
  const [status, setStatus] = useState('');
  const saveRef = useRef(onSave);
  const savedRef = useRef(value);
  useEffect(() => { saveRef.current = onSave; }, [onSave]);
  useEffect(() => { savedRef.current = value; setText(value); }, [value]);
  useEffect(() => { if (text === savedRef.current) return; const timer = window.setTimeout(() => { void saveRef.current(text).then(() => { savedRef.current = text; setStatus('已保存'); }).catch((error) => setStatus(String(error))); }, 700); return () => window.clearTimeout(timer); }, [text]);
  return <div className="vp-note"><div className="vp-note-head"><span>Markdown · 支持 $公式$ 与 $$独立公式$$</span><small>{status}</small></div><div className="markdown-split"><div><strong>编辑</strong><textarea value={text} onChange={(event) => { setText(event.target.value); setStatus('保存中…'); }} onBlur={() => { if (text !== savedRef.current) void saveRef.current(text).then(() => { savedRef.current = text; setStatus('已保存'); }).catch((error) => setStatus(String(error))); }} placeholder={placeholder} /></div><div><strong>预览</strong><MarkdownPreview text={text} /></div></div></div>;
}
