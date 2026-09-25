export type Platform = 'codeforces' | 'atcoder' | 'luogu' | 'nowcoder' | 'qoj' | 'leetcode';
export type Metric = 'first_ac' | 'daily_unique' | 'accepted_submissions' | 'activity';

export interface AccountConfig {
  platform: Platform;
  account: string;
  secret: string;
}

export interface WatchedPerson {
  id: number;
  platform: Platform;
  account: string;
  nickname: string;
  relationship: string;
  secret: string;
  enabled: boolean;
  initialized: boolean;
  status: 'idle' | 'checking' | 'ok' | 'warning' | 'error';
  message: string;
  lastChecked: number | null;
  lastSuccess: number | null;
}

export interface WatchedBindingInput {
  platform: Platform;
  account: string;
  secret: string;
}

export interface WatchedAcEvent {
  id: number;
  personId: number;
  platform: Platform;
  account: string;
  nickname: string;
  relationship: string;
  submissionId: string;
  problemId: string;
  problemName: string;
  problemUrl: string;
  epochSecond: number;
  language: string;
  difficulty: string | null;
  createdAt: number;
  dismissed: boolean;
}

export interface WatchedSyncResult {
  checked: number;
  insertedEvents: number;
  events: WatchedAcEvent[];
  failures: string[];
}

export interface SyncStatus {
  platform: Platform;
  account: string;
  status: 'idle' | 'syncing' | 'ok' | 'warning' | 'error' | 'auth_required';
  message: string;
  last_attempt: number | null;
  last_success: number | null;
  cached_records: number;
}

export interface PlatformSummary {
  platform: Platform;
  account: string;
  solved: number | null;
  accepted_submissions: number;
  active_days: number;
  today_count: number;
  last_success: number | null;
  status: string;
  message: string;
  activity_only: boolean;
  cached_records: number;
  last_attempt: number | null;
}

export interface DailyPoint {
  day: string;
  count: number;
}

export interface DifficultyBucket {
  platform: Platform;
  label: string;
  count: number;
  order: number;
}

export interface SubmissionItem {
  platform: Platform;
  account: string;
  source: 'oj' | 'daily' | string;
  source_day: string | null;
  submission_id: string;
  problem_key: string;
  problem_id: string;
  problem_name: string;
  problem_url: string;
  epoch_second: number;
  language: string;
  difficulty: string | null;
  tags: string[];
}

export interface DifficultyDayPoint {
  platform: Platform;
  day: string;
  label: string;
  order: number;
}

export interface ContestReviewPreview {
  platform: Platform;
  contestId: string;
  contestName: string;
  contestUrl: string;
  account: string;
  startEpoch: number | null;
  durationSeconds: number | null;
  problemCount: number;
  submissionCount: number;
  codeAvailable: boolean;
  completeness: 'complete' | 'partial' | string;
  notes: string[];
}

export interface ContestReviewExportResult {
  path: string;
  contestName: string;
  problemCount: number;
  submissionCount: number;
  codeCount: number;
  completeness: 'complete' | 'partial' | string;
  notes: string[];
}

export interface SolvedGain {
  platform: Platform;
  amount: number;
  id: number;
  offsetX: number;
  offsetY: number;
}

export interface KnowledgeBucket {
  platform: Platform;
  axis: string;
  count: number;
  score: number;
}

export interface RatingHistoryPoint {
  contest_id: string;
  contest_name: string;
  epoch_second: number;
  old_rating: number;
  new_rating: number;
  rank: number | null;
}

export interface RatingSummary {
  last_updated: number | null;
  stale: boolean;
  platform: Platform;
  account: string;
  display_name?: string;
  current: number;
  maximum: number;
  last_change: number;
  contest_count: number;
  last_contest_epoch: number;
  history: RatingHistoryPoint[];
}

export interface SnapshotStats {
  solved: number;
  accepted_submissions: number;
  active_days: number;
  longest_streak: number;
  current_streak: number;
  peak_day: string | null;
  peak_count: number;
}

export interface Snapshot {
  stats: SnapshotStats;
  career: SnapshotStats;
  daily: DailyPoint[];
  platforms: PlatformSummary[];
  difficulty: DifficultyBucket[];
  nowcoder_daily_difficulty: DifficultyBucket[];
  difficulty_daily: DifficultyDayPoint[];
  knowledge: KnowledgeBucket[];
  ratings: RatingSummary[];
  recent: SubmissionItem[];
  today_problems?: SubmissionItem[];
  metric_available: boolean;
  warnings: string[];
}

export interface UpdateInfo {
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  updateAvailable: boolean;
  installable?: boolean;
  notes?: string;
  publishedAt?: string;
}

export interface DayDetail {
  day: string;
  items: SubmissionItem[];
  aggregates: Array<{ platform: Platform; metric: string; count: number; note: string }>;
}

export interface SyncResult {
  platform: Platform;
  inserted: number;
  updated: number;
  message: string;
  status: string;
  partial: boolean;
}

export interface DifficultyDetail {
  platform: Platform;
  label: string;
  count: number;
  items: SubmissionItem[];
  note: string | null;
}

export type TrainingMode = 'relaxed' | 'balanced' | 'pressure';
export type TrainingRole = 'Warmup' | 'Stable' | 'Core' | 'Weakness' | 'Observation' | 'Stretch';
export type TagVisibility = 'never' | 'before_solving' | 'after_ac';

export interface CanonicalProblem {
  canonicalId: string;
  platform: Platform;
  problemKey: string;
  problemId: string;
  name: string;
  url: string;
  difficulty: string | null;
  tags: string[];
  trainingSuitability: number | null;
  observationDependency: number | null;
  implementationLoad: number | null;
  knowledgeDependency: number | null;
  interactive: boolean;
  outputOnly: boolean;
}

export interface ProblemSetProblem {
  position: number;
  problem: CanonicalProblem;
  role: TrainingRole;
  note: string;
}

export interface ProblemSet {
  id: number;
  title: string;
  description: string;
  setType: 'static';
  tagVisibility: TagVisibility;
  sourceSetId: number | null;
  sourceUrl: string | null;
  createdAt: number;
  updatedAt: number;
  problems: ProblemSetProblem[];
}

export type ProblemSetInput = Omit<ProblemSet, 'id' | 'createdAt' | 'updatedAt'> & { id: number | null };

export interface TrainingMatchProblem extends ProblemSetProblem {
  solved: boolean;
  solvedAt: number | null;
}

export interface TrainingMatch {
  id: number;
  problemSetId: number | null;
  title: string;
  mode: TrainingMode;
  status: 'running' | 'finished';
  tagVisibility: TagVisibility;
  targetSolveRateMin: number;
  targetSolveRateMax: number;
  durationMinutes: number;
  startedAt: number;
  endedAt: number | null;
  createdAt: number;
  problems: TrainingMatchProblem[];
}

export interface CandidateSourceStatus {
  platform: Platform;
  available: boolean;
  problemCount: number;
  message: string;
}

export interface CandidatePool {
  generatedAt: number;
  mode: TrainingMode;
  requestedCount: number;
  excludedSolved: number;
  candidates: CanonicalProblem[];
  sources: CandidateSourceStatus[];
}
