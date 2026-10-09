import { useEffect, useLayoutEffect, useRef, useState, type FocusEvent as ReactFocusEvent, type KeyboardEvent as ReactKeyboardEvent, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { BookOpenCheck, Bookmark, ChevronDown, CircleHelp, Clock3, Dumbbell, Download, LayoutDashboard, Layers3, ListChecks, PanelLeftClose, PanelLeftOpen, Play, Settings2, Sparkles, TableProperties, Users, LibraryBig } from 'lucide-react';
import icpcIcon from '../assets/platforms/icpc.ico';
import ojiLogo from '../assets/branding/oji-logo.png';
import PlatformIcon from './PlatformIcon';
import type { Page } from '../lib/navigation';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform } from '../types';
import { useI18n } from '../lib/i18n';

type NavGroup = 'platforms' | 'trackers' | 'training';

/* Hover intent: open quickly, but keep the flyout alive long enough that a
   pointer drifting a few pixels away does not close it again. */
const OPEN_DELAY = 120;
const CLOSE_DELAY = 320;

export default function Sidebar({ page, onChange, collapsed, onToggle, onAssistant, assistantVisible, selectedPlatform, onPlatform }: {
  page: Page;
  onChange: (page: Page) => void;
  collapsed: boolean;
  onToggle: () => void;
  onAssistant: () => void;
  assistantVisible: boolean;
  selectedPlatform: Platform | null;
  onPlatform: (platform: Platform | null) => void;
}) {
  const { t } = useI18n();
  const [openGroup, setOpenGroup] = useState<NavGroup | null>(null);
  const [flyoutInstant, setFlyoutInstant] = useState(false);
  const [flyoutPosition, setFlyoutPosition] = useState({ left: 0, top: 0 });
  const hosts = useRef<Partial<Record<NavGroup, HTMLDivElement | null>>>({});
  const flyout = useRef<HTMLDivElement>(null);
  const openTimer = useRef(0);
  const closeTimer = useRef(0);
  const pointerInside = useRef(false);

  useEffect(() => () => { window.clearTimeout(openTimer.current); window.clearTimeout(closeTimer.current); }, []);

  useLayoutEffect(() => {
    if (!openGroup) return;
    const anchor = hosts.current[openGroup];
    const panel = flyout.current;
    if (!anchor || !panel) return;
    const reposition = () => {
      const rect = anchor.getBoundingClientRect();
      setFlyoutPosition({
        left: Math.max(8, Math.min(rect.right, window.innerWidth - panel.offsetWidth - 8)),
        top: Math.max(8, Math.min(rect.top, window.innerHeight - panel.offsetHeight - 8)),
      });
    };
    reposition();
    const observer = new ResizeObserver(reposition);
    observer.observe(anchor);
    observer.observe(panel);
    window.addEventListener('resize', reposition);
    window.addEventListener('scroll', reposition, true);
    return () => {
      observer.disconnect();
      window.removeEventListener('resize', reposition);
      window.removeEventListener('scroll', reposition, true);
    };
  }, [openGroup, collapsed]);

  const cancelTimers = () => {
    window.clearTimeout(openTimer.current);
    window.clearTimeout(closeTimer.current);
  };
  const scheduleOpen = (group: NavGroup) => {
    cancelTimers();
    if (openGroup === group) return;
    openTimer.current = window.setTimeout(() => { setFlyoutInstant(false); setOpenGroup(group); }, OPEN_DELAY);
  };
  const scheduleClose = () => {
    pointerInside.current = false;
    cancelTimers();
    closeTimer.current = window.setTimeout(() => setOpenGroup(null), CLOSE_DELAY);
  };
  /* A pointer leaving the trigger and entering the flyout crosses the shared
     edge, so both sides cancel the pending close before it fires. */
  const flyoutProps = {
    onMouseEnter: () => { pointerInside.current = true; cancelTimers(); },
    onMouseLeave: scheduleClose,
    onKeyDown: (event: ReactKeyboardEvent) => { if (event.key === 'Escape') setOpenGroup(null); },
    onBlur: (event: ReactFocusEvent) => {
      const next = event.relatedTarget as Node | null;
      if (!pointerInside.current && !event.currentTarget.contains(next) && !(openGroup && hosts.current[openGroup]?.contains(next))) setOpenGroup(null);
    },
  };
  // The sidebar scroll container clips overflow; render the flyout outside it.
  const renderFlyout = (children: ReactNode) => createPortal(
    <div className={`nav-flyout${flyoutInstant ? ' instant' : ''}`} ref={flyout} style={flyoutPosition} {...flyoutProps}>{children}</div>,
    document.body,
  );
  const hostProps = (group: NavGroup) => ({
    ref: (element: HTMLDivElement | null) => { hosts.current[group] = element; },
    /* Entering any other entry closes the open flyout at once instead of waiting
       out the grace period, so the panel can never cover the entry being clicked. */
    onMouseEnter: () => {
      pointerInside.current = true;
      window.clearTimeout(openTimer.current);
      window.clearTimeout(closeTimer.current);
      setOpenGroup((current) => (current && current !== group ? null : current));
      scheduleOpen(group);
    },
    onMouseLeave: scheduleClose,
    onKeyDown: (event: ReactKeyboardEvent) => { if (event.key === 'Escape') setOpenGroup(null); },
    onBlur: (event: ReactFocusEvent) => {
      const next = event.relatedTarget as Node | null;
      if (!pointerInside.current && !event.currentTarget.contains(next) && !flyout.current?.contains(next)) setOpenGroup(null);
    },
  });
  const trigger = (group: NavGroup, Icon: typeof Layers3, label: string, testId: string) => (
    <button
      className={`nav-flyout-trigger ${openGroup === group ? 'open' : ''}`}
      data-testid={testId}
      aria-expanded={openGroup === group}
      aria-haspopup="true"
      title={collapsed ? label : undefined}
      onClick={() => { cancelTimers(); setFlyoutInstant(true); setOpenGroup(group); }}
      onFocus={() => { cancelTimers(); setOpenGroup(group); }}
    >
      <Icon size={17} />
      <span className="nav-label">{label}</span>
      <ChevronDown className="nav-flyout-chevron" size={14} />
    </button>
  );

  return (
    <aside className={`sidebar ${collapsed ? 'collapsed' : ''}`}>
      <div className="sidebar-head">
        <div className="brand" onClick={() => onChange('overview')} title={collapsed ? 'OJ Insight' : undefined}>
          <div className="brand-mark"><img src={ojiLogo} alt="" /></div>
          <div className="brand-copy"><strong lang="en">OJ Insight</strong></div>
        </div>
        <button className="sidebar-toggle" onClick={onToggle} title={collapsed ? t('展开侧栏') : t('收起侧栏')} aria-label={collapsed ? t('展开侧栏') : t('收起侧栏')}>{collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}</button>
      </div>
      <nav>
        <div className="nav-flyout-host" {...hostProps('platforms')}>
          {trigger('platforms', LayoutDashboard, t('平台'), 'nav-platforms')}
          {openGroup === 'platforms' && renderFlyout(<>
              <button className={!selectedPlatform ? 'active' : ''} aria-current={!selectedPlatform} onClick={() => onPlatform(null)}><Layers3 size={18} /><span>{t('综合总览')}</span></button>
              {PLATFORM_ORDER.map((platform) => (
                <button key={platform} className={selectedPlatform === platform ? 'active' : ''} aria-current={selectedPlatform === platform} onClick={() => { onPlatform(platform); }}>
                  <PlatformIcon platform={platform} className="nav-flyout-icon" /><span>{PLATFORM_META[platform].name}</span>
                </button>
              ))}
          </>)}
        </div>

        <div className="nav-flyout-host" {...hostProps('trackers')}>
          {trigger('trackers', TableProperties, 'Trackers', 'nav-trackers')}
          {openGroup === 'trackers' && renderFlyout(<>
              <div className="nav-flyout-title">Trackers</div>
              <button className={page === 'xcpc' ? 'active' : ''} aria-current={page === 'xcpc'} onClick={() => { onChange('xcpc'); }}><span className="icpc-nav-logo" aria-hidden="true"><img src={icpcIcon} alt="" /></span><span>ICPC / CCPC</span></button>
              <button className={page === 'tracker-codeforces' ? 'active' : ''} aria-current={page === 'tracker-codeforces'} onClick={() => { onChange('tracker-codeforces'); }}><PlatformIcon platform="codeforces" className="nav-flyout-icon" /><span>Codeforces</span></button>
              <button className={page === 'tracker-atcoder' ? 'active' : ''} aria-current={page === 'tracker-atcoder'} onClick={() => { onChange('tracker-atcoder'); }}><PlatformIcon platform="atcoder" className="nav-flyout-icon" /><span>AtCoder</span></button>
          </>)}
        </div>

        <div className="nav-flyout-host" {...hostProps('training')}>
          {trigger('training', Dumbbell, t('训练中心'), 'nav-training')}
          {openGroup === 'training' && renderFlyout(<>
              <div className="nav-flyout-title">{t('训练中心')}</div>
              <button className={page === 'problem-sets' ? 'active' : ''} aria-current={page === 'problem-sets'} onClick={() => { onChange('problem-sets'); }}><ListChecks size={15} /><span>{t('题单')}</span></button>
              <button className={page === 'community' ? 'active' : ''} aria-current={page === 'community'} onClick={() => { onChange('community'); }}><LibraryBig size={15} /><span>{t('推荐题单')}</span></button>
              <button className={page === 'training' ? 'active' : ''} aria-current={page === 'training'} onClick={() => { onChange('training'); }}><Sparkles size={15} /><span>{t('个性化组题')}</span></button>
              <div className="nav-flyout-separator" role="separator" />
              <button className={page === 'contests' ? 'active' : ''} aria-current={page === 'contests'} onClick={() => { onChange('contests'); }}><TableProperties size={15} /><span>{t('模拟赛')}</span></button>
              <button className={page === 'vp' ? 'active' : ''} aria-current={page === 'vp'} onClick={() => { onChange('vp'); }}><Play size={15} /><span>{t('参赛区')}</span></button>
              <button className={page === 'contest-review' ? 'active' : ''} aria-current={page === 'contest-review'} onClick={() => { onChange('contest-review'); }}><BookOpenCheck size={15} /><span>{t('赛后分析')}</span></button>
              <div className="nav-flyout-separator" role="separator" />
              <button className={page === 'solve-journal' ? 'active' : ''} aria-current={page === 'solve-journal'} onClick={() => { onChange('solve-journal'); }}><BookOpenCheck size={15} /><span>{t('解题手记')}</span></button>
              <button className={page === 'favorites' ? 'active' : ''} aria-current={page === 'favorites'} onClick={() => { onChange('favorites'); }}><Bookmark size={15} /><span>收藏夹</span></button>
          </>)}
        </div>

        <div className="nav-title">TOOLS</div>
        <button title={collapsed ? t('关注') : undefined} className={page === 'relationships' ? 'active' : ''} onClick={() => onChange('relationships')}><Users size={17} /><span className="nav-label">{t('关注')}</span></button>
        <button title={collapsed ? t('导出') : undefined} className={page === 'export' ? 'active' : ''} onClick={() => onChange('export')}><Download size={17} /><span className="nav-label">{t('导出')}</span></button>
        <button title={collapsed ? t('设置') : undefined} className={page === 'settings' ? 'active' : ''} onClick={() => onChange('settings')}><Settings2 size={17} /><span className="nav-label">{t('设置')}</span></button>
        <button title={collapsed ? t('关于') : undefined} className={page === 'about' ? 'active' : ''} onClick={() => onChange('about')}><CircleHelp size={17} /><span className="nav-label">{t('关于')}</span></button>
        <button title={collapsed ? t('做题小助手') : undefined} className="assistant-tool" aria-pressed={assistantVisible} onClick={onAssistant}><Clock3 size={17} /><span className="nav-label">{t('做题小助手')}</span></button>
      </nav>
    </aside>
  );
}
