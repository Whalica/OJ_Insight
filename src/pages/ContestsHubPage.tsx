import { useState } from 'react';
import ContestsPage from './ContestsPage';
import VpPage from './VpPage';
import TrainingReviewPage from './TrainingReviewPage';
import type { AccountMap } from '../lib/ui';

type Tab = 'library' | 'vp' | 'review';

/** 比赛 hub: preparing a contest, running it, and reviewing it are one task. */
export default function ContestsHubPage({ notify, accounts, onOpenSettings, initialTab = 'library' }: {
  notify: (message: string) => void;
  accounts: AccountMap;
  onOpenSettings: () => void;
  initialTab?: Tab;
}) {
  const [tab, setTab] = useState<Tab>(initialTab);
  return <>
    <header className="topbar"><div><small lang="en">TRAINING CENTER · CONTESTS</small><h1>比赛</h1><p>配置与历史场次、参赛计时、赛后复盘都在这里。提交仍在原 OJ 完成。</p></div></header>
    <div className="review-tabs"><button className={tab === 'library' ? 'active' : ''} onClick={() => setTab('library')}>模拟赛</button><button className={tab === 'vp' ? 'active' : ''} onClick={() => setTab('vp')}>参赛区</button><button className={tab === 'review' ? 'active' : ''} onClick={() => setTab('review')}>赛后分析</button></div>
    {tab === 'library' ? <ContestsPage notify={notify} onOpenVp={() => setTab('vp')} embedded />
      : tab === 'vp' ? <VpPage notify={notify} embedded />
        : <TrainingReviewPage accounts={accounts} notify={notify} onOpenSettings={onOpenSettings} embedded />}
  </>;
}
