import { useState } from 'react';
import { LibraryBig, ListChecks } from 'lucide-react';
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
  const navigation = <div className="problem-set-tabs" aria-label="题单来源">
    <button aria-pressed={tab === 'local'} className={tab === 'local' ? 'active' : ''} onClick={() => setTab('local')}><ListChecks size={18} /><span>本地题单</span></button>
    <button aria-pressed={tab === 'community'} className={tab === 'community' ? 'active' : ''} onClick={() => setTab('community')}><LibraryBig size={18} /><span>推荐题单</span></button>
  </div>;
  return tab === 'local'
    ? <ProblemSetsPage notify={notify} onTrain={onTrain} onSolveProblem={onSolveProblem} navigation={navigation} />
    : <CommunityPage notify={notify} onOpenLocalSets={() => setTab('local')} navigation={navigation} />;
}
