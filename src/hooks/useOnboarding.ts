import { useEffect, useState } from 'react';
import { PLATFORM_ORDER } from '../lib/platforms';
import type { AccountMap } from '../lib/ui';

export const ONBOARDING_KEY = 'oj-insight.onboarding';
type OnboardingState = { status: 'active' | 'completed' | 'skipped' | 'existing'; step: number };

function readState(): OnboardingState | null {
  try {
    const value = JSON.parse(localStorage.getItem(ONBOARDING_KEY) || 'null');
    if (!value || !['active', 'completed', 'skipped', 'existing'].includes(value.status)) return null;
    return { status: value.status, step: Number.isInteger(value.step) ? Math.max(0, Math.min(3, value.step)) : 0 };
  } catch { return null; }
}

export function useOnboarding(accounts: AccountMap, accountsLoaded: boolean) {
  const [state, setState] = useState(readState);
  // Capture before App persists its initial page, so a fresh launch stays fresh.
  const [previouslyUsed] = useState(() => !!localStorage.getItem('oj-insight.last-page') || !!localStorage.getItem('oj-insight.preferences'));
  const [ready, setReady] = useState(false);
  const persist = (next: OnboardingState) => {
    localStorage.setItem(ONBOARDING_KEY, JSON.stringify(next));
    setState(next);
  };

  useEffect(() => {
    if (!accountsLoaded || ready) return;
    const hasAccounts = PLATFORM_ORDER.some((platform) => accounts[platform].some((entry) => entry.account.trim()));
    if (!state) persist({ status: previouslyUsed || hasAccounts ? 'existing' : 'active', step: 0 });
    else if (state.status === 'active' && state.step === 2 && !hasAccounts) persist({ status: 'active', step: 1 });
    setReady(true);
  }, [accountsLoaded, accounts, ready, state, previouslyUsed]);

  return {
    open: ready && state?.status === 'active',
    ready,
    step: state?.step || 0,
    setStep: (step: number) => persist({ status: 'active', step }),
    restart: () => persist({ status: 'active', step: 0 }),
    finish: (skipped = false) => persist({ status: skipped ? 'skipped' : 'completed', step: 0 }),
  };
}
