import { CircleHelp, Database, Download, LayoutDashboard, PanelLeftClose, PanelLeftOpen, Settings2, TableProperties } from 'lucide-react';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform } from '../types';

type Page = 'overview' | 'xcpc' | 'export' | 'data' | 'settings' | 'about' | Platform;

export default function Sidebar({ page, onChange, collapsed, onToggle }: { page: Page; onChange: (page: Page) => void; collapsed: boolean; onToggle: () => void }) {
  return (
    <aside className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-head">
        <div className="brand" onClick={() => onChange('overview')} title={collapsed ? 'OJ Insight' : undefined}>
          <div className="brand-mark"><span /><span /><span /><span /></div>
          <div className="brand-copy"><strong>OJ Insight</strong><small>Competitive Programming Analytics</small></div>
        </div>
        <button className="sidebar-toggle" onClick={onToggle} title={collapsed ? '展开侧栏' : '收起侧栏'} aria-label={collapsed ? '展开侧栏' : '收起侧栏'}>{collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}</button>
      </div>
      <nav>
        <button title={collapsed ? '总览' : undefined} className={page === 'overview' ? 'active' : ''} onClick={() => onChange('overview')}><LayoutDashboard size={17} /><span className="nav-label">总览</span></button>
        <div className="nav-title">PLATFORMS</div>
        {PLATFORM_ORDER.map((p) => (
          <button title={collapsed ? PLATFORM_META[p].name : undefined} key={p} className={page === p ? 'active' : ''} onClick={() => onChange(p)}>
            <span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} /><span className="nav-label">{PLATFORM_META[p].name}</span>
          </button>
        ))}
        <div className="nav-title">TRACKERS</div>
        <button title={collapsed ? 'XCPC Tracker' : undefined} className={page === 'xcpc' ? 'active' : ''} onClick={() => onChange('xcpc')}><TableProperties size={17} /><span className="nav-label">XCPC Tracker</span></button>
        <div className="nav-title">TOOLS</div>
        <button title={collapsed ? '导出' : undefined} className={page === 'export' ? 'active' : ''} onClick={() => onChange('export')}><Download size={17} /><span className="nav-label">导出</span></button>
        <button title={collapsed ? '数据源' : undefined} className={page === 'data' ? 'active' : ''} onClick={() => onChange('data')}><Database size={17} /><span className="nav-label">数据源</span></button>
        <button title={collapsed ? '设置' : undefined} className={page === 'settings' ? 'active' : ''} onClick={() => onChange('settings')}><Settings2 size={17} /><span className="nav-label">设置</span></button>
        <button title={collapsed ? '关于' : undefined} className={page === 'about' ? 'active' : ''} onClick={() => onChange('about')}><CircleHelp size={17} /><span className="nav-label">关于</span></button>
      </nav>
    </aside>
  );
}
