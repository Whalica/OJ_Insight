import ContestsPage from './ContestsPage';
import VpPage from './VpPage';
import TrainingReviewPage from './TrainingReviewPage';
import type { AccountMap } from '../lib/ui';

type Tab = 'library' | 'vp' | 'review';

/** 比赛 hub: preparing a contest, running it, and reviewing it are one task. */
export default function ContestsHubPage({ notify, accounts, onOpenSettings, onOpenVp, initialTab = 'library' }: {
  notify: (message: string) => void;
  accounts: AccountMap;
  onOpenSettings: () => void;
  onOpenVp: () => void;
  initialTab?: Tab;
}) {
  return initialTab === 'library' ? <ContestsPage notify={notify} onOpenVp={onOpenVp} />
    : initialTab === 'vp' ? <VpPage notify={notify} />
      : <TrainingReviewPage accounts={accounts} notify={notify} onOpenSettings={onOpenSettings} />;
}
