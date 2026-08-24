use serde::{Deserialize, Serialize};

pub const PLATFORMS: [&str; 6] = [
    "codeforces",
    "atcoder",
    "luogu",
    "nowcoder",
    "qoj",
    "leetcode",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountConfig {
    pub platform: String,
    pub account: String,
    pub secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub platform: String,
    pub account: String,
    pub source: String,
    pub source_day: Option<String>,
    pub submission_id: String,
    pub problem_key: String,
    pub problem_id: String,
    pub problem_name: String,
    pub problem_url: String,
    pub epoch_second: i64,
    pub language: String,
    pub difficulty: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AggregateDay {
    pub day: String,
    pub epoch_second: Option<i64>,
    pub metric: String,
    pub count: i64,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct DifficultyStat {
    pub label: String,
    pub count: i64,
    pub order: i64,
}

#[derive(Debug, Clone)]
pub struct RemoteData {
    pub platform: String,
    pub account: String,
    pub submissions: Vec<Submission>,
    pub aggregates: Vec<AggregateDay>,
    pub solved_count: Option<i64>,
    pub difficulty: Vec<DifficultyStat>,
    pub activity_only: bool,
    pub notes: Vec<String>,
    pub cursor_epoch: i64,
    pub replace_submissions: bool,
    pub replace_aggregates: bool,
}

#[derive(Debug, Clone)]
pub struct SyncError {
    pub status: String,
    pub message: String,
}

impl SyncError {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            message: message.into(),
        }
    }
    pub fn auth(message: impl Into<String>) -> Self {
        Self {
            status: "auth_required".into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for SyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for SyncError {}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub platform: String,
    pub inserted: i64,
    pub updated: i64,
    pub message: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub platform: String,
    pub account: String,
    pub status: String,
    pub message: String,
    pub last_attempt: Option<i64>,
    pub last_success: Option<i64>,
    pub cached_records: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailyPoint {
    pub day: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DifficultyDayPoint {
    pub platform: String,
    pub day: String,
    pub label: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DifficultyBucket {
    pub platform: String,
    pub label: String,
    pub count: i64,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformSummary {
    pub platform: String,
    pub account: String,
    pub solved: Option<i64>,
    pub accepted_submissions: i64,
    pub active_days: i64,
    pub today_count: i64,
    pub last_success: Option<i64>,
    pub status: String,
    pub message: String,
    pub activity_only: bool,
    pub cached_records: i64,
    pub last_attempt: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SnapshotStats {
    pub solved: i64,
    pub accepted_submissions: i64,
    pub active_days: i64,
    pub longest_streak: i64,
    pub current_streak: i64,
    pub peak_day: Option<String>,
    pub peak_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub stats: SnapshotStats,
    pub career: SnapshotStats,
    pub daily: Vec<DailyPoint>,
    pub platforms: Vec<PlatformSummary>,
    pub difficulty: Vec<DifficultyBucket>,
    pub difficulty_daily: Vec<DifficultyDayPoint>,
    pub recent: Vec<Submission>,
    pub metric_available: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregateDetail {
    pub platform: String,
    pub metric: String,
    pub count: i64,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DayDetail {
    pub day: String,
    pub items: Vec<Submission>,
    pub aggregates: Vec<AggregateDetail>,
}
