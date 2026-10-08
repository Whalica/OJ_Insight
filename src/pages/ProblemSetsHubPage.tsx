import { useState } from 'react';
import ProblemSetsPage from './ProblemSetsPage';
import CommunityPage from './CommunityPage';
import type { ProblemSetProblem } from '../types';

type Tab = 'local' | 'community';

/** 题单 hub: 本地题单 and 推荐题单 are the same entity from two sources. */
export default function ProblemSetsHubPage({ notify, onTrain, onSolveProblem, initialTab = 'local' }: {
  notify: (message: string) => void;
  onTrain: (setId: number) => void;
  onSolveProblem: (problem: ProblemSetProblem['problem']) => void;
  initialTab?: Tab;
}) {
  const [tab, setTab] = useState<Tab>(initialTab);
  return <>
    <header className="topbar"><div><small lang="en">TRAINING CENTER · PROBLEM SETS</small><h1>题单</h1><p>本地题单与社区推荐题单都会成为可反复使用的题目集合；社区内容预览后保存为本地副本。</p></div></header>
    <div className="review-tabs"><button className={tab === 'local' ? 'active' : ''} onClick={() => setTab('local')}>本地题单</button><button className={tab === 'community' ? 'active' : ''} onClick={() => setTab('community')}>推荐题单</button></div>
    {tab === 'local'
      ? <ProblemSetsPage notify={notify} onTrain={onTrain} onSolveProblem={onSolveProblem} embedded />
      : <CommunityPage notify={notify} onOpenLocalSets={() => setTab('local')} embedded />}
  </>;
}
