import { save } from '@tauri-apps/plugin-dialog';
import { sep } from '@tauri-apps/api/path';
import { APP_VERSION } from '../lib/version';
import { mergeKnowledgeBuckets } from '../lib/knowledge';
import { difficultyColor } from '../lib/platforms';
import type { AccountConfig, Platform, Snapshot } from '../types';
import { api } from './api';

const CELL = 11;
const GAP = 3;
const STEP = CELL + GAP;
const LEFT = 76;
const TOP = 46;
const WEEKS = 53;

export type ExportSection = { label: string; platform: Platform | null; snapshot: Snapshot };

function palette() {
  const style = getComputedStyle(document.documentElement);
  const value = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  return {
    bg: value('--bg', '#0b0e12'), panel: value('--panel', '#12171d'), panelSoft: value('--panel-soft', '#171d23'),
    line: value('--line', '#29323b'), text: value('--text', '#f1f4f7'), muted: value('--muted', '#89949f'),
    accent: value('--accent', '#55d77d'), brick: value('--brick-empty', '#1c232a'),
    heat: `rgb(${value('--heatmap-rgb', '85 215 125').replace(/\s+/g, ',')})`,
    unrated: value('--unrated-difficulty', '#8b9094'), atcoder: value('--atcoder-difficulty-gray', '#9aa4ad'),
  };
}

async function svgToPng(svg: string, width: number, height: number) {
  const canvas = document.createElement('canvas');
  canvas.width = width * 2; canvas.height = height * 2;
  const ctx = canvas.getContext('2d');
  if (!ctx) throw new Error('当前系统无法创建图片画布');
  ctx.scale(2, 2);
  try {
    const bitmap = await createImageBitmap(new Blob([svg], { type: 'image/svg+xml;charset=utf-8' }));
    ctx.drawImage(bitmap, 0, 0, width, height); bitmap.close();
  } catch {
    const img = new Image(); const url = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
    await new Promise<void>((resolve, reject) => { img.onload = () => resolve(); img.onerror = () => reject(new Error('图表渲染失败')); img.src = url; });
    ctx.drawImage(img, 0, 0, width, height);
  }
  return await new Promise<Blob>((resolve, reject) => canvas.toBlob((value) => value ? resolve(value) : reject(new Error('PNG 编码失败')), 'image/png'));
}

async function copyPng(png: Blob) {
  if (navigator.clipboard?.write && typeof ClipboardItem !== 'undefined') {
    try {
      await navigator.clipboard.write([new ClipboardItem({ 'image/png': png })]);
      return;
    } catch { /* WebView2 may expose ClipboardItem but reject image writes. */ }
  }
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader(); reader.onload = () => resolve(String(reader.result)); reader.onerror = () => reject(new Error('无法读取生成的图片')); reader.readAsDataURL(png);
  });
  const host = document.createElement('div');
  host.contentEditable = 'true'; host.style.cssText = 'position:fixed;left:-10000px;top:0;';
  const image = document.createElement('img'); image.src = dataUrl; host.appendChild(image); document.body.appendChild(host);
  try {
    await image.decode();
    const range = document.createRange(); range.selectNode(image);
    const selection = window.getSelection(); selection?.removeAllRanges(); selection?.addRange(range);
    if (!document.execCommand('copy')) throw new Error('系统剪贴板拒绝了图片写入');
    selection?.removeAllRanges();
  } finally { host.remove(); }
}

