import { useMemo, useState } from 'react';
import type { DailyPoint } from '../types';

interface Props { startDay: string; endDay: string; daily: DailyPoint[]; onDay?: (day: string) => void; }
const CELL=12,GAP=3,STEP=CELL+GAP;
const LEVELS=['level-0','level-1','level-2','level-3','level-4'];

function utc(day:string){return new Date(`${day}T00:00:00Z`);}
function makeDays(startDay:string,endDay:string,data:Map<string,number>){
  const start=utc(startDay),end=utc(endDay),gridStart=new Date(start);gridStart.setUTCDate(gridStart.getUTCDate()-gridStart.getUTCDay());
  const out:Array<{day:string;dow:number;week:number;count:number}>=[];
  for(const d=new Date(gridStart);d<=end;d.setUTCDate(d.getUTCDate()+1)){
    const day=d.toISOString().slice(0,10);if(day<startDay||day>endDay)continue;
    const diff=Math.floor((d.getTime()-gridStart.getTime())/86400000);out.push({day,dow:d.getUTCDay(),week:Math.floor(diff/7),count:data.get(day)||0});
  }
  return out;
}

export default function Heatmap({startDay,endDay,daily,onDay}:Props){
  const[hover,setHover]=useState<{day:string;count:number;x:number;y:number}|null>(null);
  const map=useMemo(()=>new Map(daily.map(x=>[x.day,x.count])),[daily]);
  const days=useMemo(()=>makeDays(startDay,endDay,map),[startDay,endDay,map]);
  const weeks=Math.max(1,...days.map(d=>d.week+1));const max=Math.max(1,...days.map(d=>d.count));
  const lvl=(n:number)=>!n?0:n/max<=.2?1:n/max<=.45?2:n/max<=.72?3:4;
  const months=useMemo(()=>{const seen=new Set<string>();const arr:Array<{month:string;week:number}>=[];for(const d of days){const key=d.day.slice(0,7);if(!seen.has(key)){seen.add(key);arr.push({month:`${Number(d.day.slice(5,7))}月`,week:d.week});}}return arr;},[days]);
  return <div className="heatmap-shell"><div className="heatmap-scroll"><div className="heatmap" style={{width:84+weeks*STEP}}><div className="month-labels">{months.map((m,i)=><span key={`${m.month}-${i}`} style={{left:46+m.week*STEP}}>{m.month}</span>)}</div><div className="weekday-labels"><span>一</span><span>三</span><span>五</span></div><div className="cells" style={{width:weeks*STEP,height:7*STEP}}>{days.map(d=><button key={d.day} className={`heat-cell ${LEVELS[lvl(d.count)]}`} style={{left:d.week*STEP,top:d.dow*STEP,width:CELL,height:CELL}} aria-label={`${d.day}: ${d.count}`} onClick={()=>onDay?.(d.day)} onMouseEnter={e=>setHover({day:d.day,count:d.count,x:e.clientX,y:e.clientY})} onMouseMove={e=>setHover(v=>v?{...v,x:e.clientX,y:e.clientY}:v)} onMouseLeave={()=>setHover(null)}/>)}</div></div></div><div className="heat-legend"><span>少</span>{LEVELS.map(x=><i key={x} className={x}/>)}<span>多</span></div>{hover&&<div className="heat-tooltip" style={{left:hover.x+14,top:hover.y+14}}><strong>{hover.day}</strong><span>{hover.count} 条</span></div>}</div>;
}
