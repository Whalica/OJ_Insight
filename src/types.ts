export type Platform = 'codeforces' | 'atcoder' | 'luogu' | 'nowcoder' | 'qoj' | 'leetcode';
export type Metric = 'first_ac' | 'daily_unique' | 'accepted_submissions' | 'activity';

export interface AccountConfig {
  platform: Platform;
  account: string;
  secret: string;
}

export interface SyncStatus {
  platform: Platform;
  account: string;
  status: 'idle' | 'syncing' | 'ok' | 'error' | 'auth_required';
  message: string;
  last_attempt: number | null;
  last_success: number | null;
}

export interface PlatformSummary {
  platform: Platform;
  account: string;
  solved: number | null;
  accepted_submissions: number;
  active_days: number;
  last_success: number | null;
  status: string;
  message: string;
  activity_only: boolean;
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
  submission_id: string;
  problem_key: string;
  problem_id: string;
  problem_name: string;
  problem_url: string;
  epoch_second: number;
  language: string;
  difficulty: string | null;
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
  daily: DailyPoint[];
  platforms: PlatformSummary[];
  difficulty: DifficultyBucket[];
  recent: SubmissionItem[];
  metric_available: boolean;
  warnings: string[];
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
}
