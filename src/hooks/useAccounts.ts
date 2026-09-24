import { useCallback, useState } from 'react';
import { emptyAccounts, type AccountMap } from '../lib/ui';
import { api } from '../services/api';
import type { SyncStatus } from '../types';

export function useAccounts() {
  const [accounts, setAccounts] = useState<AccountMap>(emptyAccounts);
  const [statuses, setStatuses] = useState<SyncStatus[]>([]);
  const [accountsLoaded, setAccountsLoaded] = useState(false);

  const loadAccounts = useCallback(async () => {
    const next = emptyAccounts();
    for (const entry of await api.getAccounts()) next[entry.platform].push(entry);
    setAccounts(next);
    setAccountsLoaded(true);
    return next;
  }, []);

  const loadStatuses = useCallback(async () => {
    setStatuses(await api.getStatuses());
  }, []);

  return { accounts, statuses, accountsLoaded, loadAccounts, loadStatuses };
}
