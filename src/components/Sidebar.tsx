import { useEffect, useState } from 'react';
import { BookOpenCheck, ChevronDown, CircleHelp, Database, Download, Dumbbell, Layers3, LayoutDashboard, ListChecks, PanelLeftClose, PanelLeftOpen, Settings2, TableProperties, Users } from 'lucide-react';
import type { Page } from '../lib/navigation';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform } from '../types';

type NavGroup = 'platforms' | 'trackers' | 'training';

export default function Sidebar({ page, onChange, collapsed, onToggle }: { page: Page; onChange: (page: Page) => void; collapsed: boolean; onToggle: () => void }) {
  const pageGroup: NavGroup | null = PLATFORM_ORDER.includes(page as Platform) ? 'platforms' : page === 'xcpc' || page.startsWith('tracker-') ? 'trackers' : page === 'training' || page === 'problem-sets' || page === 'contest-review' ? 'training' : null;
  const savedGroup = localStorage.getItem('oj-insight.sidebar-group');
  const [openGroup, setOpenGroup] = useState<NavGroup | null>(() => pageGroup || (savedGroup === 'trackers' || savedGroup === 'platforms' || savedGroup === 'training' ? savedGroup : null));

  useEffect(() => {
    if (!pageGroup) return;
    setOpenGroup(pageGroup);
    localStorage.setItem('oj-insight.sidebar-group', pageGroup);
  }, [pageGroup]);

  const toggleGroup = (group: NavGroup) => {
    if (collapsed) onToggle();
    const next = openGroup === group ? null : group;
    setOpenGroup(next);
    if (next) localStorage.setItem('oj-insight.sidebar-group', next); else localStorage.removeItem('oj-insight.sidebar-group');
  };

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

        <section className={`nav-group ${openGroup === 'platforms' && !collapsed ? 'open' : ''}`}>
          <button className="nav-group-trigger" aria-expanded={openGroup === 'platforms' && !collapsed} title={collapsed ? 'Platforms' : undefined} onClick={() => toggleGroup('platforms')}><Layers3 size={17} /><span className="nav-label">Platforms</span><ChevronDown className="nav-group-chevron" size={14} /></button>
          <div className="nav-group-items">{PLATFORM_ORDER.map((platform) => (
            <button title={collapsed ? PLATFORM_META[platform].name : undefined} key={platform} className={page === platform ? 'active' : ''} onClick={() => onChange(platform)}>
              <span className="oj-dot" style={{ background: PLATFORM_META[platform].accent }} /><span className="nav-label">{PLATFORM_META[platform].name}</span>
            </button>
          ))}</div>
        </section>

        <section className={`nav-group ${openGroup === 'trackers' && !collapsed ? 'open' : ''}`}>
          <button className="nav-group-trigger" aria-expanded={openGroup === 'trackers' && !collapsed} title={collapsed ? 'Trackers' : undefined} onClick={() => toggleGroup('trackers')}><TableProperties size={17} /><span className="nav-label">Trackers</span><ChevronDown className="nav-group-chevron" size={14} /></button>
          <div className="nav-group-items tracker-items">
            <button title={collapsed ? 'ICPC/CCPC' : undefined} className={page === 'xcpc' ? 'active' : ''} onClick={() => onChange('xcpc')}><span className="oj-dot" style={{ background: '#48d0c0' }} /><span className="nav-label">ICPC / CCPC</span></button>
            <button className={page === 'tracker-codeforces' ? 'active' : ''} onClick={() => onChange('tracker-codeforces')}><span className="oj-dot" style={{ background: '#5aa6e8' }} /><span className="nav-label">Codeforces</span></button>
            <button className={page === 'tracker-atcoder' ? 'active' : ''} onClick={() => onChange('tracker-atcoder')}><span className="oj-dot" style={{ background: '#9aa4ad' }} /><span className="nav-label">AtCoder</span></button>
          </div>
        </section>

        <section className={`nav-group ${openGroup === 'training' && !collapsed ? 'open' : ''}`}>
          <button className="nav-group-trigger" aria-expanded={openGroup === 'training' && !collapsed} title={collapsed ? '训练中心' : undefined} onClick={() => toggleGroup('training')}><Dumbbell size={17} /><span className="nav-label">训练中心</span><ChevronDown className="nav-group-chevron" size={14} /></button>
          <div className="nav-group-items tracker-items">
            <button title={collapsed ? '个性化训练' : undefined} className={page === 'training' ? 'active' : ''} onClick={() => onChange('training')}><span className="oj-dot training-dot" /><span className="nav-label">个性化训练</span></button>
            <button title={collapsed ? '我的题单' : undefined} className={page === 'problem-sets' ? 'active' : ''} onClick={() => onChange('problem-sets')}><ListChecks size={14} /><span className="nav-label">我的题单</span></button>
            <button title={collapsed ? '训练与比赛复盘' : undefined} className={page === 'contest-review' ? 'active' : ''} onClick={() => onChange('contest-review')}><BookOpenCheck size={14} /><span className="nav-label">训练与比赛复盘</span></button>
          </div>
        </section>

        <div className="nav-title">TOOLS</div>
        <button title={collapsed ? '关注' : undefined} className={page === 'relationships' ? 'active' : ''} onClick={() => onChange('relationships')}><Users size={17} /><span className="nav-label">关注</span></button>
        <button title={collapsed ? '导出' : undefined} className={page === 'export' ? 'active' : ''} onClick={() => onChange('export')}><Download size={17} /><span className="nav-label">导出</span></button>
        <button title={collapsed ? '数据源' : undefined} className={page === 'data' ? 'active' : ''} onClick={() => onChange('data')}><Database size={17} /><span className="nav-label">数据源</span></button>
        <button title={collapsed ? '设置' : undefined} className={page === 'settings' ? 'active' : ''} onClick={() => onChange('settings')}><Settings2 size={17} /><span className="nav-label">设置</span></button>
        <button title={collapsed ? '关于' : undefined} className={page === 'about' ? 'active' : ''} onClick={() => onChange('about')}><CircleHelp size={17} /><span className="nav-label">关于</span></button>
      </nav>
    </aside>
  );
}
