import type { Page } from '@playwright/test';
import type { AccountConfig, ContestReviewPreview, Snapshot, WatchedAcEvent, WatchedPerson } from '../../src/types';
import type { XcpcContest } from '../../src/lib/xcpc';

const EMPTY_SNAPSHOT: Snapshot = {
  stats: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  career: { solved: 0, accepted_submissions: 0, active_days: 0, longest_streak: 0, current_streak: 0, peak_day: null, peak_count: 0 },
  daily: [],
  platforms: [],
  difficulty: [],
  difficulty_daily: [],
  knowledge: [],
  ratings: [],
  recent: [],
  today_problems: [],
  metric_available: true,
  warnings: [],
};

interface TauriFixtures {
  onboarding?: boolean;
  syncErrors?: Record<string, string>;
  syncDelay?: number;
  snapshot?: Snapshot;
  afterSyncSnapshot?: Snapshot;
  accounts?: AccountConfig[];
  contests?: XcpcContest[];
  contestReview?: ContestReviewPreview;
  watchedPeople?: WatchedPerson[];
  watchedEvents?: WatchedAcEvent[];
  watchedAvatars?: Record<string, { mime: string; bytes: number[] }>;
}

export async function installTauriMock(page: Page, fixtures: TauriFixtures = {}) {
  await page.addInitScript(({ snapshot, afterSyncSnapshot, accounts, contests, contestReview, watchedPeople, watchedEvents, watchedAvatars, onboarding, syncErrors, syncDelay }) => {
    if (!onboarding && !localStorage.getItem('oj-insight.onboarding')) localStorage.setItem('oj-insight.onboarding', JSON.stringify({ status: 'completed', step: 0 }));
    let currentAccounts = JSON.parse(localStorage.getItem('__test_accounts__') || 'null') || accounts;
    const syncStatuses: Array<Record<string, unknown>> = [];
    const calls: Array<{ command: string; args: Record<string, unknown> }> = [];
    (window as unknown as { __TAURI_CALLS__: typeof calls }).__TAURI_CALLS__ = calls;
    let currentSnapshot = snapshot;
    let companionPort = 10046;
    const invoke = async (command: string, args: Record<string, unknown> = {}) => {
      calls.push({ command, args });
      switch (command) {
        case 'can_install_updates':
          return false;
        case 'get_companion_status':
          return { port: companionPort, listening: true, error: '' };
        case 'set_companion_port':
          companionPort = Number(args.port);
          return { port: companionPort, listening: true, error: '' };
        case 'plugin:event|listen':
          return 1;
        case 'plugin:event|unlisten':
          return undefined;
        case 'check_for_updates':
          return { currentVersion: '0.10.6', latestVersion: '0.10.6', releaseUrl: '', updateAvailable: false };
        case 'list_problem_sets':
          return JSON.parse(localStorage.getItem('__test_problem_sets__') || '[]');
        case 'list_training_matches':
          return [];
        case 'list_solve_records':
          return JSON.parse(localStorage.getItem('__test_solve_records__') || '[]');
        case 'save_solve_record': {
          const rows = JSON.parse(localStorage.getItem('__test_solve_records__') || '[]') as Array<{ id: string }>;
          localStorage.setItem('__test_solve_records__', JSON.stringify([...rows.filter((row) => row.id !== (args.record as { id: string }).id), args.record]));
          return undefined;
        }
        case 'delete_solve_record': {
          const rows = JSON.parse(localStorage.getItem('__test_solve_records__') || '[]') as Array<{ id: string }>;
          localStorage.setItem('__test_solve_records__', JSON.stringify(rows.filter((row) => row.id !== args.id)));
          return undefined;
        }
        case 'get_accounts':
          if (currentAccounts.length && (window as unknown as { __TEST_REFRESH_FAIL__?: boolean }).__TEST_REFRESH_FAIL__) throw new Error('账号刷新失败，请重试');
          return currentAccounts;
        case 'save_all_accounts':
          if ((window as unknown as { __TEST_SAVE_FAIL__?: boolean }).__TEST_SAVE_FAIL__) throw new Error('保存失败，请重试');
          currentAccounts = args.accounts;
          localStorage.setItem('__test_accounts__', JSON.stringify(currentAccounts));
          return undefined;
        case 'get_sync_statuses':
          return [...syncStatuses];
        case 'get_watched_people':
          return watchedPeople.map((person: WatchedPerson) => ({ ...person }));
        case 'get_watched_avatar':
          (window as unknown as { __WATCHED_AVATAR_REQUESTS__: string[] }).__WATCHED_AVATAR_REQUESTS__ = [
            ...((window as unknown as { __WATCHED_AVATAR_REQUESTS__?: string[] }).__WATCHED_AVATAR_REQUESTS__ || []),
            `${args.platform}:${args.account}`,
          ];
          return watchedAvatars[`${args.platform}:${args.account}`] || null;
        case 'get_watched_events':
          return watchedEvents;
        case 'get_pending_watched_notifications':
          return watchedEvents.filter((event: WatchedAcEvent) => !event.dismissed);
        case 'get_snapshot':
          return currentSnapshot;
        case 'sync_platform': {
          if (syncDelay) await new Promise((resolve) => setTimeout(resolve, syncDelay));
          const failure = syncErrors[String(args.platform)];
          const entries = (currentAccounts as AccountConfig[]).filter((entry) => entry.platform === args.platform);
          syncStatuses.splice(0, syncStatuses.length, ...syncStatuses.filter((entry) => entry.platform !== args.platform), ...entries.map((entry) => ({ platform: entry.platform, account: entry.account, status: failure ? 'error' : 'ok', message: failure || '同步成功', last_success: failure ? null : 1, last_attempt: 1, cached_records: failure ? 0 : 1 })));
          if (failure) throw new Error(failure);
          currentSnapshot = afterSyncSnapshot || currentSnapshot;
          return { platform: args.platform, inserted: 1, updated: 0, message: '同步成功', status: 'ok', partial: false };
        }
        case 'get_xcpc_contests':
          return contests;
        case 'inspect_contest_review':
          return contestReview;
        case 'get_day_detail': {
          const day = String(args.day || '');
          await fetch(`/__day?day=${encodeURIComponent(day)}`);
          return { day, items: [], aggregates: [] };
        }
        case 'open_external':
          await fetch(`/__open?url=${encodeURIComponent(String(args.url || ''))}`);
          return undefined;
        case 'prepare_tracker_session':
          return undefined;
        case 'open_study_assistant':
          (window as unknown as { __ASSISTANT_OPEN_COUNT__: number }).__ASSISTANT_OPEN_COUNT__ = ((window as unknown as { __ASSISTANT_OPEN_COUNT__?: number }).__ASSISTANT_OPEN_COUNT__ || 0) + 1;
          return undefined;
        case 'resize_study_assistant':
        case 'drag_study_assistant':
        case 'close_study_assistant':
          return undefined;
        case 'plugin:window|start_resize_dragging':
          (window as unknown as { __ASSISTANT_RESIZE_DIRECTION__: unknown }).__ASSISTANT_RESIZE_DIRECTION__ = args.value;
          return undefined;
        case 'sync_watched_people':
          (window as unknown as { __WATCHED_SYNC_COUNT__: number }).__WATCHED_SYNC_COUNT__ = ((window as unknown as { __WATCHED_SYNC_COUNT__?: number }).__WATCHED_SYNC_COUNT__ || 0) + 1;
          return { checked: watchedPeople.length, insertedEvents: 0, events: [], failures: [] };
        case 'sync_watched_person':
          return { checked: 0, insertedEvents: 0, events: [], failures: [] };
        case 'dismiss_watched_event':
        case 'save_watched_person':
        case 'delete_watched_person':
          return undefined;
        case 'save_watched_people':
          (window as unknown as { __WATCHED_SAVE__: unknown }).__WATCHED_SAVE__ = args;
          return undefined;
        case 'edit_watched_person':
          (window as unknown as { __WATCHED_EDIT__: unknown }).__WATCHED_EDIT__ = args;
          return undefined;
        default:
          throw new Error(`Unhandled Tauri command in layout test: ${command}`);
      }
    };
    (window as unknown as { __TAURI_INTERNALS__: { invoke: typeof invoke; transformCallback: (callback: unknown) => number; metadata: { currentWindow: { label: string } } } }).__TAURI_INTERNALS__ = { invoke, transformCallback: () => 1, metadata: { currentWindow: { label: 'study-assistant' } } };
  }, {
    snapshot: fixtures.snapshot || EMPTY_SNAPSHOT,
    afterSyncSnapshot: fixtures.afterSyncSnapshot,
    accounts: fixtures.accounts || [],
    onboarding: fixtures.onboarding || false,
    syncErrors: fixtures.syncErrors || {},
    syncDelay: fixtures.syncDelay || 0,
    contests: fixtures.contests || [],
    contestReview: fixtures.contestReview,
    watchedPeople: fixtures.watchedPeople || [],
    watchedEvents: fixtures.watchedEvents || [],
    watchedAvatars: fixtures.watchedAvatars || {},
  });
}