async function deliverSvg(svg: string, width: number, height: number, filename: string, format: 'png' | 'svg', action: 'save' | 'copy') {
  if (action === 'copy') {
    const png = await svgToPng(svg, width, height);
    await copyPng(png);
    return true;
  }
  const storage = await api.storageInfo();
  const separator = sep();
  const slash = storage.exportDir.endsWith('/') || storage.exportDir.endsWith('\\') ? '' : separator;
  const path = await save({ defaultPath: `${storage.exportDir}${slash}${filename}.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] });
  if (!path) return false;
  const data = format === 'svg' ? new TextEncoder().encode(svg) : new Uint8Array(await (await svgToPng(svg, width, height)).arrayBuffer());
  await api.writeExportFile(path, Array.from(data));
  return true;
}

const level = (n: number, max: number) => {
  if (!n) return 0;
  if (max <= 1) return 4;
  const x = n / max;
  if (x <= .2) return 1;
  if (x <= .45) return 2;
  if (x <= .72) return 3;
  return 4;
};

const levelOpacity = [0, .30, .52, .76, 1];
type ExportDay = { day: string; week: number; dow: number; count: number };

function yearDays(year: number, points: Map<string, number>, startDay?:string, endDay?:string) {
  const jan1 = new Date(Date.UTC(year, 0, 1));
  const dec31 = new Date(Date.UTC(year, 11, 31));
  const firstSunday = new Date(jan1);
  firstSunday.setUTCDate(jan1.getUTCDate() - jan1.getUTCDay());
  const rows: ExportDay[] = [];
  for (let d = new Date(firstSunday); d <= dec31; d.setUTCDate(d.getUTCDate() + 1)) {
    if (d.getUTCFullYear() > year) break;
    const day = d.toISOString().slice(0, 10);
    if (d.getUTCFullYear() === year && (!startDay || day>=startDay) && (!endDay || day<=endDay)) {
      const diff = Math.floor((d.getTime() - firstSunday.getTime()) / 86400000);
      rows.push({ day, week: Math.floor(diff / 7), dow: d.getUTCDay(), count: points.get(day) || 0 });
    }
  }
  return rows;
}

function rangeDays(startDay:string,endDay:string,points:Map<string,number>) {
  const start=new Date(`${startDay}T00:00:00Z`),end=new Date(`${endDay}T00:00:00Z`),firstSunday=new Date(start);
  firstSunday.setUTCDate(start.getUTCDate()-start.getUTCDay());const rows:ExportDay[]=[];
  for(let d=new Date(firstSunday);d<=end;d.setUTCDate(d.getUTCDate()+1)){
    const day=d.toISOString().slice(0,10);if(day<startDay)continue;
    const diff=Math.floor((d.getTime()-firstSunday.getTime())/86400000);rows.push({day,week:Math.floor(diff/7),dow:d.getUTCDay(),count:points.get(day)||0});
  }
  return rows;
}

const esc = (value: string) => value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&apos;' }[character]!));

function frame(title: string, width: number, height: number, body: string) {
  const colors = palette();
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}"><rect width="100%" height="100%" fill="${colors.bg}"/><text x="42" y="54" fill="${colors.text}" font-size="25" font-weight="700" font-family="Segoe UI,Arial">${esc(title)}</text>${body}<text x="42" y="${height - 24}" fill="${colors.muted}" font-size="12" font-family="Segoe UI,Arial">Generated by OJ Insight</text></svg>`;
}

function statCards(snapshot: Snapshot, x: number, y: number, width: number) {
  const colors = palette();
  const cards = [['生涯解题',snapshot.career.solved],['AC 提交',snapshot.career.accepted_submissions],['活跃天数',snapshot.career.active_days],['最长连续',snapshot.career.longest_streak]];
  return cards.map(([label,value], index) => { const cardWidth=(width-36)/4, left=x+index*(cardWidth+12); return `<rect x="${left}" y="${y}" width="${cardWidth}" height="88" rx="10" fill="${colors.panelSoft}" stroke="${colors.line}"/><text x="${left+16}" y="${y+27}" fill="${colors.muted}" font-size="12" font-family="Segoe UI,Arial">${label}</text><text x="${left+16}" y="${y+66}" fill="${colors.text}" font-size="28" font-weight="700" font-family="Segoe UI,Arial">${value}</text>`; }).join('');
}

function heatmapPanel(section: ExportSection, startDay: string, endDay: string, x: number, y: number, width: number) {
  const colors = palette(); const map = new Map(section.snapshot.daily.map((item) => [item.day,item.count]));
  const days = rangeDays(startDay,endDay,map); const weeks=Math.max(1,...days.map(item=>item.week+1));
  const cell=Math.min(11,Math.max(3,(width-190)/weeks-2)), step=cell+2, max=Math.max(1,...section.snapshot.daily.map(item=>item.count));
  let body=`<rect x="${x}" y="${y}" width="${width}" height="132" rx="10" fill="${colors.panel}" stroke="${colors.line}"/><text x="${x+18}" y="${y+27}" fill="${colors.text}" font-size="14" font-weight="700" font-family="Segoe UI,Arial">${esc(section.label)}</text><text x="${x+18}" y="${y+49}" fill="${colors.muted}" font-size="11" font-family="Segoe UI,Arial">${section.snapshot.career.solved} 题 · ${section.snapshot.career.active_days} 活跃天</text>`;
  for(const item of days){const bucket=level(item.count,max),left=x+170+item.week*step,top=y+18+item.dow*step;body+=`<rect x="${left}" y="${top}" width="${cell}" height="${cell}" rx="1.5" fill="${bucket?colors.heat:colors.brick}"${bucket?` fill-opacity="${levelOpacity[bucket]}"`:''}/>`;}
  return body;
}

export async function exportHeatmap(title: string, sections: ExportSection[], format: 'png' | 'svg', startDay: string, endDay: string, action: 'save' | 'copy' = 'save') {
  const width=1240,height=105+sections.length*146+50;
  let body=statCards(sections[0].snapshot,42,78,width-84);
  sections.forEach((section,index)=>body+=heatmapPanel(section,startDay,endDay,42,184+index*146,width-84));
  const svg=frame(title,width,height,body);
  return deliverSvg(svg,width,height,'OJ-Insight-activity',format,action);
}

function difficultyPanel(section: ExportSection, x:number, y:number, width:number) {
  const colors=palette(); const rows=section.snapshot.difficulty.filter(item=>item.count>0 && (!section.platform || item.platform===section.platform)).slice(0,18); const max=Math.max(1,...rows.map(item=>item.count));
  let body=`<rect x="${x}" y="${y}" width="${width}" height="190" rx="10" fill="${colors.panel}" stroke="${colors.line}"/><text x="${x+18}" y="${y+29}" fill="${colors.text}" font-size="14" font-weight="700" font-family="Segoe UI,Arial">${esc(section.label)}</text>`;
  if(!rows.length)return body+`<text x="${x+width/2}" y="${y+105}" text-anchor="middle" fill="${colors.muted}" font-size="13" font-family="Segoe UI,Arial">暂无难度数据</text>`;
  const slot=(width-54)/rows.length,bar=Math.max(8,Math.min(34,slot*.62)); rows.forEach((item,index)=>{const left=x+28+index*slot+(slot-bar)/2,h=item.count/max*105;let fill=difficultyColor(item.platform,item.label,item.order);if(fill.includes('--unrated'))fill=colors.unrated;if(fill.includes('--atcoder'))fill=colors.atcoder;body+=`<rect x="${left}" y="${y+145-h}" width="${bar}" height="${h}" rx="4" fill="${fill}"/><text x="${left+bar/2}" y="${y+139-h}" text-anchor="middle" fill="${colors.text}" font-size="10" font-family="Segoe UI,Arial">${item.count}</text><text x="${left+bar/2}" y="${y+166}" text-anchor="middle" fill="${colors.muted}" font-size="9" font-family="Segoe UI,Arial">${esc(item.label)}</text>`;});
  return body;
}

function knowledgePanel(section: ExportSection,x:number,y:number,width:number) {
  const colors=palette(),rows=mergeKnowledgeBuckets(section.snapshot.knowledge||[],section.platform),cx=x+190,cy=y+170,radius=100,pt=(i:number,score:number)=>{const a=Math.PI*2*i/8-Math.PI/2;return[cx+Math.cos(a)*radius*score/100,cy+Math.sin(a)*radius*score/100]};
  let body=`<rect x="${x}" y="${y}" width="${width}" height="330" rx="10" fill="${colors.panel}" stroke="${colors.line}"/><text x="${x+18}" y="${y+29}" fill="${colors.text}" font-size="14" font-weight="700" font-family="Segoe UI,Arial">${esc(section.label)}</text>`;
  [25,50,75,100].forEach(score=>body+=`<polygon points="${rows.map((_,i)=>pt(i,score).join(',')).join(' ')}" fill="none" stroke="${colors.line}"/>`); body+=`<polygon points="${rows.map((item,i)=>pt(i,item.score).join(',')).join(' ')}" fill="${colors.accent}" fill-opacity=".22" stroke="${colors.accent}" stroke-width="2"/>`;
  rows.forEach((item,index)=>{const top=y+61+index*31;body+=`<text x="${x+360}" y="${top}" fill="${colors.muted}" font-size="11" font-family="Segoe UI,Arial">${esc(item.axis)}</text><rect x="${x+458}" y="${top-8}" width="${Math.max(1,width-560)}" height="6" rx="3" fill="${colors.line}"/><rect x="${x+458}" y="${top-8}" width="${Math.max(0,width-560)*item.score/100}" height="6" rx="3" fill="${colors.accent}"/><text x="${x+width-24}" y="${top}" text-anchor="end" fill="${colors.text}" font-size="11" font-family="Segoe UI,Arial">${item.score} 分</text>`;}); return body;
}

export async function exportVisual(title: string, sections: ExportSection[], kind: 'difficulty' | 'knowledge' | 'overview', format: 'png' | 'svg', action: 'save' | 'copy', startDay?: string, endDay?: string) {
  const width=1240; let body=''; let height=0;
  if(kind==='difficulty'){height=105+sections.length*204+42;sections.forEach((section,index)=>body+=difficultyPanel(section,42,80+index*204,width-84));}
  else if(kind==='knowledge'){const supported=sections.filter(section=>!section.platform||['codeforces','leetcode','qoj'].includes(section.platform));height=105+supported.length*344+42;supported.forEach((section,index)=>body+=knowledgePanel(section,42,80+index*344,width-84));}
  else {const total=sections[0];height=1180;body+=statCards(total.snapshot,42,78,width-84);body+=heatmapPanel(total,startDay||'2026-01-01',endDay||'2026-12-31',42,184,width-84);body+=difficultyPanel(total,42,330,width-84);body+=knowledgePanel(total,42,534,width-84);const colors=palette();body+=`<text x="42" y="900" fill="${colors.text}" font-size="17" font-weight="700" font-family="Segoe UI,Arial">各 OJ 生涯概况</text>`;sections.slice(1).forEach((section,index)=>{const col=index%3,row=Math.floor(index/3),x=42+col*390,y=925+row*92;body+=`<rect x="${x}" y="${y}" width="372" height="74" rx="9" fill="${colors.panel}" stroke="${colors.line}"/><text x="${x+16}" y="${y+27}" fill="${colors.text}" font-size="13" font-weight="700" font-family="Segoe UI,Arial">${esc(section.label)}</text><text x="${x+16}" y="${y+52}" fill="${colors.muted}" font-size="11" font-family="Segoe UI,Arial">${section.snapshot.career.solved} 题 · ${section.snapshot.career.active_days} 活跃天 · ${section.snapshot.ratings.length} 个 Rating 账号</text>`;});}
  const svg=frame(title,width,height,body);return deliverSvg(svg,width,height,`OJ-Insight-${kind}`,format,action);
}

export async function exportPersonalProfile(accounts: AccountConfig[], includeCredentials: boolean) {
  const storage = await api.storageInfo();
  const separator = sep();
  const slash = storage.exportDir.endsWith('/') || storage.exportDir.endsWith('\\') ? '' : separator;
  const suffix = includeCredentials ? 'with-credentials' : 'safe';
  const defaultPath = `${storage.exportDir}${slash}OJ-Insight-personal-profile-${suffix}.json`;
  const path = await save({ defaultPath, filters: [{ name: 'JSON', extensions: ['json'] }] });
  if (!path) return false;
  const exportedAccounts = accounts
    .filter((entry) => entry.account.trim())
    .map((entry) => ({
      platform: entry.platform,
      account: entry.account.trim(),
      ...(includeCredentials && entry.secret.trim() ? { secret: entry.secret.trim() } : {}),
    }));
  const profile = {
    schema: 'com.ojinsight.personal-profile',
    schema_version: 1,
    app_version: APP_VERSION,
    exported_at: new Date().toISOString(),
    contains_credentials: includeCredentials,
    accounts: exportedAccounts,
  };
  const data = new TextEncoder().encode(`${JSON.stringify(profile, null, 2)}\n`);
  await api.writeExportFile(path, Array.from(data));
  return true;
}
