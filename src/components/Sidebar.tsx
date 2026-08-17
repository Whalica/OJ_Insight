import { BarChart3, CircleHelp, Database, Download, LayoutDashboard, Settings2 } from 'lucide-react';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform } from '../types';

type Page = 'overview' | 'export' | 'data' | 'settings' | 'about' | Platform;

export default function Sidebar({ page, onChange }: { page: Page; onChange: (page: Page) => void }) {
  return (
    <aside className="sidebar">
      <div className="brand" onClick={() => onChange('overview')}>
        <div className="brand-mark"><span /><span /><span /><span /></div>
        <div><strong>OJ Insight</strong><small>Competitive Programming Analytics</small></div>
      </div>
      <nav>
        <button className={page === 'overview' ? 'active' : ''} onClick={() => onChange('overview')}><LayoutDashboard size={17} />总览</button>
        <div className="nav-title">PLATFORMS</div>
        {PLATFORM_ORDER.map((p) => (
          <button key={p} className={page === p ? 'active' : ''} onClick={() => onChange(p)}>
            <span className="oj-dot" style={{ background: PLATFORM_META[p].accent }} />{PLATFORM_META[p].name}
          </button>
        ))}
        <div className="nav-title">TOOLS</div>
        <button className={page === 'export' ? 'active' : ''} onClick={() => onChange('export')}><Download size={17} />导出</button>
        <button className={page === 'data' ? 'active' : ''} onClick={() => onChange('data')}><Database size={17} />数据源</button>
        <button className={page === 'settings' ? 'active' : ''} onClick={() => onChange('settings')}><Settings2 size={17} />设置</button>
        <button className={page === 'about' ? 'active' : ''} onClick={() => onChange('about')}><CircleHelp size={17} />关于</button>
      </nav>
      <div className="sidebar-foot"><BarChart3 size={14} /> Local-first · SQLite</div>
    </aside>
  );
}
