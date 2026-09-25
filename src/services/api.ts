import { invoke } from '@tauri-apps/api/core';

import type {
  AccountConfig,
  ContestReviewExportResult,
  ContestReviewPreview,
  DayDetail,
  DifficultyDetail,
  Metric,
  Platform,
  Snapshot,
  SyncResult,
  SyncStatus,
  UpdateInfo,
  WatchedAcEvent,
  WatchedBindingInput,
  WatchedPerson,
  WatchedSyncResult,
  CanonicalProblem,
  CandidatePool,
  Contest,
  ContestInput,
  VpSubmission,
  ProblemSet,
  ProblemSetInput,
  TrainingMatch,
  TrainingMode,
} from '../types';
import type { XcpcContest } from '../lib/xcpc';

export interface StorageInfo {
  rootDir: string;
  dataDir: string;
  databasePath: string;
  exportDir: string;
  webviewDir: string;
  logDir: string;
}

export const api = {
  storageInfo: () => invoke<StorageInfo>('get_storage_info'),
  getAccounts: () => invoke<AccountConfig[]>('get_accounts'),
  saveAccount: (platform: Platform, account: string, secret: string) =>
    invoke<void>('save_account', { platform, account, secret }),
  saveAccounts: (platform: Platform, accounts: AccountConfig[]) =>
    invoke<void>('save_accounts', { platform, accounts }),
  saveAllAccounts: (accounts: AccountConfig[]) =>
    invoke<void>('save_all_accounts', { accounts }),
  getStatuses: () => invoke<SyncStatus[]>('get_sync_statuses'),
  getWatchedPeople: () => invoke<WatchedPerson[]>('get_watched_people'),
  getWatchedAvatar: (platform: Platform, account: string) => invoke<{ mime: string; bytes: number[] } | null>('get_watched_avatar', { platform, account }),
  getWatchedEvents: (retention: number) => invoke<WatchedAcEvent[]>('get_watched_events', { retention }),
  getPendingWatchedNotifications: () => invoke<WatchedAcEvent[]>('get_pending_watched_notifications'),
  saveWatchedPerson: (platform: Platform, account: string, nickname: string, relationship: string, secret: string) =>
    invoke<void>('save_watched_person', { platform, account, nickname, relationship, secret }),
  saveWatchedPeople: (nickname: string, relationship: string, bindings: WatchedBindingInput[]) =>
    invoke<void>('save_watched_people', { nickname, relationship, bindings }),
  editWatchedPerson: (personIds: number[], nickname: string, relationship: string, bindings: WatchedBindingInput[]) =>
    invoke<void>('edit_watched_person', { personIds, nickname, relationship, bindings }),
  deleteWatchedPerson: (personId: number) => invoke<void>('delete_watched_person', { personId }),
  syncWatchedPeople: () => invoke<WatchedSyncResult>('sync_watched_people'),
  syncWatchedPerson: (personId: number) => invoke<WatchedSyncResult>('sync_watched_person', { personId }),
  dismissWatchedEvent: (eventId: number) => invoke<void>('dismiss_watched_event', { eventId }),
  getXcpcContests: (forceRefresh = false, refreshRatings = false) =>
    invoke<XcpcContest[]>('get_xcpc_contests', { forceRefresh, refreshRatings }),
  inspectContestReview: (platform: Platform, account: string, contestInput: string, includePostContest: boolean) =>
    invoke<ContestReviewPreview>('inspect_contest_review', { platform, account, contestInput, includePostContest }),
  generateContestReview: (platform: Platform, account: string, contestInput: string, includePostContest: boolean, path: string) =>
    invoke<ContestReviewExportResult>('generate_contest_review', { platform, account, contestInput, includePostContest, path }),
  syncPlatform: (platform: Platform, full = false) =>
    invoke<SyncResult>('sync_platform', { platform, full }),
  syncAll: () => invoke<SyncResult[]>('sync_all'),
  clearPlatform: (platform: Platform) =>
    invoke<void>('clear_platform_records', { platform }),
  clearAll: () => invoke<void>('clear_all_records'),
  snapshot: (
    platform: Platform | null,
    startDay: string | null,
    endDay: string | null,
    metric: Metric,
    account: string | null = null,
    source: string | null = null,
    timeZone = 'Asia/Shanghai',
  ) =>
    invoke<Snapshot>('get_snapshot', {
      platform,
      startDay,
      endDay,
      metric,
      account,
      source,
      timeZone,
    }),
  dayDetail: (
    day: string,
    platform: Platform | null,
    account: string | null = null,
    source: string | null = null,
    timeZone = 'Asia/Shanghai',
  ) => invoke<DayDetail>('get_day_detail', { day, platform, account, source, timeZone }),
  difficultyDetail: (
    platform: Platform,
    label: string,
    account: string | null = null,
    source: string | null = null,
  ) => invoke<DifficultyDetail>('get_difficulty_detail', { platform, label, account, source }),
  checkForUpdates: () => invoke<UpdateInfo>('check_for_updates'),
  canInstallUpdates: () => invoke<boolean>('can_install_updates'),
  openExternal: (url: string) => invoke<void>('open_external', { url }),
  writeExportFile: (path: string, data: number[]) =>
    invoke<void>('write_export_file', { path, data }),
  listProblemSets: () => invoke<ProblemSet[]>('list_problem_sets'),
  saveProblemSet: (input: ProblemSetInput) => invoke<ProblemSet>('save_problem_set', { input }),
  deleteProblemSet: (id: number) => invoke<void>('delete_problem_set', { id }),
  exportProblemSet: (id: number) => invoke<string>('export_problem_set', { id }),
  importProblemSet: (data: string) => invoke<ProblemSet>('import_problem_set', { data }),
  filterTrainingCandidates: (candidates: CanonicalProblem[]) => invoke<CanonicalProblem[]>('filter_training_candidates', { candidates }),
  startTrainingMatch: (problemSetId: number, mode: TrainingMode, durationMinutes: number, targetMin: number, targetMax: number) => invoke<TrainingMatch>('start_training_match', { problemSetId, mode, durationMinutes, targetMin, targetMax }),
  listTrainingMatches: () => invoke<TrainingMatch[]>('list_training_matches'),
  refreshTrainingMatch: (id: number) => invoke<TrainingMatch>('refresh_training_match', { id }),
  finishTrainingMatch: (id: number) => invoke<TrainingMatch>('finish_training_match', { id }),
  deleteTrainingMatch: (id: number) => invoke<void>('delete_training_match', { id }),
  generateTrainingCandidates: (platforms: Platform[], mode: TrainingMode, candidateCount: number) => invoke<CandidatePool>('generate_training_candidates', { platforms, mode, candidateCount }),
  exportAiTrainingPack: (candidates: CanonicalProblem[], mode: TrainingMode, extraRequirements?: string) => invoke<number[]>('export_ai_training_pack', { candidates, mode, extraRequirements }),
  exportTrainingPack: (problemSetId: number, mode: TrainingMode) => invoke<string>('export_training_pack', { problemSetId, mode }),
  importMatchManifest: (data: string) => invoke<TrainingMatch>('import_match_manifest', { data }),
  listContests: () => invoke<Contest[]>('list_contests'),
  saveContest: (input: ContestInput) => invoke<Contest>('save_contest', { input }),
  deleteContest: (id: number) => invoke<void>('delete_contest', { id }),
  contestFromSet: (setId: number, mode: TrainingMode, durationMinutes: number) => invoke<Contest>('contest_from_set', { setId, mode, durationMinutes }),
  importGeneratedContest: (data: string) => invoke<Contest>('import_generated_contest', { data }),
  queueContest: (id: number, countdownSeconds: number) => invoke<TrainingMatch>('queue_contest', { id, countdownSeconds }),
  startVp: (id: number) => invoke<TrainingMatch>('start_vp', { id }),
  pauseVp: (id: number) => invoke<TrainingMatch>('pause_vp', { id }),
  resumeVp: (id: number) => invoke<TrainingMatch>('resume_vp', { id }),
  saveVpNote: (id: number, position: number | null, note: string) => invoke<TrainingMatch>('save_vp_note', { id, position, note }),
  listVpSubmissions: (id: number) => invoke<VpSubmission[]>('list_vp_submissions', { id }),
  refreshVpSubmissions: (id: number) => invoke<VpSubmission[]>('refresh_vp_submissions', { id }),
  bindVpCode: (id: number, position: number, path: string) => invoke<void>('bind_vp_code', { id, position, path }),
  listVpCodeFiles: (id: number) => invoke<[number, string][]>('list_vp_code_files', { id }),
  exportVpReviewPack: (id: number) => invoke<number[]>('export_vp_review_pack', { id }),
  lookupProblemMetadata: (problem: CanonicalProblem) => invoke<CanonicalProblem>('lookup_problem_metadata', { problem }),
};
