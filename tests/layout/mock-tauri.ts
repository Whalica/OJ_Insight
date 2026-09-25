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
  metric_available: true,
  warnings: [],
};

interface TauriFixtures {
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
  await page.addInitScript(({ snapshot, afterSyncSnapshot, accounts, contests, contestReview, watchedPeople, watchedEvents, watchedAvatars }) => {
    let currentSnapshot = snapshot;
    const invoke = async (command: string, args: Record<string, unknown> = {}) => {
      switch (command) {
        case 'list_problem_sets':
        case 'list_training_matches':
          return [];
        case 'get_accounts':
          return accounts;
        case 'get_sync_statuses':
          return [];
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
        case 'sync_platform':
          currentSnapshot = afterSyncSnapshot || currentSnapshot;
          return { platform: args.platform, inserted: 1, updated: 0, message: '同步成功', status: 'ok', partial: false };
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
    (window as unknown as { __TAURI_INTERNALS__: { invoke: typeof invoke } }).__TAURI_INTERNALS__ = { invoke };
  }, {
    snapshot: fixtures.snapshot || EMPTY_SNAPSHOT,
    afterSyncSnapshot: fixtures.afterSyncSnapshot,
    accounts: fixtures.accounts || [],
    contests: fixtures.contests || [],
    contestReview: fixtures.contestReview,
    watchedPeople: fixtures.watchedPeople || [],
    watchedEvents: fixtures.watchedEvents || [],
    watchedAvatars: fixtures.watchedAvatars || {},
  });
}
